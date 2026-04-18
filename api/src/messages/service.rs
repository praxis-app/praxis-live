use axum::http::StatusCode;
use entity::{message_images, messages, users};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::path::{Path, PathBuf};
use uuid::Uuid as NativeUuid;

use super::types::{
    serialize_timestamp, CreateMessageRequest, FeedMessageResponse,
    ImageResponse, MessageResponse, MessageUser, StoredImage,
};
use crate::{
    channels,
    common::{ApiError, AppResult},
    users as user_api,
};

const MAX_IMAGE_COUNT: usize = 8;

pub(crate) async fn get_feed(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<FeedMessageResponse>> {
    channels::get_channel(database, server_id, channel_id).await?;

    let messages = messages::Entity::find()
        .filter(messages::Column::ChannelId.eq(channel_id))
        .order_by_desc(messages::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    let user_ids: Vec<Uuid> =
        messages.iter().map(|message| message.user_id).collect();
    let message_ids: Vec<Uuid> =
        messages.iter().map(|message| message.id).collect();

    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.clone()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let profile_pictures =
        user_api::get_user_profile_pictures_map(database, &user_ids).await?;
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
                &profile_pictures,
                images.iter().filter(|image| image.message_id == message.id),
            ),
        })
        .collect())
}

pub(crate) async fn create_message(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    request: CreateMessageRequest,
) -> AppResult<MessageResponse> {
    validate_create_message(&request)?;
    channels::get_channel(database, server_id, channel_id).await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;

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
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })?;

    Ok(MessageResponse {
        id: message.id.to_string(),
        body: message.body,
        images: shaped_images,
        user: Some(MessageUser {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture: user_api::get_user_profile_picture(
                database, user_id,
            )
            .await?,
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
    user_id: Uuid,
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
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Message not found.")
        })?;

    if message.channel_id != channel_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Message not found."));
    }

    channels::get_channel(database, server_id, channel_id).await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;

    if message.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }

    let image = message_images::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;

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
    upload_root: &Path,
    server_id: Uuid,
    channel_id: Uuid,
    message_id: Uuid,
    image_id: Uuid,
) -> AppResult<StoredImage> {
    let message = messages::Entity::find_by_id(message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Message not found.")
        })?;

    if message.channel_id != channel_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Message not found."));
    }

    channels::get_channel(database, server_id, channel_id).await?;

    let image = message_images::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;

    if image.message_id != message_id {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Image not found."));
    }

    let storage_key = image.storage_key.ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, "Image not uploaded yet.")
    })?;
    let bytes = tokio::fs::read(resolve_upload_path(upload_root, &storage_key))
        .await
        .map_err(internal_error)?;

    Ok(StoredImage {
        content_type: image.content_type,
        bytes,
    })
}

pub(crate) fn upload_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("content")
}

fn resolve_upload_path(upload_root: &Path, storage_key: &str) -> PathBuf {
    upload_root.join(storage_key)
}

fn shape_message<'a>(
    message: &messages::Model,
    user: Option<&users::Model>,
    profile_pictures: &std::collections::BTreeMap<
        Uuid,
        crate::users::ImageReference,
    >,
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
            display_name: user.display_name.clone(),
            profile_picture: profile_pictures.get(&user.id).cloned(),
        }),
        user_id: Some(message.user_id.to_string()),
        bot_id: None,
        bot: None,
        command_status: None,
        created_at: serialize_timestamp(message.created_at),
    }
}

fn shape_image(
    image: &message_images::Model,
    is_placeholder: bool,
) -> ImageResponse {
    ImageResponse {
        id: image.id.to_string(),
        is_placeholder: is_placeholder.then_some(true),
        created_at: serialize_timestamp(image.created_at),
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

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("chat request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
