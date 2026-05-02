use axum::{
    routing::{post, MethodRouter},
    Router,
};
use sea_orm::DatabaseConnection;

use super::service::LiveKitConfig;
use crate::{auth::HasJwtSecret, calls::service::join_call};
use std::sync::Arc;

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
