use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service,
    types::{
        InvitePath, InvitePayload, InviteRequest, InviteValidityResponse,
        InvitesPayload, ServerPath,
    },
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{response::EmptyResponse, AppResult},
};

#[derive(Clone, Debug)]
pub(super) struct InvitesState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl InvitesState {
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

impl HasJwtSecret for InvitesState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn is_valid_invite(
    State(state): State<InvitesState>,
    Path(token): Path<String>,
) -> AppResult<Json<InviteValidityResponse>> {
    let is_valid_invite =
        service::is_valid_invite(&state.database, &token).await?;
    Ok(Json(InviteValidityResponse { is_valid_invite }))
}

// TODO: This and `create_invite`/`delete_invite` below only check that the
// caller is logged in, not that they belong to or manage this server — the
// service layer performs no such check either. `get_invites` in particular
// returns live invite tokens, which double as server join credentials, to
// any authenticated user for any server. Confirm whether that's intentional.
pub(super) async fn get_invites(
    State(state): State<InvitesState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<InvitesPayload>> {
    let invites =
        service::get_valid_invites(&state.database, path.server_id).await?;
    Ok(Json(InvitesPayload { invites }))
}

pub(super) async fn create_invite(
    State(state): State<InvitesState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<InviteRequest>,
) -> AppResult<Json<InvitePayload>> {
    let invite = service::create_invite(
        &state.database,
        path.server_id,
        user_id,
        payload,
    )
    .await?;
    Ok(Json(InvitePayload { invite }))
}

pub(super) async fn delete_invite(
    State(state): State<InvitesState>,
    Path(path): Path<InvitePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::delete_invite(&state.database, path.server_id, path.invite_id)
        .await?;
    Ok(Json(EmptyResponse {}))
}
