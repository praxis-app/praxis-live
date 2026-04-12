use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, Response, StatusCode},
    response::Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::{prelude::Uuid, DatabaseConnection};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

use super::{
    service,
    types::{ApiError, AppResult, CreateMessageRequest},
};

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

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
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
    headers: HeaderMap,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let user_id = require_user_id(&chat_state, &headers)?;
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
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let message_id = parse_uuid(&path.message_id, "messageId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;
    let user_id = require_user_id(&chat_state, &headers)?;

    let mut image_bytes = None;
    let mut content_type = None;

    while let Some(field) = multipart.next_field().await.map_err(internal_error)? {
        if field.name() == Some("file") {
            content_type = field.content_type().map(ToOwned::to_owned);
            image_bytes = Some(field.bytes().await.map_err(internal_error)?.to_vec());
            break;
        }
    }

    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        server_id,
        channel_id,
        message_id,
        image_id,
        user_id,
        content_type,
        image_bytes.unwrap_or_default(),
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
        server_id,
        channel_id,
        message_id,
        image_id,
    )
    .await?;
    let storage_key = image
        .storage_key
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Image not uploaded yet."))?;
    let path = service::resolve_upload_path(&chat_state.upload_root, &storage_key);
    let bytes = tokio::fs::read(path).await.map_err(internal_error)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            image
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .body(Body::from(bytes))
        .map_err(internal_error)
}

fn require_user_id(chat_state: &ChatState, headers: &HeaderMap) -> AppResult<Uuid> {
    let token = bearer_token(headers)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(chat_state.jwt_secret.as_bytes()),
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

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("chat route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
