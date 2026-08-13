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
        RoleMembersRequest, RoleRequest, ServerRolePayload, ServerRolesPayload,
        UpdatePermissionsRequest,
    },
};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    common::{response::EmptyResponse, ApiError, AppResult},
    invites::InviteAccessToken,
    servers::types::UsersPayload,
    servers::{self, types::ServerPath},
};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerRolePath {
    server_id: Uuid,
    server_role_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerRoleMemberPath {
    server_id: Uuid,
    server_role_id: Uuid,
    user_id: Uuid,
}

pub(super) async fn get_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
    InviteAccessToken(invite_token): InviteAccessToken,
) -> AppResult<Json<ServerRolePayload>> {
    servers::ensure_server_read_access(
        &state.database,
        path.server_id,
        user_id,
        invite_token.as_deref(),
    )
    .await?;
    let server_role = service::get_server_role(
        &state.database,
        path.server_id,
        path.server_role_id,
    )
    .await?;
    Ok(Json(ServerRolePayload { server_role }))
}

pub(super) async fn get_server_roles(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<ServerRolesPayload>> {
    let server_roles =
        service::get_server_roles(&state.database, path.server_id).await?;
    Ok(Json(ServerRolesPayload { server_roles }))
}

pub(super) async fn get_users_eligible_for_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<UsersPayload>> {
    let users = service::get_users_eligible_for_server_role(
        &state.database,
        path.server_id,
        path.server_role_id,
    )
    .await?;
    Ok(Json(UsersPayload { users }))
}

pub(super) async fn create_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ServerRolePayload>> {
    let server_role =
        service::create_server_role(&state.database, path.server_id, payload)
            .await?;
    Ok(Json(ServerRolePayload { server_role }))
}

pub(super) async fn update_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_server_role(
        &state.database,
        path.server_id,
        path.server_role_id,
        payload,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn update_server_role_permissions(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_server_role_permissions(
        &state.database,
        path.server_id,
        path.server_role_id,
        payload.permissions,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn add_server_role_members(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<RoleMembersRequest>,
) -> AppResult<Json<EmptyResponse>> {
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_server_role_members(
        &state.database,
        path.server_id,
        path.server_role_id,
        &user_ids,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn remove_server_role_member(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRoleMemberPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::remove_server_role_member(
        &state.database,
        path.server_id,
        path.server_role_id,
        path.user_id,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn delete_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::delete_server_role(
        &state.database,
        path.server_id,
        path.server_role_id,
    )
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
