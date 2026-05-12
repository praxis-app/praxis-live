use axum::{routing::post, Router};
use sea_orm::DatabaseConnection;

use super::{
    handlers::{
        join_call, leave_call, livekit_webhook, start_call, CallsState,
    },
    service::LiveKitConfig,
};
use crate::pub_sub::PubSubService;

pub(crate) fn livekit_webhook_router(
    database: DatabaseConnection,
    jwt_secret: String,
    livekit: Option<LiveKitConfig>,
) -> Router {
    Router::new()
        .route("/livekit/webhook", post(livekit_webhook))
        .with_state(CallsState::new(database, jwt_secret, livekit, None))
}

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    livekit: Option<LiveKitConfig>,
    pub_sub_service: PubSubService,
) -> Router {
    Router::new()
        .route("/{channelId}/calls", post(start_call))
        .route("/{channelId}/calls/{callId}/join", post(join_call))
        .route("/{channelId}/calls/{callId}/leave", post(leave_call))
        .with_state(CallsState::new(
            database,
            jwt_secret,
            livekit,
            Some(pub_sub_service),
        ))
}
