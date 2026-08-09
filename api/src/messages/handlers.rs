use axum::{
    extract::{Multipart, Path, State},
    http::{Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

use super::{
    extractors::{CallMessageImageUploadContext, MessageImageUploadContext},
    service,
    types::{
        CallMessageImagePath, CreateMessageRequest, ImagePayload,
        MessageImagePath, MessagePayload,
    },
};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    calls::extractors::CallWriteContext,
    channels::{self, extractors::ChannelWriteContext},
    common::{
        images::safe_image_response, request::multipart_file,
        storage::upload_root, AppResult,
    },
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
            upload_root: Arc::new(upload_root()),
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

pub(super) async fn create_message(
    State(chat_state): State<ChatState>,
    context: ChannelWriteContext,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<MessagePayload>> {
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

    Ok(Json(MessagePayload { message }))
}

pub(super) async fn create_call_message(
    State(chat_state): State<ChatState>,
    context: CallWriteContext,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<MessagePayload>> {
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

    Ok(Json(MessagePayload { message }))
}

pub(super) async fn upload_message_image(
    State(chat_state): State<ChatState>,
    context: MessageImageUploadContext,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<ImagePayload>)> {
    let file = multipart_file(multipart, "file").await?;

    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        &context.message,
        context.image_id,
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

    Ok((StatusCode::CREATED, Json(ImagePayload { image })))
}

pub(super) async fn upload_call_message_image(
    State(chat_state): State<ChatState>,
    context: CallMessageImageUploadContext,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<ImagePayload>)> {
    let file = multipart_file(multipart, "file").await?;
    let image = service::store_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        &context.message,
        context.image_id,
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

    Ok((StatusCode::CREATED, Json(ImagePayload { image })))
}

pub(super) async fn get_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<MessageImagePath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Response<axum::body::Body>> {
    let image = service::get_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        path.server_id,
        path.channel_id,
        path.message_id,
        path.image_id,
        user_id,
    )
    .await?;
    safe_image_response(image.bytes)
}

pub(super) async fn get_call_message_image(
    State(chat_state): State<ChatState>,
    Path(path): Path<CallMessageImagePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Response<axum::body::Body>> {
    let image = service::get_call_message_image(
        &chat_state.database,
        &chat_state.upload_root,
        path.server_id,
        path.channel_id,
        path.call_id,
        path.message_id,
        path.image_id,
        user_id,
    )
    .await?;
    safe_image_response(image.bytes)
}
