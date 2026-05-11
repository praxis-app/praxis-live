use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service::{self, LiveKitConfig},
    types::CallPath,
};
use crate::{
    auth::HasJwtSecret,
    channels::extractors::{ChannelWriteContext, HasDatabase},
    common::{ApiError, AppResult},
};

#[derive(Clone, Debug)]
pub(crate) struct CallsState {
    pub(crate) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub(crate) livekit: Option<LiveKitConfig>,
}

impl CallsState {
    pub(crate) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
        livekit: Option<LiveKitConfig>,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            livekit,
        }
    }
}

impl HasJwtSecret for CallsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

impl HasDatabase for CallsState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

pub(crate) async fn start_call(
    State(state): State<CallsState>,
    context: ChannelWriteContext,
) -> AppResult<Json<serde_json::Value>> {
    let livekit = livekit_config(&state)?;

    let response = service::start_channel_call(
        &state.database,
        livekit,
        context.server_id,
        context.channel_id,
        context.user_id,
    )
    .await?;

    Ok(Json(serde_json::json!(response)))
}

pub(crate) async fn join_call(
    State(state): State<CallsState>,
    Path(path): Path<CallPath>,
    context: ChannelWriteContext,
) -> AppResult<Json<serde_json::Value>> {
    let livekit = livekit_config(&state)?;

    let response = service::join_channel_call(
        &state.database,
        livekit,
        context.server_id,
        context.channel_id,
        path.call_id,
        context.user_id,
    )
    .await?;

    Ok(Json(serde_json::json!(response)))
}

pub(crate) async fn leave_call(
    State(state): State<CallsState>,
    Path(path): Path<CallPath>,
    context: ChannelWriteContext,
) -> AppResult<Json<serde_json::Value>> {
    let livekit = livekit_config(&state)?;

    let call = service::leave_channel_call(
        &state.database,
        livekit,
        context.server_id,
        context.channel_id,
        path.call_id,
        context.user_id,
    )
    .await?;

    Ok(Json(serde_json::json!({ "call": call })))
}

// TODO: Rename to handle_livekit_webhook
pub(crate) async fn livekit_webhook(
    State(state): State<CallsState>,
    headers: HeaderMap,
    body: String,
) -> AppResult<StatusCode> {
    let livekit = livekit_config(&state)?;
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Missing authorization.")
        })?;

    service::handle_livekit_webhook(
        &state.database,
        livekit,
        &body,
        authorization,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

fn livekit_config(state: &CallsState) -> AppResult<&LiveKitConfig> {
    state.livekit.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "LiveKit is not configured.",
        )
    })
}
