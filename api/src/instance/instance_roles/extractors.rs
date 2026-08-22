//! Resolves instance-role routes and enforces the `InstanceRole` manage
//! permission before handlers execute.
//!
//! Instance roles are the highest-privilege scope in the app, and every route
//! in this module — reads included — requires the same permission. Gating that
//! here rather than in each handler means a new route cannot silently ship
//! without the check.

use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::prelude::Uuid;
use serde::Deserialize;

use super::handlers::InstanceRolesState;
use crate::{
    auth::AuthenticatedUser,
    authz::{self, PermissionScope},
    common::ApiError,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceRolePath {
    instance_role_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceRoleMemberPath {
    instance_role_id: Uuid,
    user_id: Uuid,
}

/// Caller may manage instance roles. Carries no ids; used by the routes that
/// take no path parameters.
pub(super) struct InstanceRoleManagerContext;

/// Caller may manage instance roles, for the role named in the path.
pub(super) struct InstanceRoleContext {
    pub(super) instance_role_id: Uuid,
}

/// Caller may manage instance roles, for the role and member named in the
/// path. `member_user_id` is the target of the change, never the caller.
pub(super) struct InstanceRoleMemberContext {
    pub(super) instance_role_id: Uuid,
    pub(super) member_user_id: Uuid,
}

impl FromRequestParts<InstanceRolesState> for InstanceRoleManagerContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &InstanceRolesState,
    ) -> Result<Self, Self::Rejection> {
        can_manage_instance_roles(parts, state).await?;
        Ok(Self)
    }
}

impl FromRequestParts<InstanceRolesState> for InstanceRoleContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &InstanceRolesState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<InstanceRolePath>::from_request_parts(parts, state)
                .await
                .map_err(|_| invalid_route_path())?;
        can_manage_instance_roles(parts, state).await?;

        Ok(Self {
            instance_role_id: path.instance_role_id,
        })
    }
}

impl FromRequestParts<InstanceRolesState> for InstanceRoleMemberContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &InstanceRolesState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<InstanceRoleMemberPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| invalid_route_path())?;
        can_manage_instance_roles(parts, state).await?;

        Ok(Self {
            instance_role_id: path.instance_role_id,
            member_user_id: path.user_id,
        })
    }
}

async fn can_manage_instance_roles(
    parts: &mut Parts,
    state: &InstanceRolesState,
) -> Result<(), ApiError> {
    let AuthenticatedUser(user_id) =
        AuthenticatedUser::from_request_parts(parts, state).await?;

    authz::can(
        &state.database,
        user_id,
        "manage",
        "InstanceRole",
        PermissionScope::Instance,
    )
    .await
}

fn invalid_route_path() -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
}
