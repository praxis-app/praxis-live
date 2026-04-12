use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

use super::{
    service,
    types::{ApiError, AppResult, CreateMessageRequest},
};
use crate::common::request::{multipart_file, parse_uuid, AuthenticatedUser, HasJwtSecret};

#[derive(Clone, Debug)]
pub(super) struct ChatState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
    upload_root: Arc<PathBuf>,
}

impl ChatState {
    pub(super) fn new(database: DatabaseConnection, jwt_secret: String) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            upload_root: Arc::new(service::upload_root()),
        }
    }
}

impl HasJwtSecret for ChatState {
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

#[derive(Debug, Deserialize)]
pub(super) struct MessageImagePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "imageId")]
    image_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct FeedQuery {
    offset: Option<u64>,
    limit: Option<u64>,
}

pub(super) async fn get_channel_feed(
    State(chat_state): State<ChatState>,
    Path(path): Path<ChannelPath>,
    Query(query): Query<FeedQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let limit = query.limit.unwrap_or(50).min(100);
    let feed = service::get_feed(
        &chat_state.database,
        server_id,
        channel_id,
        query.offset.unwrap_or(0),
        limit,
    )
    .await?;

    Ok(Json(serde_json::json!({ "feed": feed })))
}

pub(super) async fn create_message(
    State(chat_state): State<ChatState>,
    Path(path): Path<ChannelPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let message = service::create_message(
        &chat_state.database,
        server_id,
        channel_id,
        user_id,
        payload,
    )
    .await?;

    Ok(Json(serde_json::json!({ "message": message })))
}

pub(super) async fn upload_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<MessageImagePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let message_id = parse_uuid(&path.message_id, "messageId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;
    let file = multipart_file(multipart, "file").await?;

    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        server_id,
        channel_id,
        message_id,
        image_id,
        user_id,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "image": image })),
    ))
}

pub(super) async fn get_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<MessageImagePath>,
) -> AppResult<Response<Body>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let message_id = parse_uuid(&path.message_id, "messageId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;

    let image = service::get_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        server_id,
        channel_id,
        message_id,
        image_id,
    )
    .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            image
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .body(Body::from(image.bytes))
        .map_err(internal_error)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("chat route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
