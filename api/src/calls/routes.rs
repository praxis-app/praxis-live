use axum::{routing::post, Router};
use sea_orm::DatabaseConnection;

use super::{
    handlers::{join_call, leave_call, start_call, CallsState},
    service::LiveKitConfig,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    livekit: Option<LiveKitConfig>,
) -> Router {
    Router::new()
        .route("/{channelId}/calls", post(start_call))
        .route("/{channelId}/calls/{callId}/join", post(join_call))
        .route("/{channelId}/calls/{callId}/leave", post(leave_call))
        .with_state(CallsState::new(database, jwt_secret, livekit))
}
