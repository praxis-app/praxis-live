use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use serde::Deserialize;
use std::sync::Arc;

use super::{
    service,
    types::{JoinServerRequest, ServerConfigRequest, ServerMembersRequest, ServerRequest},
};
use crate::messages::types::{ApiError, AppResult};

#[derive(Clone, Debug)]
pub(super) struct ServersState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl ServersState {
    pub(super) fn new(database: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
}

pub(super) async fn get_servers(
    State(state): State<ServersState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let servers = service::get_servers(&state.database).await?;
    Ok(Json(serde_json::json!({ "servers": servers })))
}

pub(super) async fn get_server_by_id(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let server = service::get_server_by_id(&state.database, server_id, false).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn get_server_by_slug(
    State(state): State<ServersState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = require_user_id(&state, &headers)?;
    let server = service::get_server_by_slug(&state.database, &slug, user_id).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn get_server_by_invite_token(
    State(state): State<ServersState>,
    Path(_invite_token): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let server = service::get_default_server(&state.database).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn get_default_server(
    State(state): State<ServersState>,
) -> AppResult<Json<serde_json::Value>> {
    let server = service::get_default_server(&state.database).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn create_server(
    State(state): State<ServersState>,
    headers: HeaderMap,
    Json(payload): Json<ServerRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = require_user_id(&state, &headers)?;
    let server = service::create_server(&state.database, payload, user_id).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn update_server(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ServerRequest>,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let server = service::update_server(&state.database, server_id, payload).await?;
    Ok(Json(serde_json::json!({ "server": server })))
}

pub(super) async fn delete_server(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    service::delete_server(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn get_server_members(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let users = service::get_server_members(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "users": users })))
}

pub(super) async fn get_users_eligible_for_server(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let users = service::get_users_eligible_for_server(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "users": users })))
}

pub(super) async fn add_server_members(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ServerMembersRequest>,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::add_server_members(&state.database, server_id, &user_ids).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn remove_server_members(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ServerMembersRequest>,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let user_ids = parse_user_ids(&payload.user_ids)?;
    service::remove_server_members(&state.database, server_id, &user_ids).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn join_server(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<JoinServerRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    service::join_server(&state.database, server_id, user_id, &payload.invite_token).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn get_server_config(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    let server_config = service::get_server_config(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "serverConfig": server_config })))
}

pub(super) async fn is_anonymous_users_enabled(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let anonymous_users_enabled =
        service::is_anonymous_users_enabled(&state.database, server_id).await?;
    Ok(Json(
        serde_json::json!({ "anonymousUsersEnabled": anonymous_users_enabled }),
    ))
}

pub(super) async fn update_server_config(
    State(state): State<ServersState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ServerConfigRequest>,
) -> AppResult<Json<serde_json::Value>> {
    require_user_id(&state, &headers)?;
    let server_id = parse_uuid(&server_id, "serverId")?;
    service::update_server_config(&state.database, server_id, payload).await?;
    Ok(Json(serde_json::json!({})))
}

fn require_user_id(state: &ServersState, headers: &HeaderMap) -> AppResult<Uuid> {
    let token = bearer_token(headers)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .and_then(|claims| claims.claims.sub.parse::<Uuid>().ok())
    .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let header_value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header_value.split_once(' ')?;

    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

fn parse_uuid(value: &str, field: &str) -> AppResult<Uuid> {
    value
        .parse()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, format!("{field} must be a UUID.")))
}

fn parse_user_ids(values: &[String]) -> AppResult<Vec<Uuid>> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<Uuid>()
                .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, "userIds must be UUIDs."))
        })
        .collect()
}
