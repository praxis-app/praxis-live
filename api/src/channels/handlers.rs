use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use serde::Deserialize;
use std::sync::Arc;

use super::{service, types::ChannelRequest};
use crate::messages::types::{ApiError, AppResult};

#[derive(Clone, Debug)]
pub(super) struct ChannelsState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl ChannelsState {
    pub(super) fn new(database: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ChannelPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
}

pub(super) async fn create_channel(
    State(state): State<ChannelsState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    require_user_id(&state, &headers)?;
    let channel = service::create_channel(&state.database, server_id, payload).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

pub(super) async fn update_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    headers: HeaderMap,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    require_user_id(&state, &headers)?;
    service::update_channel(&state.database, server_id, channel_id, payload).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn delete_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    require_user_id(&state, &headers)?;
    service::delete_channel(&state.database, server_id, channel_id).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn get_channels(
    State(state): State<ChannelsState>,
    Path(server_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let channels = service::get_channels(&state.database, server_id).await?;
    Ok(Json(serde_json::json!({ "channels": channels })))
}

pub(super) async fn get_joined_channels(
    State(state): State<ChannelsState>,
    Path(server_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let user_id = require_user_id(&state, &headers)?;
    let channels = service::get_joined_channels(&state.database, server_id, user_id).await?;
    Ok(Json(serde_json::json!({ "channels": channels })))
}

pub(super) async fn get_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let channel = service::get_channel_with_server(&state.database, server_id, channel_id).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

fn require_user_id(state: &ChannelsState, headers: &HeaderMap) -> AppResult<Uuid> {
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
