use axum::{routing::post, Router};
use sea_orm::DatabaseConnection;

use super::handlers::{login, logout, signup, AuthState};

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
) -> Router {
    Router::new()
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .with_state(AuthState::new(database, jwt_secret))
}
