use axum::http::StatusCode;
use entity::{message_images, messages, users};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, Condition,
    DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::path::{Path, PathBuf};
use uuid::Uuid as NativeUuid;

use super::types::{
    serialize_timestamp, CreateMessageRequest, ImageResponse, MessageResponse,
    MessageUser, StoredImage,
};
use crate::{
    channels,
    common::{
        encryption,
        pagination::{PaginationCursor, PaginationDirection},
        text::sanitize_text,
        ApiError, AppResult,
    },
    pub_sub::{PubSubService, PubSubTopic},
    users as users_service,
};

const MAX_IMAGE_COUNT: usize = 8;

pub(crate) async fn get_channel_message_feed(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    cursor: Option<PaginationCursor>,
    direction: PaginationDirection,
    limit: u64,
) -> AppResult<Vec<MessageResponse>> {
    channels::get_channel(database, server_id, channel_id).await?;

    let mut query = messages::Entity::find()
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::CallId.is_null());
    if let Some(cursor) = cursor {
        query = query.filter(cursor_condition(cursor, direction));
    }
    query = match direction {
        PaginationDirection::Older => query
            .order_by_desc(messages::Column::CreatedAt)
            .order_by_desc(messages::Column::Id),
        PaginationDirection::Newer => query
            .order_by_asc(messages::Column::CreatedAt)
            .order_by_asc(messages::Column::Id),
    };
    let messages = query
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    shape_messages(database, messages).await
}

pub(crate) async fn get_call_message_feed(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    cursor: Option<PaginationCursor>,
    direction: PaginationDirection,
    limit: u64,
) -> AppResult<Vec<MessageResponse>> {
    crate::calls::service::get_call(database, server_id, channel_id, call_id)
        .await?;

    let mut query = messages::Entity::find()
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::CallId.eq(call_id));
    if let Some(cursor) = cursor {
        query = query.filter(cursor_condition(cursor, direction));
    }
    query = match direction {
        PaginationDirection::Older => query
            .order_by_desc(messages::Column::CreatedAt)
            .order_by_desc(messages::Column::Id),
        PaginationDirection::Newer => query
            .order_by_asc(messages::Column::CreatedAt)
            .order_by_asc(messages::Column::Id),
    };
    let messages = query
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    shape_messages(database, messages).await
}

fn cursor_condition(
    cursor: PaginationCursor,
    direction: PaginationDirection,
) -> Condition {
    let timestamp_comparison = match direction {
        PaginationDirection::Older => {
            messages::Column::CreatedAt.lt(cursor.created_at)
        }
        PaginationDirection::Newer => {
            messages::Column::CreatedAt.gt(cursor.created_at)
        }
    };
    let id_comparison = match direction {
        PaginationDirection::Older => messages::Column::Id.lt(cursor.id),
        PaginationDirection::Newer => messages::Column::Id.gt(cursor.id),
    };

    Condition::any().add(timestamp_comparison).add(
        Condition::all()
            .add(messages::Column::CreatedAt.eq(cursor.created_at))
            .add(id_comparison),
    )
}

pub(crate) async fn create_message(
    database: &DatabaseConnection,
    channel_id: Uuid,
    user_id: Uuid,
    request: CreateMessageRequest,
) -> AppResult<MessageResponse> {
    create_message_record(database, channel_id, None, user_id, request).await
}

pub(crate) async fn create_call_message(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    user_id: Uuid,
    request: CreateMessageRequest,
) -> AppResult<MessageResponse> {
    crate::calls::service::get_call(database, server_id, channel_id, call_id)
        .await?;
    create_message_record(database, channel_id, Some(call_id), user_id, request)
        .await
}

pub(crate) async fn broadcast_message(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    message: &MessageResponse,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "message",
        "message": message,
    });

    broadcast_to_channel_members(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        sender_id,
        body,
    )
    .await
}

pub(crate) async fn broadcast_image_upload(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
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
        database,
        pub_sub_service,
        server_id,
        channel_id,
        sender_id,
        body,
    )
    .await
}

pub(crate) async fn broadcast_message_to_call(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    sender_id: Uuid,
    message: &MessageResponse,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "message",
        "message": message,
    });

    broadcast_to_call_members(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        call_id,
        sender_id,
        body,
    )
    .await
}

pub(crate) async fn broadcast_call_image_upload(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    sender_id: Uuid,
    message_id: &str,
    image_id: &str,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "image",
        "isPlaceholder": false,
        "messageId": message_id,
        "imageId": image_id,
    });

    broadcast_to_call_members(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        call_id,
        sender_id,
        body,
    )
    .await
}

async fn create_message_record(
    database: &DatabaseConnection,
    channel_id: Uuid,
    call_id: Option<Uuid>,
    user_id: Uuid,
    request: CreateMessageRequest,
) -> AppResult<MessageResponse> {
    validate_create_message(&request)?;

    let body = request
        .body
        .map(|value| sanitize_text(&value))
        .filter(|value| !value.is_empty());

    let encrypted = match body.as_deref() {
        Some(body) => {
            let (key, unwrapped_key) =
                channels::get_unwrapped_channel_key(database, channel_id)
                    .await?;
            Some((key.id, encryption::encrypt_text(body, &unwrapped_key)?))
        }
        None => None,
    };

    let message = messages::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        channel_id: Set(channel_id),
        call_id: Set(call_id),
        user_id: Set(user_id),
        key_id: Set(encrypted.as_ref().map(|(key_id, _)| *key_id)),
        ciphertext: Set(encrypted
            .as_ref()
            .map(|(_, value)| value.ciphertext.clone())),
        iv: Set(encrypted.as_ref().map(|(_, value)| value.iv.clone())),
        tag: Set(encrypted.as_ref().map(|(_, value)| value.tag.clone())),
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
        body,
        images: shaped_images,
        user: Some(MessageUser {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture: users_service::get_user_profile_picture(
                database, user_id,
            )
            .await?,
        }),
        user_id: Some(user.id.to_string()),
        bot_id: None,
        bot: None,
        command_status: None,
        thread_root_id: message.thread_root_id.map(|id| id.to_string()),
        parent_message_id: message.parent_message_id.map(|id| id.to_string()),
        created_at: serialize_timestamp(message.created_at),
    })
}

pub(crate) async fn shape_messages(
    database: &DatabaseConnection,
    messages: Vec<messages::Model>,
) -> AppResult<Vec<MessageResponse>> {
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
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;
    let images = message_images::Entity::find()
        .filter(message_images::Column::MessageId.is_in(message_ids))
        .order_by_asc(message_images::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let key_ids = messages
        .iter()
        .filter_map(|message| message.key_id)
        .collect::<Vec<_>>();
    let key_map =
        channels::get_unwrapped_channel_key_map(database, key_ids).await?;

    Ok(messages
        .into_iter()
        .map(|message| {
            shape_message(
                &message,
                users.iter().find(|user| user.id == message.user_id),
                &profile_pictures,
                images.iter().filter(|image| image.message_id == message.id),
                decrypt_message_body(&message, &key_map),
            )
        })
        .collect())
}

pub(crate) async fn store_message_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    message: &messages::Model,
    image_id: Uuid,
    content_type: Option<String>,
    bytes: Vec<u8>,
) -> AppResult<ImageResponse> {
    if bytes.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No image uploaded",
        ));
    }

    let image = message_images::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;

    if image.message_id != message.id {
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

pub(crate) async fn get_call_message_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    message_id: Uuid,
    image_id: Uuid,
) -> AppResult<StoredImage> {
    load_call_message(database, server_id, channel_id, call_id, message_id)
        .await?;
    get_message_image(
        database,
        upload_root,
        server_id,
        channel_id,
        message_id,
        image_id,
    )
    .await
}

pub(crate) async fn load_message(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    message_id: Uuid,
) -> AppResult<messages::Model> {
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

    Ok(message)
}

pub(crate) async fn load_call_message(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    message_id: Uuid,
) -> AppResult<messages::Model> {
    crate::calls::service::get_call(database, server_id, channel_id, call_id)
        .await?;
    let message =
        load_message(database, server_id, channel_id, message_id).await?;

    if message.call_id != Some(call_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "Message not found."));
    }

    Ok(message)
}

fn resolve_upload_path(upload_root: &Path, storage_key: &str) -> PathBuf {
    upload_root.join(storage_key)
}

fn shape_message<'a>(
    message: &messages::Model,
    user: Option<&users::Model>,
    profile_pictures: &std::collections::BTreeMap<
        Uuid,
        crate::users::UserImageRef,
    >,
    images: impl Iterator<Item = &'a message_images::Model>,
    body: Option<String>,
) -> MessageResponse {
    MessageResponse {
        id: message.id.to_string(),
        body,
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
        thread_root_id: message.thread_root_id.map(|id| id.to_string()),
        parent_message_id: message.parent_message_id.map(|id| id.to_string()),
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

fn decrypt_message_body(
    message: &messages::Model,
    key_map: &std::collections::HashMap<Uuid, Vec<u8>>,
) -> Option<String> {
    let (Some(ciphertext), Some(iv), Some(tag), Some(key_id)) = (
        message.ciphertext.as_ref(),
        message.iv.as_ref(),
        message.tag.as_ref(),
        message.key_id,
    ) else {
        return None;
    };
    let key = key_map.get(&key_id)?;
    encryption::decrypt_text(ciphertext, iv, tag, key).ok()
}

fn validate_create_message(request: &CreateMessageRequest) -> AppResult<()> {
    validate_message_content(request.body.as_deref(), request.image_count)
}

pub(crate) fn validate_message_content(
    body: Option<&str>,
    image_count: usize,
) -> AppResult<()> {
    if image_count > MAX_IMAGE_COUNT {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("A message can include at most {MAX_IMAGE_COUNT} images."),
        ));
    }

    let has_body = body
        .map(|body| !sanitize_text(body).is_empty())
        .unwrap_or(false);

    if has_body || image_count > 0 {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "A message must include text or at least one image.",
        ))
    }
}

async fn broadcast_to_call_members(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    sender_id: Uuid,
    body: serde_json::Value,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(database, channel_id).await?;

    for member_id in members {
        if member_id == sender_id {
            continue;
        }

        let topic = PubSubTopic::call_message(
            server_id, channel_id, call_id, member_id,
        )
        .to_string();
        pub_sub_service.publish(&topic, body.clone()).await?;
    }

    Ok(())
}

async fn broadcast_to_channel_members(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    body: serde_json::Value,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(database, channel_id).await?;

    for member_id in members {
        if member_id == sender_id {
            continue;
        }

        let topic = PubSubTopic::new_message(server_id, channel_id, member_id)
            .to_string();
        pub_sub_service.publish(&topic, body.clone()).await?;
    }

    Ok(())
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("chat request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
