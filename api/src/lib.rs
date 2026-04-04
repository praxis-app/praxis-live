mod auth;
mod health;
mod user;
mod view;

use axum::{routing::get, Router};
use sea_orm::DatabaseConnection;

pub fn router(database: DatabaseConnection, jwt_secret: String) -> Router {
    let api = Router::new()
        .route("/health", get(health::health))
        .merge(auth::router(database, jwt_secret));

    view::attach(Router::new().nest("/api", api))
}
