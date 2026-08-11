use axum::{
    extract::{Path, State},
    http::Response,
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

use super::{
    service,
    types::{
        CallMessageImagePath, CreateMessageRequest, MessageImagePath,
        MessagePayload,
    },
};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    calls::extractors::CallWriteContext,
    channels::{self, extractors::ChannelWriteContext},
    common::{
        images::safe_image_response, request::JsonOrMultipartFiles,
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
    multipart: JsonOrMultipartFiles<CreateMessageRequest>,
) -> AppResult<Json<MessagePayload>> {
    let (payload, images) = multipart.into_payload_and_files();
    let message = service::create_message(
        &chat_state.database,
        &chat_state.upload_root,
        context.channel_id,
        context.user_id,
        payload,
        images,
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
    multipart: JsonOrMultipartFiles<CreateMessageRequest>,
) -> AppResult<Json<MessagePayload>> {
    let (payload, images) = multipart.into_payload_and_files();
    let message = service::create_call_message(
        &chat_state.database,
        &chat_state.upload_root,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        payload,
        images,
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
