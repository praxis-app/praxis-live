use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use entity::users;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use std::sync::Arc;

use super::get_user_by_id;
use crate::{
    common::request::{parse_uuid, AuthenticatedUser, HasJwtSecret},
    messages::types::{ApiError, AppResult},
    servers,
};

#[derive(Clone, Debug)]
pub(super) struct UsersState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl UsersState {
    pub(super) fn new(database: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

impl HasJwtSecret for UsersState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_current_user(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let user = get_user_by_id(&state.database, user_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;
    let servers = servers::service::get_servers_for_user(&state.database, user_id).await?;
    let current_server = servers::service::get_current_server(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id.to_string(),
            "name": user.name,
            "anonymous": false,
            "permissions": {
                "instance": ["read:Server", "create:Server", "update:Server", "delete:Server"],
                "servers": {}
            },
            "profilePicture": null,
            "currentServer": current_server,
            "serversCount": servers.len()
        }
    })))
}

pub(super) async fn get_current_user_servers(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let servers = servers::service::get_servers_for_user(&state.database, user_id).await?;
    Ok(Json(serde_json::json!({ "servers": servers })))
}

pub(super) async fn is_first_user(
    State(state): State<UsersState>,
) -> AppResult<Json<serde_json::Value>> {
    let count = users::Entity::find()
        .count(&state.database)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "isFirstUser": count == 0 })))
}

pub(super) async fn get_user_profile(
    State(state): State<UsersState>,
    Path(user_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = parse_uuid(&user_id, "userId")?;
    let user = users::Entity::find()
        .filter(users::Column::Id.eq(user_id))
        .one(&state.database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "User not found."))?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id.to_string(),
            "name": user.name,
            "profilePicture": null,
            "coverPhoto": null
        }
    })))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("users request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
