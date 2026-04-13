use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    add_server_members, create_server, delete_server, get_default_server, get_server_by_id,
    get_server_by_invite_token, get_server_by_slug, get_server_config, get_server_members,
    get_servers, get_users_eligible_for_server, is_anonymous_users_enabled, join_server,
    remove_server_members, update_server, update_server_config, ServersState,
};
use crate::channels;

pub(crate) fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let servers_router = Router::new()
        .route("/servers", get(get_servers))
        .route("/servers", post(create_server))
        .route("/servers/default", get(get_default_server))
        .route(
            "/servers/invite/{inviteToken}",
            get(get_server_by_invite_token),
        )
        .route("/servers/slug/{slug}", get(get_server_by_slug))
        .route("/servers/{serverId}", get(get_server_by_id))
        .route("/servers/{serverId}", put(update_server))
        .route("/servers/{serverId}", delete(delete_server))
        .route("/servers/{serverId}/join", post(join_server))
        .route("/servers/{serverId}/members", get(get_server_members))
        .route("/servers/{serverId}/members", post(add_server_members))
        .route("/servers/{serverId}/members", delete(remove_server_members))
        .route(
            "/servers/{serverId}/members/eligible",
            get(get_users_eligible_for_server),
        )
        .route("/servers/{serverId}/configs", get(get_server_config))
        .route("/servers/{serverId}/configs", put(update_server_config))
        .route(
            "/servers/{serverId}/configs/anon-enabled",
            get(is_anonymous_users_enabled),
        )
        .with_state(ServersState::new(database.clone(), jwt_secret.clone()));

    servers_router.nest(
        "/servers/{serverId}/channels",
        channels::router(database, jwt_secret),
    )
}
