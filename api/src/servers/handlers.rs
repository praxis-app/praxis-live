use axum::{
    extract::{Path, State},
    http::{Response, StatusCode},
    response::Json,
};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::{path::PathBuf, sync::Arc};

use super::{
    extractors::ServerEditContext,
    service,
    types::{
        AnonymousUsersEnabledResponse, JoinServerRequest, ServerConfigPayload,
        ServerConfigRequest, ServerImagePath, ServerMembersRequest, ServerPath,
        ServerPayload, ServerRequest, ServersPayload, UsersPayload,
    },
};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    common::{
        request::JsonOrMultipartFiles, response::EmptyResponse,
        storage::upload_root, ApiError, AppResult,
    },
    invites::InviteAccessToken,
};

#[derive(Clone, Debug)]
pub(super) struct ServersState {
    pub(super) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    upload_root: Arc<PathBuf>,
}

impl ServersState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            upload_root: Arc::new(upload_root()),
        }
    }
}

impl HasJwtSecret for ServersState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_servers(
    State(state): State<ServersState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<ServersPayload>> {
    let servers = service::get_servers(&state.database).await?;
    Ok(Json(ServersPayload { servers }))
}

pub(super) async fn get_server_by_id(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<ServerPayload>> {
    let server =
        service::get_server_by_id(&state.database, path.server_id, false)
            .await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn get_server_by_slug(
    State(state): State<ServersState>,
    Path(slug): Path<String>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<ServerPayload>> {
    let server =
        service::get_server_by_slug(&state.database, &slug, user_id).await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn get_server_by_invite_token(
    State(state): State<ServersState>,
    Path(invite_token): Path<String>,
) -> AppResult<Json<ServerPayload>> {
    let server =
        service::get_server_by_invite_token(&state.database, &invite_token)
            .await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn get_default_server(
    State(state): State<ServersState>,
) -> AppResult<Json<ServerPayload>> {
    let server = service::get_default_server(&state.database).await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn create_server(
    State(state): State<ServersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: JsonOrMultipartFiles<ServerRequest>,
) -> AppResult<Json<ServerPayload>> {
    let (payload, images) = multipart.into_payload_and_files();
    let image = images.into_iter().next();
    let server = service::create_server(
        &state.database,
        &state.upload_root,
        payload,
        user_id,
        image,
    )
    .await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn update_server(
    State(state): State<ServersState>,
    context: ServerEditContext,
    multipart: JsonOrMultipartFiles<ServerRequest>,
) -> AppResult<Json<ServerPayload>> {
    let (payload, images) = multipart.into_payload_and_files();
    let image = images.into_iter().next();
    let server = service::update_server(
        &state.database,
        &state.upload_root,
        context.path.server_id,
        payload,
        image,
    )
    .await?;
    Ok(Json(ServerPayload { server }))
}

pub(super) async fn get_server_image(
    State(state): State<ServersState>,
    Path(path): Path<ServerImagePath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
    InviteAccessToken(invite_token): InviteAccessToken,
) -> AppResult<Response<axum::body::Body>> {
    let image = service::get_server_image(
        &state.database,
        &state.upload_root,
        path.server_id,
        path.image_id,
        user_id,
        invite_token.as_deref(),
    )
    .await?;

    crate::common::images::safe_image_response(image.bytes)
}

pub(super) async fn delete_server(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<EmptyResponse>> {
    service::delete_server(&state.database, &state.upload_root, path.server_id)
        .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn get_server_members(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<UsersPayload>> {
    let users =
        service::get_server_members(&state.database, path.server_id).await?;
    Ok(Json(UsersPayload { users }))
}

pub(super) async fn get_users_eligible_for_server(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<UsersPayload>> {
    let users =
        service::get_users_eligible_for_server(&state.database, path.server_id)
            .await?;
    Ok(Json(UsersPayload { users }))
}

pub(super) async fn add_server_members(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ServerMembersRequest>,
) -> AppResult<Json<EmptyResponse>> {
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_server_members(&state.database, path.server_id, &user_ids)
        .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn remove_server_members(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ServerMembersRequest>,
) -> AppResult<Json<EmptyResponse>> {
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::remove_server_members(&state.database, path.server_id, &user_ids)
        .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn join_server(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<JoinServerRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::join_server(
        &state.database,
        path.server_id,
        user_id,
        &payload.invite_token,
    )
    .await?;
    Ok(Json(EmptyResponse {}))
}

pub(super) async fn get_server_config(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<ServerConfigPayload>> {
    let server_config =
        service::get_server_config(&state.database, path.server_id).await?;
    Ok(Json(ServerConfigPayload { server_config }))
}

pub(super) async fn is_anonymous_users_enabled(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
) -> AppResult<Json<AnonymousUsersEnabledResponse>> {
    let anonymous_users_enabled =
        service::is_anonymous_users_enabled(&state.database, path.server_id)
            .await?;
    Ok(Json(AnonymousUsersEnabledResponse {
        anonymous_users_enabled,
    }))
}

pub(super) async fn update_server_config(
    State(state): State<ServersState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ServerConfigRequest>,
) -> AppResult<Json<EmptyResponse>> {
    service::update_server_config(&state.database, path.server_id, payload)
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
