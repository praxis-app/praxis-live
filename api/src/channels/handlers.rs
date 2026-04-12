use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use super::{service, types::ChannelRequest};
use crate::{
    common::request::{parse_uuid, AuthenticatedUser, HasJwtSecret},
    messages::types::AppResult,
};

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

impl HasJwtSecret for ChannelsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ChannelPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
}

pub(super) async fn create_channel(
    State(state): State<ChannelsState>,
    Path(server_id): Path<String>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
    let channel = service::create_channel(&state.database, server_id, payload).await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

pub(super) async fn update_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    service::update_channel(&state.database, server_id, channel_id, payload).await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn delete_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
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
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&server_id, "serverId")?;
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
