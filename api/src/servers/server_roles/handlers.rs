use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::sync::Arc;

use super::{
    service,
    types::{RoleMembersRequest, RoleRequest, UpdatePermissionsRequest},
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{request::parse_uuid, ApiError, AppResult},
};
use axum::http::StatusCode;

#[derive(Clone, Debug)]
pub(super) struct ServerRolesState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl ServerRolesState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

impl HasJwtSecret for ServerRolesState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_server_role(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    let server_role =
        service::get_server_role(&state.database, server_id, role_id).await?;
    Ok(Json(serde_json::json!({ "serverRole": server_role })))
}

pub(super) async fn get_server_roles(
    State(state): State<ServerRolesState>,
    Path(server_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let server_roles =
        service::get_server_roles(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "serverRoles": server_roles })))
}

pub(super) async fn get_users_eligible_for_server_role(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    let users = service::get_users_eligible_for_server_role(
        &state.database,
        server_id,
        role_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "users": users })))
}

pub(super) async fn create_server_role(
    State(state): State<ServerRolesState>,
    Path(server_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let server_role =
        service::create_server_role(&state.database, server_id, payload)
            .await?;
    Ok(Json(serde_json::json!({ "serverRole": server_role })))
}

pub(super) async fn update_server_role(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    service::update_server_role(&state.database, server_id, role_id, payload)
        .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn update_server_role_permissions(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    service::update_server_role_permissions(
        &state.database,
        server_id,
        role_id,
        payload.permissions,
    )
    .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn add_server_role_members(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleMembersRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_server_role_members(
        &state.database,
        server_id,
        role_id,
        &user_ids,
    )
    .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn remove_server_role_member(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id, user_id)): Path<(String, String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    let member_id = parse_uuid(&user_id, "userId")?;
    service::remove_server_role_member(
        &state.database,
        server_id,
        role_id,
        member_id,
    )
    .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn delete_server_role(
    State(state): State<ServerRolesState>,
    Path((server_id, role_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let role_id = parse_uuid(&role_id, "serverRoleId")?;
    service::delete_server_role(&state.database, server_id, role_id).await?;
    Ok(Json(serde_json::json!({})))
}

fn parse_user_ids(values: &[String]) -> AppResult<Vec<Uuid>> {
    values
        .iter()
        .map(|value| {
            value.parse::<Uuid>().map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "userIds must be UUIDs.")
            })
        })
        .collect()
}
