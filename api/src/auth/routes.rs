use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;

use super::handlers::{login, logout, me, signup, AuthState};

pub(crate) fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .with_state(AuthState::new(database, jwt_secret))
}
