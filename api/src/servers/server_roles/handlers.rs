use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::sync::Arc;

use super::{
    extractors::{
        ServerRoleContext, ServerRoleManagerContext, ServerRoleMemberContext,
    },
    service,
    types::{
        RoleMembersRequest, RoleRequest, ServerRolePath, ServerRolePayload,
        ServerRolesPayload, UpdatePermissionsRequest,
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
    pub(super) database: DatabaseConnection,
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
    Path(path): Path<ServerRolePath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
    InviteAccessToken(invite_token): InviteAccessToken,
) -> AppResult<Json<ServerRolePayload>> {
    servers::can_read_server(
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
    // Any authenticated user may read complete role definitions because the
    // proposal flow needs the current role permissions and members in order to
    // propose changes. Role mutations remain independently permission-gated.
    let server_roles =
        service::get_server_roles(&state.database, path.server_id).await?;
    Ok(Json(ServerRolesPayload { server_roles }))
}

pub(super) async fn get_users_eligible_for_server_role(
    State(state): State<ServerRolesState>,
    Path(path): Path<ServerRolePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<UsersPayload>> {
    // Readable without `ServerRole: manage` because proposing a membership
    // change requires selecting users who do not yet hold the role. It still
    // takes read access to the server, and the candidates are that server's
    // members, so this discloses no more than the roster already does.
    // Direct membership mutations remain independently permission-gated.
    servers::can_read_server(
        &state.database,
        path.server_id,
        Some(user_id),
        None,
    )
    .await?;
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
    context: ServerRoleManagerContext,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<ServerRolePayload>> {
    let server_role = service::create_server_role(
        &state.database,
        context.server_id,
        payload,
    )
    .await?;
    Ok(Json(ServerRolePayload { server_role }))
}

pub(super) async fn update_server_role(
    State(state): State<ServerRolesState>,
    context: ServerRoleContext,
    Json(payload): Json<RoleRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_server_role(
        &state.database,
        context.server_id,
        context.server_role_id,
        payload,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn update_server_role_permissions(
    State(state): State<ServerRolesState>,
    context: ServerRoleContext,
    Json(payload): Json<UpdatePermissionsRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_server_role_permissions(
        &state.database,
        context.server_id,
        context.server_role_id,
        payload.permissions,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn add_server_role_members(
    State(state): State<ServerRolesState>,
    context: ServerRoleContext,
    Json(payload): Json<RoleMembersRequest>,
) -> AppResult<Json<EmptyResponse>> {
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_server_role_members(
        &state.database,
        context.server_id,
        context.server_role_id,
        &user_ids,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn remove_server_role_member(
    State(state): State<ServerRolesState>,
    context: ServerRoleMemberContext,
) -> AppResult<Json<EmptyResponse>> {
    service::remove_server_role_member(
        &state.database,
        context.server_id,
        context.server_role_id,
        context.member_user_id,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn delete_server_role(
    State(state): State<ServerRolesState>,
    context: ServerRoleContext,
) -> AppResult<Json<EmptyResponse>> {
    service::delete_server_role(
        &state.database,
        context.server_id,
        context.server_role_id,
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
