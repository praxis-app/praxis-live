use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use chrono::{DateTime, FixedOffset};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

use super::{
    extractors::MessageImageUploadContext,
    service,
    types::{CreateMessageRequest, MessageImagePath},
};
use crate::{
    auth::{AuthenticatedUserOptional, HasJwtSecret},
    channels::{self, extractors::ChannelWriteContext},
    common::{
        request::{multipart_file, parse_uuid},
        ApiError, AppResult,
    },
    polls,
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
    Path(path): Path<channels::types::ChannelRoutePath>,
    Query(query): Query<FeedQuery>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let fetch_limit = offset.saturating_add(limit);
    let mut feed = service::get_feed(
        &chat_state.database,
        server_id,
        channel_id,
        0,
        fetch_limit,
    )
    .await?;
    let polls = polls::service::get_inline_polls(
        &chat_state.database,
        server_id,
        channel_id,
        0,
        fetch_limit,
        user_id,
    )
    .await?;

    let mut feed = feed
        .drain(..)
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    for poll in polls {
        let mut value = serde_json::to_value(poll).map_err(internal_error)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "type".to_owned(),
                serde_json::Value::String("poll".to_owned()),
            );
        }
        feed.push(value);
    }

    feed.sort_by(|left, right| {
        timestamp_millis(right)
            .cmp(&timestamp_millis(left))
            .then_with(|| id_string(right).cmp(&id_string(left)))
    });
    let feed = feed
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect::<Vec<_>>();

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
    if let Err(error) = broadcast_message(
        &chat_state,
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
    if let Err(error) = broadcast_image_upload(
        &chat_state,
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

fn timestamp_millis(value: &serde_json::Value) -> i64 {
    value
        .get("createdAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|timestamp| {
            DateTime::<FixedOffset>::parse_from_rfc3339(timestamp).ok()
        })
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_default()
}

fn id_string(value: &serde_json::Value) -> String {
    value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

async fn broadcast_message(
    chat_state: &ChatState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    message: &crate::messages::types::MessageResponse,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "message",
        "message": message,
    });

    broadcast_to_channel_members(
        chat_state, server_id, channel_id, sender_id, body,
    )
    .await
}

async fn broadcast_image_upload(
    chat_state: &ChatState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    message_id: &str,
    image_id: &str,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "image",
        "isPlaceholder": false,
        "messageId": message_id,
        "imageId": image_id,
    });

    broadcast_to_channel_members(
        chat_state, server_id, channel_id, sender_id, body,
    )
    .await
}

async fn broadcast_to_channel_members(
    chat_state: &ChatState,
    server_id: sea_orm::prelude::Uuid,
    channel_id: sea_orm::prelude::Uuid,
    sender_id: sea_orm::prelude::Uuid,
    body: serde_json::Value,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(&chat_state.database, channel_id)
            .await?;

    for member_id in members {
        if member_id == sender_id {
            continue;
        }

        let topic = format!("new-message-{server_id}-{channel_id}-{member_id}");
        chat_state
            .pub_sub_service
            .publish(&topic, body.clone())
            .await?;
    }

    Ok(())
}
