use axum::http::StatusCode;
use entity::{channel_members, channels, message_images, messages, server_members, servers, users};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::path::{Path, PathBuf};
use uuid::Uuid as NativeUuid;

use super::types::{
    serialize_timestamp, ApiError, AppResult, ChannelRequest, ChannelResponse, ChannelServer,
    CreateMessageRequest, FeedMessageResponse, ImageResponse, MessageResponse, MessageUser,
    StoredImage,
};

pub(crate) const DEFAULT_SERVER_ID: &str = "11111111-1111-1111-1111-111111111111";
const MAX_IMAGE_COUNT: usize = 8;

pub(crate) async fn provision_user_memberships(
    database: &DatabaseConnection,
    user_id: i64,
) -> Result<(), sea_orm::DbErr> {
    let default_server_id = default_server_id();
    let server_membership = server_members::Entity::find()
        .filter(server_members::Column::UserId.eq(user_id))
        .filter(server_members::Column::ServerId.eq(default_server_id))
        .one(database)
        .await?;

    if server_membership.is_none() {
        server_members::ActiveModel {
            server_id: Set(default_server_id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(database)
        .await?;
    }

    let existing = channel_members::Entity::find()
        .filter(channel_members::Column::UserId.eq(user_id))
        .count(database)
        .await?;

    if existing > 0 {
        return Ok(());
    }

    let channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(default_server_id))
        .all(database)
        .await?;

    for channel in channels {
        channel_members::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(database)
        .await?;
    }

    Ok(())
}

pub(crate) async fn list_channels(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<ChannelResponse>> {
    ensure_server(database, server_id).await?;

    let server = load_server(database, server_id).await?;
    let channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .order_by_asc(channels::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(channels
        .into_iter()
        .map(|channel| shape_channel(channel, &server))
        .collect())
}

pub(crate) async fn list_joined_channels(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: i64,
) -> AppResult<Vec<ChannelResponse>> {
    ensure_server(database, server_id).await?;

    let server = load_server(database, server_id).await?;
    let memberships = channel_members::Entity::find()
        .filter(channel_members::Column::UserId.eq(user_id))
        .all(database)
        .await
        .map_err(internal_error)?;

    let channel_ids: Vec<Uuid> = memberships
        .into_iter()
        .map(|member| member.channel_id)
        .collect();

    if channel_ids.is_empty() {
        return Ok(vec![]);
    }

    let channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .filter(channels::Column::Id.is_in(channel_ids))
        .order_by_asc(channels::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(channels
        .into_iter()
        .map(|channel| shape_channel(channel, &server))
        .collect())
}

pub(crate) async fn get_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<ChannelResponse> {
    let server = load_server(database, server_id).await?;
    let channel = find_channel(database, server_id, channel_id).await?;
    Ok(shape_channel(channel, &server))
}

pub(crate) async fn create_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    request: ChannelRequest,
) -> AppResult<ChannelResponse> {
    let server = load_server(database, server_id).await?;
    let (name, description) = validate_channel_request(request)?;

    let channel = channels::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        name: Set(name),
        description: Set(description),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    let server_members = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;

    for member in server_members {
        let _ = channel_members::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(member.user_id),
            ..Default::default()
        }
        .insert(database)
        .await;
    }

    Ok(shape_channel(channel, &server))
}

pub(crate) async fn update_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    request: ChannelRequest,
) -> AppResult<()> {
    let (name, description) = validate_channel_request(request)?;
    let channel = find_channel(database, server_id, channel_id).await?;
    let mut active = channel.into_active_model();
    active.name = Set(name);
    active.description = Set(description);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn delete_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<()> {
    let channel = find_channel(database, server_id, channel_id).await?;
    channel.delete(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn get_feed(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<FeedMessageResponse>> {
    find_channel(database, server_id, channel_id).await?;

    let messages = messages::Entity::find()
        .filter(messages::Column::ChannelId.eq(channel_id))
        .order_by_desc(messages::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    let user_ids: Vec<i64> = messages.iter().map(|message| message.user_id).collect();
    let message_ids: Vec<Uuid> = messages.iter().map(|message| message.id).collect();

    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids))
        .all(database)
        .await
        .map_err(internal_error)?;
    let images = message_images::Entity::find()
        .filter(message_images::Column::MessageId.is_in(message_ids))
        .order_by_asc(message_images::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(messages
        .into_iter()
        .map(|message| FeedMessageResponse {
            kind: "message",
            message: shape_message(
                &message,
                users.iter().find(|user| user.id == message.user_id),
                images.iter().filter(|image| image.message_id == message.id),
            ),
        })
        .collect())
}

pub(crate) async fn create_message(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: i64,
    request: CreateMessageRequest,
) -> AppResult<MessageResponse> {
    validate_create_message(&request)?;
    find_channel(database, server_id, channel_id).await?;
    ensure_channel_membership(database, channel_id, user_id).await?;

    let body = request
        .body
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    let message = messages::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        channel_id: Set(channel_id),
        user_id: Set(user_id),
        body: Set(body),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    let mut shaped_images = Vec::with_capacity(request.image_count);
    for _ in 0..request.image_count {
        let image = message_images::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            message_id: Set(message.id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;

        shaped_images.push(shape_image(&image, true));
    }

    let user = users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;

    Ok(MessageResponse {
        id: message.id.to_string(),
        body: message.body,
        images: shaped_images,
        user: Some(MessageUser {
            id: user.id.to_string(),
            name: user.name,
            profile_picture: None,
        }),
        user_id: Some(user.id.to_string()),
        bot_id: None,
        bot: None,
        command_status: None,
        created_at: serialize_timestamp(message.created_at),
    })
}

pub(crate) async fn store_message_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    server_id: Uuid,
    channel_id: Uuid,
    message_id: Uuid,
    image_id: Uuid,
    user_id: i64,
    content_type: Option<String>,
    bytes: Vec<u8>,
) -> AppResult<ImageResponse> {
    if bytes.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No image uploaded",
        ));
    }

    let message = messages::Entity::find_by_id(message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Message not found."))?;

    if message.channel_id != channel_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Message not found."));
    }

    find_channel(database, server_id, channel_id).await?;
    ensure_channel_membership(database, channel_id, user_id).await?;

    if message.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }

    let image = message_images::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Image not found."))?;

    if image.message_id != message_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Image not found."));
    }

    let storage_key = format!("message-images/{image_id}");
    let destination = upload_root.join(&storage_key);

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_error)?;
    }

    tokio::fs::write(&destination, bytes)
        .await
        .map_err(internal_error)?;

    let mut active = image.into_active_model();
    active.storage_key = Set(Some(storage_key));
    active.content_type = Set(content_type);
    let image = active.update(database).await.map_err(internal_error)?;

    Ok(ImageResponse {
        id: image.id.to_string(),
        is_placeholder: None,
        created_at: serialize_timestamp(image.created_at),
    })
}

pub(crate) async fn get_message_image(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    message_id: Uuid,
    image_id: Uuid,
) -> AppResult<StoredImage> {
    let message = messages::Entity::find_by_id(message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Message not found."))?;

    if message.channel_id != channel_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Message not found."));
    }

    find_channel(database, server_id, channel_id).await?;

    let image = message_images::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Image not found."))?;

    if image.message_id != message_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Image not found."));
    }

    Ok(StoredImage {
        storage_key: image.storage_key,
        content_type: image.content_type,
    })
}

pub(crate) fn upload_root() -> PathBuf {
    std::env::temp_dir().join("praxis-live-chat-uploads")
}

pub(crate) fn resolve_upload_path(upload_root: &Path, storage_key: &str) -> PathBuf {
    upload_root.join(storage_key)
}

fn shape_channel(channel: channels::Model, server: &servers::Model) -> ChannelResponse {
    ChannelResponse {
        id: channel.id.to_string(),
        name: channel.name,
        description: channel.description,
        server: ChannelServer {
            id: server.id.to_string(),
            slug: server.slug.clone(),
        },
    }
}

fn shape_message<'a>(
    message: &messages::Model,
    user: Option<&users::Model>,
    images: impl Iterator<Item = &'a message_images::Model>,
) -> MessageResponse {
    MessageResponse {
        id: message.id.to_string(),
        body: message.body.clone(),
        images: images
            .map(|image| shape_image(image, image.storage_key.is_none()))
            .collect(),
        user: user.map(|user| MessageUser {
            id: user.id.to_string(),
            name: user.name.clone(),
            profile_picture: None,
        }),
        user_id: Some(message.user_id.to_string()),
        bot_id: None,
        bot: None,
        command_status: None,
        created_at: serialize_timestamp(message.created_at),
    }
}

fn shape_image(image: &message_images::Model, is_placeholder: bool) -> ImageResponse {
    ImageResponse {
        id: image.id.to_string(),
        is_placeholder: is_placeholder.then_some(true),
        created_at: serialize_timestamp(image.created_at),
    }
}

async fn load_server(database: &DatabaseConnection, server_id: Uuid) -> AppResult<servers::Model> {
    servers::Entity::find_by_id(server_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Server not found."))
}

async fn ensure_server(database: &DatabaseConnection, server_id: Uuid) -> AppResult<()> {
    load_server(database, server_id).await.map(|_| ())
}

async fn find_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<channels::Model> {
    channels::Entity::find_by_id(channel_id)
        .filter(channels::Column::ServerId.eq(server_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Channel not found."))
}

async fn ensure_channel_membership(
    database: &DatabaseConnection,
    channel_id: Uuid,
    user_id: i64,
) -> AppResult<()> {
    let membership = channel_members::Entity::find()
        .filter(channel_members::Column::ChannelId.eq(channel_id))
        .filter(channel_members::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?;

    if membership.is_some() {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}

fn validate_create_message(request: &CreateMessageRequest) -> AppResult<()> {
    if request.image_count > MAX_IMAGE_COUNT {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("A message can include at most {MAX_IMAGE_COUNT} images."),
        ));
    }

    let has_body = request
        .body
        .as_ref()
        .map(|body| !body.trim().is_empty())
        .unwrap_or(false);

    if has_body || request.image_count > 0 {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "A message must include text or at least one image.",
        ))
    }
}

fn validate_channel_request(request: ChannelRequest) -> AppResult<(String, Option<String>)> {
    let name = request.name.trim().to_ascii_lowercase();
    if !(2..=30).contains(&name.chars().count()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Channel name must be between 2 and 30 characters.",
        ));
    }

    let description = request
        .description
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    if description
        .as_ref()
        .map(|value| value.chars().count() > 255)
        .unwrap_or(false)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Channel description must be at most 255 characters.",
        ));
    }

    Ok((name, description))
}

fn default_server_id() -> Uuid {
    DEFAULT_SERVER_ID
        .parse()
        .expect("default server id should be valid")
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("chat request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
