use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use super::{service, types::InviteRequest};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{request::parse_uuid, AppResult},
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

#[derive(Debug, Deserialize)]
pub(super) struct InvitePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "inviteId")]
    invite_id: String,
}

pub(super) async fn is_valid_invite(
    State(state): State<InvitesState>,
    Path(token): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let is_valid_invite =
        service::is_valid_invite(&state.database, &token).await?;
    Ok(Json(serde_json::json!({
        "isValidInvite": is_valid_invite
    })))
}

pub(super) async fn get_invites(
    State(state): State<InvitesState>,
    Path(server_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let invites =
        service::get_valid_invites(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "invites": invites })))
}

pub(super) async fn create_invite(
    State(state): State<InvitesState>,
    Path(server_id): Path<String>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<InviteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let invite =
        service::create_invite(&state.database, server_id, user_id, payload)
            .await?;
    Ok(Json(serde_json::json!({ "invite": invite })))
}

pub(super) async fn delete_invite(
    State(state): State<InvitesState>,
    Path(path): Path<InvitePath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let invite_id = parse_uuid(&path.invite_id, "inviteId")?;
    service::delete_invite(&state.database, server_id, invite_id).await?;
    Ok(Json(serde_json::json!({})))
}
