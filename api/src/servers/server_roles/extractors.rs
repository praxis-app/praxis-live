//! Resolves server-role routes and enforces the `ServerRole` manage
//! permission before handlers execute.
//!
//! Only the mutating routes use these. The read routes stay deliberately open
//! to any authenticated user, because the proposal flow needs current role
//! definitions and eligible members in order to propose changes to them.

use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::prelude::Uuid;

use super::{
    handlers::ServerRolesState,
    service,
    types::{ServerRoleMemberPath, ServerRolePath},
};
use crate::{
    auth::AuthenticatedUser, common::ApiError, servers::types::ServerPath,
};

/// Caller may manage roles in the server named in the path.
pub(super) struct ServerRoleManagerContext {
    pub(super) server_id: Uuid,
}

/// Caller may manage roles in the server named in the path, for the role also
/// named there.
pub(super) struct ServerRoleContext {
    pub(super) server_id: Uuid,
    pub(super) server_role_id: Uuid,
}

/// Caller may manage roles in the server named in the path, for the role and
/// member also named there. `member_user_id` is the target of the change,
/// never the caller.
pub(super) struct ServerRoleMemberContext {
    pub(super) server_id: Uuid,
    pub(super) server_role_id: Uuid,
    pub(super) member_user_id: Uuid,
}

impl FromRequestParts<ServerRolesState> for ServerRoleManagerContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerRolesState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<ServerPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| invalid_route_path())?;
        ensure_server_role_manager(parts, state, path.server_id).await?;

        Ok(Self {
            server_id: path.server_id,
        })
    }
}

impl FromRequestParts<ServerRolesState> for ServerRoleContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerRolesState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<ServerRolePath>::from_request_parts(parts, state)
                .await
                .map_err(|_| invalid_route_path())?;
        ensure_server_role_manager(parts, state, path.server_id).await?;

        Ok(Self {
            server_id: path.server_id,
            server_role_id: path.server_role_id,
        })
    }
}

impl FromRequestParts<ServerRolesState> for ServerRoleMemberContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerRolesState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<ServerRoleMemberPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| invalid_route_path())?;
        ensure_server_role_manager(parts, state, path.server_id).await?;

        Ok(Self {
            server_id: path.server_id,
            server_role_id: path.server_role_id,
            member_user_id: path.user_id,
        })
    }
}

async fn ensure_server_role_manager(
    parts: &mut Parts,
    state: &ServerRolesState,
    server_id: Uuid,
) -> Result<(), ApiError> {
    let AuthenticatedUser(user_id) =
        AuthenticatedUser::from_request_parts(parts, state).await?;

    service::ensure_can_manage_server_roles(&state.database, user_id, server_id)
        .await
}

fn invalid_route_path() -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
}
