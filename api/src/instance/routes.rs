use axum::Router;
use sea_orm::DatabaseConnection;

use super::instance_roles;

pub(crate) fn router(
    database: DatabaseConnection,
    jwt_secret: String,
) -> Router {
    Router::new().nest(
        "/instance/roles",
        instance_roles::router(database, jwt_secret),
    )
}
