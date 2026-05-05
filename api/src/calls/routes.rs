use axum::{
    routing::{post, MethodRouter},
    Router,
};
use sea_orm::DatabaseConnection;

use super::{
    handlers::{join_call, CallsState},
    service::LiveKitConfig,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
    livekit: Option<LiveKitConfig>,
) -> Router {
    let join_route: MethodRouter<CallsState> = post(join_call);

    Router::new()
        .route("/{channelId}/calls", join_route.clone())
        .route("/{channelId}/calls/join", join_route)
        .with_state(CallsState::new(database, jwt_secret, livekit))
}
