use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{
    add_server_role_members, create_server_role, delete_server_role,
    get_server_role, get_server_roles, get_users_eligible_for_server_role,
    remove_server_role_member, update_server_role,
    update_server_role_permissions, ServerRolesState,
};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
) -> Router {
    Router::new()
        .route("/", get(get_server_roles))
        .route("/", post(create_server_role))
        .route("/{serverRoleId}", get(get_server_role))
        .route("/{serverRoleId}", put(update_server_role))
        .route("/{serverRoleId}", delete(delete_server_role))
        .route(
            "/{serverRoleId}/permissions",
            put(update_server_role_permissions),
        )
        .route("/{serverRoleId}/members", post(add_server_role_members))
        .route(
            "/{serverRoleId}/members/eligible",
            get(get_users_eligible_for_server_role),
        )
        .route(
            "/{serverRoleId}/members/{userId}",
            delete(remove_server_role_member),
        )
        .with_state(ServerRolesState::new(database, jwt_secret))
}
