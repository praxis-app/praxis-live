use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{calls, users};
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use serde::Serialize;
use std::env;

use super::types::{CallResponse, JoinCallResponse};
use crate::common::{ApiError, AppResult};

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

pub(crate) async fn join_channel_call(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<JoinCallResponse> {
    let user = get_user(database, user_id).await?;
    let call =
        get_or_create_channel_call(database, server_id, channel_id, user_id)
            .await?;
    let room_name = call.livekit_room.clone();
    let token = create_livekit_token(livekit, &room_name, &user)?;
    let call = CallResponse {
        id: call.id.to_string(),
        server_id: call.server_id.to_string(),
        channel_id: call.channel_id.to_string(),
        room_name: room_name.clone(),
        status: call.status,
    };

    Ok(JoinCallResponse {
        livekit_url: livekit.url.to_owned(),
        room_name,
        token,
        call,
    })
}

fn room_name(server_id: uuid::Uuid, channel_id: uuid::Uuid) -> String {
    format!("praxis-server-{server_id}-channel-{channel_id}")
}

pub(crate) async fn get_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
) -> AppResult<calls::Model> {
    calls::Entity::find_by_id(call_id)
        .filter(calls::Column::ServerId.eq(server_id))
        .filter(calls::Column::ChannelId.eq(channel_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Call not found."))
}

async fn get_or_create_channel_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<calls::Model> {
    if let Some(call) = calls::Entity::find()
        .filter(calls::Column::ChannelId.eq(channel_id))
        .filter(calls::Column::Status.is_in(["starting", "active"]))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        return Ok(call);
    }

    calls::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        server_id: Set(server_id),
        channel_id: Set(channel_id),
        livekit_room: Set(room_name(server_id, channel_id)),
        status: Set("starting".to_owned()),
        started_by: Set(user_id),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)
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
