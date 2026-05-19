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
    extractors::{CallMessageImageUploadContext, MessageImageUploadContext},
    service,
    types::{CallMessageImagePath, CreateMessageRequest, MessageImagePath},
};
use crate::{
    auth::{AuthenticatedUserOptional, HasJwtSecret},
    calls::extractors::CallWriteContext,
    channels::{self, extractors::ChannelWriteContext},
    common::{request::multipart_file, ApiError, AppResult},
    pub_sub::PubSubService,
};

#[derive(Clone, Debug)]
pub(super) struct ChatState {
    pub(super) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub_sub_service: PubSubService,
    upload_root: Arc<PathBuf>,
}

impl ChatState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
        pub_sub_service: PubSubService,
    ) -> Self {
        Self {
            database,
            pub_sub_service,
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

impl channels::extractors::HasDatabase for ChatState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct FeedQuery {
    offset: Option<u64>,
    limit: Option<u64>,
}

pub(super) async fn get_channel_feed(
    State(chat_state): State<ChatState>,
    Path(path): Path<channels::types::ChannelPath>,
    Query(query): Query<FeedQuery>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let feed = service::get_combined_channel_feed(
        &chat_state.database,
        path.server_id,
        path.channel_id,
        offset,
        limit,
        user_id,
    )
    .await?;

    Ok(Json(serde_json::json!({ "feed": feed })))
}

pub(super) async fn get_call_feed(
    State(chat_state): State<ChatState>,
    Path(path): Path<crate::calls::types::CallPath>,
    Query(query): Query<FeedQuery>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let feed = service::get_combined_call_feed(
        &chat_state.database,
        path.server_id,
        path.channel_id,
        path.call_id,
        offset,
        limit,
        user_id,
    )
    .await?;

    Ok(Json(serde_json::json!({ "feed": feed })))
}

pub(super) async fn create_message(
    State(chat_state): State<ChatState>,
    context: ChannelWriteContext,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let message = service::create_message(
        &chat_state.database,
        context.channel_id,
        context.user_id,
        payload,
    )
    .await?;
    if let Err(error) = service::broadcast_message(
        &chat_state.database,
        &chat_state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        &message,
    )
    .await
    {
        tracing::warn!("failed to broadcast created message: {error}");
    }

    Ok(Json(serde_json::json!({ "message": message })))
}

pub(super) async fn create_call_message(
    State(chat_state): State<ChatState>,
    context: CallWriteContext,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let message = service::create_call_message(
        &chat_state.database,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        payload,
    )
    .await?;
    if let Err(error) = service::broadcast_message_to_call(
        &chat_state.database,
        &chat_state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        &message,
    )
    .await
    {
        tracing::warn!("failed to broadcast created call message: {error}");
    }

    Ok(Json(serde_json::json!({ "message": message })))
}

pub(super) async fn upload_message_image(
    State(chat_state): State<ChatState>,
    context: MessageImageUploadContext,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let file = multipart_file(multipart, "file").await?;

    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        &context.message,
        context.image_id,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;
    if let Err(error) = service::broadcast_image_upload(
        &chat_state.database,
        &chat_state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        &context.message.id.to_string(),
        &context.image_id.to_string(),
    )
    .await
    {
        tracing::warn!("failed to broadcast uploaded message image: {error}");
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "image": image })),
    ))
}

pub(super) async fn upload_call_message_image(
    State(chat_state): State<ChatState>,
    context: CallMessageImageUploadContext,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let file = multipart_file(multipart, "file").await?;
    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        &context.message,
        context.image_id,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;
    if let Err(error) = service::broadcast_call_image_upload(
        &chat_state.database,
        &chat_state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        &context.message.id.to_string(),
        &context.image_id.to_string(),
    )
    .await
    {
        tracing::warn!(
            "failed to broadcast uploaded call message image: {error}"
        );
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "image": image })),
    ))
}

pub(super) async fn get_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<MessageImagePath>,
) -> AppResult<Response<Body>> {
    let image = service::get_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        path.server_id,
        path.channel_id,
        path.message_id,
        path.image_id,
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

pub(super) async fn get_call_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<CallMessageImagePath>,
) -> AppResult<Response<Body>> {
    let image = service::get_call_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        path.server_id,
        path.channel_id,
        path.call_id,
        path.message_id,
        path.image_id,
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
