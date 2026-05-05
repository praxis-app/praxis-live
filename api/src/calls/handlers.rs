use axum::{extract::State, http::StatusCode, response::Json};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::service::{self, LiveKitConfig};
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

pub(crate) async fn join_call(
    State(state): State<CallsState>,
    context: ChannelWriteContext,
) -> AppResult<Json<serde_json::Value>> {
    let livekit = state.livekit.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "LiveKit is not configured.",
        )
    })?;

    let response = service::join_channel_call(
        &state.database,
        livekit,
        context.server_id,
        context.channel_id,
        context.user_id,
    )
    .await?;

    Ok(Json(serde_json::json!(response)))
}
