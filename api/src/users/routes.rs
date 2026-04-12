use axum::{routing::get, Router};
use sea_orm::DatabaseConnection;

use super::handlers::{
    get_current_user, get_current_user_servers, get_user_profile, is_first_user, UsersState,
};

pub(crate) fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    Router::new()
        .route("/users/me", get(get_current_user))
        .route("/users/me/servers", get(get_current_user_servers))
        .route("/users/is-first", get(is_first_user))
        .route("/users/{userId}/profile", get(get_user_profile))
        .with_state(UsersState::new(database, jwt_secret))
}
