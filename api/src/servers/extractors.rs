use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::prelude::Uuid;

use super::{handlers::ServersState, service, types::ServerPath};
use crate::{
    auth::AuthenticatedUser, common::ApiError, invites::InviteAccessToken,
};

/// Caller may read the server named in the path: a member, an invite holder,
/// the default server's audience, or an instance-level `Server: manage`
/// holder. Requires a real token — an invite token widens *which* servers a
/// caller can see, it does not stand in for identity.
pub(super) struct ServerViewContext {
    pub(super) path: ServerPath,
}

impl FromRequestParts<ServersState> for ServerViewContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServersState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<ServerPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let InviteAccessToken(invite_token) =
            InviteAccessToken::from_request_parts(parts, state).await?;

        service::can_view_server(
            &state.database,
            path.server_id,
            Some(user_id),
            invite_token.as_deref(),
        )
        .await?;

        Ok(Self { path })
    }
}

pub(super) struct ServerEditContext {
    pub(super) path: ServerPath,
    pub(super) user_id: Uuid,
}

impl FromRequestParts<ServersState> for ServerEditContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServersState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<ServerPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;

        service::can_update_server(&state.database, user_id, path.server_id)
            .await?;

        Ok(Self { path, user_id })
    }
}
