use axum::{
    extract::{Path, State},
    http::StatusCode,
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

#[derive(Clone, Debug)]
pub(super) struct InstanceRolesState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl InstanceRolesState {
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

impl HasJwtSecret for InstanceRolesState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_instance_role(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    let instance_role =
        service::get_instance_role(&state.database, role_id).await?;
    Ok(Json(serde_json::json!({ "instanceRole": instance_role })))
}

pub(super) async fn get_instance_roles(
    State(state): State<InstanceRolesState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let instance_roles = service::get_instance_roles(&state.database).await?;
    Ok(Json(serde_json::json!({ "instanceRoles": instance_roles })))
}

pub(super) async fn get_users_eligible_for_instance_role(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    let users =
        service::get_users_eligible_for_instance_role(&state.database, role_id)
            .await?;
    Ok(Json(serde_json::json!({ "users": users })))
}

pub(super) async fn create_instance_role(
    State(state): State<InstanceRolesState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let instance_role =
        service::create_instance_role(&state.database, payload).await?;
    Ok(Json(serde_json::json!({ "instanceRole": instance_role })))
}

pub(super) async fn update_instance_role(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    service::update_instance_role(&state.database, role_id, payload).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn update_instance_role_permissions(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    service::update_instance_role_permissions(
        &state.database,
        role_id,
        payload.permissions,
    )
    .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn add_instance_role_members(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleMembersRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_instance_role_members(&state.database, role_id, &user_ids)
        .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn remove_instance_role_member(
    State(state): State<InstanceRolesState>,
    Path((role_id, user_id)): Path<(String, String)>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    let user_id = parse_uuid(&user_id, "userId")?;
    service::remove_instance_role_member(&state.database, role_id, user_id)
        .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn delete_instance_role(
    State(state): State<InstanceRolesState>,
    Path(role_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let role_id = parse_uuid(&role_id, "instanceRoleId")?;
    service::delete_instance_role(&state.database, role_id).await?;
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
