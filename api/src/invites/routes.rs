use axum::{
    routing::{delete, get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    create_invite, delete_invite, get_invites, is_valid_invite, InvitesState,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
) -> Router {
    Router::new()
        .route("/invites/validate/{token}", get(is_valid_invite))
        .route("/servers/{serverId}/invites", get(get_invites))
        .route("/servers/{serverId}/invites", post(create_invite))
        .route(
            "/servers/{serverId}/invites/{inviteId}",
            delete(delete_invite),
        )
        .with_state(InvitesState::new(database, jwt_secret))
}
