use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::service;
use crate::{
    common::request::{parse_uuid, AuthenticatedUser, HasJwtSecret},
    messages::types::AppResult,
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
    let user = service::get_current_user(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({ "user": user })))
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
    let is_first_user = service::is_first_user(&state.database).await?;
    Ok(Json(serde_json::json!({ "isFirstUser": is_first_user })))
}

pub(super) async fn get_user_profile(
    State(state): State<UsersState>,
    Path(user_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = parse_uuid(&user_id, "userId")?;
    let user = service::get_user_profile(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({ "user": user })))
}
