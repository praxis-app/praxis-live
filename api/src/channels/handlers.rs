use axum::{
    extract::{Path, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

use super::{service, types::ChannelRequest};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::AppResult,
};

#[derive(Clone, Debug)]
pub(super) struct ChannelsState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl ChannelsState {
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

impl HasJwtSecret for ChannelsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ChannelPath {
    #[serde(rename = "serverId")]
    server_id: sea_orm::prelude::Uuid,
    #[serde(rename = "channelId")]
    channel_id: sea_orm::prelude::Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerPath {
    server_id: sea_orm::prelude::Uuid,
}

pub(super) async fn create_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let channel =
        service::create_channel(&state.database, path.server_id, payload)
            .await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}

pub(super) async fn update_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
    Json(payload): Json<ChannelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    service::update_channel(
        &state.database,
        path.server_id,
        path.channel_id,
        payload,
    )
    .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn delete_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    service::delete_channel(&state.database, path.server_id, path.channel_id)
        .await?;
    Ok(Json(serde_json::json!({})))
}

pub(super) async fn get_channels(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
) -> AppResult<Json<serde_json::Value>> {
    let channels =
        service::get_channels(&state.database, path.server_id).await?;
    Ok(Json(serde_json::json!({ "channels": channels })))
}

pub(super) async fn get_joined_channels(
    State(state): State<ChannelsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let channels =
        service::get_joined_channels(&state.database, path.server_id, user_id)
            .await?;
    Ok(Json(serde_json::json!({ "channels": channels })))
}

pub(super) async fn get_channel(
    State(state): State<ChannelsState>,
    Path(path): Path<ChannelPath>,
) -> AppResult<Json<serde_json::Value>> {
    let channel = service::get_channel_with_server(
        &state.database,
        path.server_id,
        path.channel_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "channel": channel })))
}
