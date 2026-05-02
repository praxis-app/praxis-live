use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::{Duration, Utc};
use entity::users;
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Serialize;
use std::env;

use super::{
    routes::CallsState,
    types::{CallResponse, ChannelCallPath, JoinCallResponse},
};
use crate::{
    auth::AuthenticatedUser,
    channels,
    common::{ApiError, AppResult},
};

const TOKEN_TTL_MINUTES: i64 = 30;

#[derive(Clone, Debug)]
pub(crate) struct LiveKitConfig {
    pub(crate) url: String,
    api_key: String,
    api_secret: String,
}

impl LiveKitConfig {
    pub(crate) fn from_env() -> Option<Self> {
        let url = livekit_url_from_env()?;
        let api_key = env::var("LIVEKIT_API_KEY").ok()?;
        let api_secret = env::var("LIVEKIT_API_SECRET").ok()?;

        if url.trim().is_empty()
            || api_key.trim().is_empty()
            || api_secret.trim().is_empty()
        {
            return None;
        }

        Some(Self {
            url,
            api_key,
            api_secret,
        })
    }
}

fn livekit_url_from_env() -> Option<String> {
    let host = env::var("LIVEKIT_HOST").ok()?;
    let port = env::var("LIVEKIT_PORT").ok()?;

    if host.trim().is_empty() || port.trim().is_empty() {
        return None;
    }

    Some(format!("ws://{host}:{port}"))
}

pub(crate) async fn join_call(
    State(state): State<CallsState>,
    Path(path): Path<ChannelCallPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let livekit = state.livekit.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "LiveKit is not configured.",
        )
    })?;

    channels::get_channel(&state.database, path.server_id, path.channel_id)
        .await?;
    channels::ensure_channel_membership(
        &state.database,
        path.channel_id,
        user_id,
    )
    .await?;

    let user = get_user(&state.database, user_id).await?;
    let room_name = room_name(path.server_id, path.channel_id);
    let token = create_livekit_token(livekit, &room_name, &user)?;
    let call = CallResponse {
        id: room_name.clone(),
        server_id: path.server_id.to_string(),
        channel_id: path.channel_id.to_string(),
        room_name: room_name.clone(),
        status: "starting".to_owned(),
    };

    Ok(Json(serde_json::json!(JoinCallResponse {
        livekit_url: livekit.url.clone(),
        room_name,
        token,
        call,
    })))
}

fn room_name(server_id: uuid::Uuid, channel_id: uuid::Uuid) -> String {
    format!("praxis-server-{server_id}-channel-{channel_id}")
}

async fn get_user(
    database: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> AppResult<users::Model> {
    users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })
}

fn create_livekit_token(
    livekit: &LiveKitConfig,
    room_name: &str,
    user: &users::Model,
) -> AppResult<String> {
    let now = Utc::now();
    let claims = LiveKitClaims {
        iss: livekit.api_key.clone(),
        sub: user.id.to_string(),
        name: user
            .display_name
            .clone()
            .unwrap_or_else(|| user.name.clone()),
        nbf: now.timestamp() as usize,
        exp: (now + Duration::minutes(TOKEN_TTL_MINUTES)).timestamp() as usize,
        video: VideoGrant {
            room: room_name.to_owned(),
            room_join: true,
            can_subscribe: true,
            can_publish: true,
            can_publish_data: false,
        },
        metadata: serde_json::json!({
            "userId": user.id,
            "name": user.name,
            "displayName": user.display_name,
        })
        .to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(livekit.api_secret.as_bytes()),
    )
    .map_err(internal_error)
}

#[derive(Debug, Serialize)]
struct LiveKitClaims {
    iss: String,
    sub: String,
    name: String,
    nbf: usize,
    exp: usize,
    video: VideoGrant,
    metadata: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoGrant {
    room: String,
    room_join: bool,
    can_subscribe: bool,
    can_publish: bool,
    can_publish_data: bool,
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("call request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
