use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use serde::Deserialize;
use std::sync::Arc;

use super::{
    service,
    types::{
        InstanceRolePayload, InstanceRolesPayload, RoleMembersRequest,
        RoleRequest, UpdatePermissionsRequest,
    },
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{response::EmptyResponse, ApiError, AppResult},
    servers::types::UsersPayload,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InstanceRolePath {
    instance_role_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct InstanceRoleMemberPath {
    instance_role_id: Uuid,
    user_id: Uuid,
}

pub(super) async fn get_instance_role(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<InstanceRolePayload>> {
    let instance_role =
        service::get_instance_role(&state.database, path.instance_role_id)
            .await?;
    Ok(Json(InstanceRolePayload { instance_role }))
}

pub(super) async fn get_instance_roles(
    State(state): State<InstanceRolesState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<InstanceRolesPayload>> {
    let instance_roles = service::get_instance_roles(&state.database).await?;
    Ok(Json(InstanceRolesPayload { instance_roles }))
}

pub(super) async fn get_users_eligible_for_instance_role(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<UsersPayload>> {
    let users = service::get_users_eligible_for_instance_role(
        &state.database,
        path.instance_role_id,
    )
    .await?;
    Ok(Json(UsersPayload { users }))
}

pub(super) async fn create_instance_role(
    State(state): State<InstanceRolesState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<InstanceRolePayload>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    let instance_role =
        service::create_instance_role(&state.database, payload).await?;
    Ok(Json(InstanceRolePayload { instance_role }))
}

pub(super) async fn update_instance_role(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    service::update_instance_role(
        &state.database,
        path.instance_role_id,
        payload,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn update_instance_role_permissions(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    service::update_instance_role_permissions(
        &state.database,
        path.instance_role_id,
        payload.permissions,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn add_instance_role_members(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<RoleMembersRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_instance_role_members(
        &state.database,
        path.instance_role_id,
        &user_ids,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn remove_instance_role_member(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRoleMemberPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    service::remove_instance_role_member(
        &state.database,
        path.instance_role_id,
        path.user_id,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn delete_instance_role(
    State(state): State<InstanceRolesState>,
    Path(path): Path<InstanceRolePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::ensure_can_manage_instance_roles(&state.database, user_id).await?;
    service::delete_instance_role(&state.database, path.instance_role_id)
        .await?;
    Ok(Json(EmptyResponse {}))
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
