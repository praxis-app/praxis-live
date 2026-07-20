use axum::http::StatusCode;
use chrono::Utc;
use entity::{enums::ForumPostStatus, forum_posts, message_images, messages};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid as NativeUuid;

use super::{
    responses::{shape_forum_post, shape_post_summaries},
    types::{
        CreateForumPostRequest, CreateForumReplyRequest, ForumPostResponse,
        ForumPostSummaryResponse, UpdateForumPostRequest,
    },
};
use crate::{
    channels,
    common::{encryption, text::sanitize_text, ApiError, AppResult},
    messages::{self as messages_service, types::MessageResponse},
    polls::service as polls_service,
};

const MAX_POST_TITLE_LENGTH: usize = 100;

pub(crate) async fn list_forum_posts(
    database: &DatabaseConnection,
    channel_id: Uuid,
    sort: Option<&str>,
    status: Option<&str>,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<ForumPostSummaryResponse>> {
    let status = parse_status_filter(status)?;
    let sort = parse_sort(sort)?;
    let mut select = forum_posts::Entity::find()
        .filter(forum_posts::Column::ChannelId.eq(channel_id));
    if let Some(status) = status {
        select = select.filter(forum_posts::Column::Status.eq(status));
    }
    select = match sort {
        ForumPostSort::Recent => select
            .order_by_desc(forum_posts::Column::LatestActivityAt)
            .order_by_desc(forum_posts::Column::Id),
        ForumPostSort::Newest => select
            .order_by_desc(forum_posts::Column::CreatedAt)
            .order_by_desc(forum_posts::Column::Id),
    };

    let posts = select
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;
    shape_post_summaries(database, posts).await
}

pub(crate) async fn create_forum_post(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    request: CreateForumPostRequest,
) -> AppResult<ForumPostResponse> {
    let title = validate_title(&request.title)?;
    let body = validate_body(&request.body, "A forum post body is required.")?;
    let prepared_proposal = match request.proposal {
        Some(proposal) => Some(
            polls_service::prepare_forum_proposal(
                database, server_id, channel_id, user_id, proposal,
            )
            .await?,
        ),
        None => None,
    };
    let (key, unwrapped_key) =
        channels::get_unwrapped_channel_key(database, channel_id).await?;
    let encrypted_title = encryption::encrypt_text(&title, &unwrapped_key)?;
    let encrypted_body = encryption::encrypt_text(&body, &unwrapped_key)?;
    let now = Utc::now().fixed_offset();
    let post_id = NativeUuid::new_v4();
    let root_message_id = NativeUuid::new_v4();

    let transaction = database.begin().await.map_err(internal_error)?;
    let proposal = match prepared_proposal {
        Some(prepared) => Some(
            polls_service::insert_prepared_poll(
                &transaction,
                None,
                user_id,
                prepared,
            )
            .await?,
        ),
        None => None,
    };

    messages::ActiveModel {
        id: Set(root_message_id),
        channel_id: Set(channel_id),
        user_id: Set(user_id),
        key_id: Set(Some(key.id)),
        ciphertext: Set(Some(encrypted_body.ciphertext)),
        iv: Set(Some(encrypted_body.iv)),
        tag: Set(Some(encrypted_body.tag)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;

    forum_posts::ActiveModel {
        id: Set(post_id),
        channel_id: Set(channel_id),
        source_channel_id: Set(None),
        user_id: Set(user_id),
        root_message_id: Set(root_message_id),
        poll_id: Set(proposal.as_ref().map(|proposal| proposal.id)),
        ciphertext: Set(encrypted_title.ciphertext),
        iv: Set(encrypted_title.iv),
        tag: Set(encrypted_title.tag),
        key_id: Set(key.id),
        status: Set(ForumPostStatus::Open),
        latest_activity_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    get_forum_post(database, channel_id, post_id, user_id).await
}

pub(crate) async fn get_forum_post(
    database: &DatabaseConnection,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostResponse> {
    shape_forum_post(
        database,
        load_post(database, channel_id, post_id).await?,
        user_id,
    )
    .await
}

pub(crate) async fn update_forum_post(
    database: &DatabaseConnection,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: UpdateForumPostRequest,
) -> AppResult<ForumPostResponse> {
    if request.title.is_none() && request.body.is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "A forum post update is required.",
        ));
    }

    let title = request.title.as_deref().map(validate_title).transpose()?;
    let body = request
        .body
        .as_deref()
        .map(|body| validate_body(body, "A forum post body is required."))
        .transpose()?;
    let encrypted = if title.is_some() || body.is_some() {
        let (key, unwrapped_key) =
            channels::get_unwrapped_channel_key(database, channel_id).await?;
        let title = title
            .as_deref()
            .map(|title| encryption::encrypt_text(title, &unwrapped_key))
            .transpose()?;
        let body = body
            .as_deref()
            .map(|body| encryption::encrypt_text(body, &unwrapped_key))
            .transpose()?;
        Some((key.id, title, body))
    } else {
        None
    };

    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(post.user_id, user_id, "Only the post author can edit it.")?;
    let root_message_id = post.root_message_id;
    let now = Utc::now().fixed_offset();
    let mut active = post.into_active_model();
    if let Some((key_id, Some(encrypted_title), _)) = encrypted.as_ref() {
        active.key_id = Set(*key_id);
        active.ciphertext = Set(encrypted_title.ciphertext.clone());
        active.iv = Set(encrypted_title.iv.clone());
        active.tag = Set(encrypted_title.tag.clone());
    }
    active.updated_at = Set(now);
    active.update(&transaction).await.map_err(internal_error)?;

    if let Some((key_id, _, Some(encrypted_body))) = encrypted {
        let root = messages::Entity::find_by_id(root_message_id)
            .one(&transaction)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| {
                internal_consistency_error("Post root message not found.")
            })?;
        let mut root = root.into_active_model();
        root.key_id = Set(Some(key_id));
        root.ciphertext = Set(Some(encrypted_body.ciphertext));
        root.iv = Set(Some(encrypted_body.iv));
        root.tag = Set(Some(encrypted_body.tag));
        root.updated_at = Set(now);
        root.update(&transaction).await.map_err(internal_error)?;
    }

    transaction.commit().await.map_err(internal_error)?;
    get_forum_post(database, channel_id, post_id, user_id).await
}

pub(crate) async fn create_forum_post_proposal(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: crate::polls::types::CreatePollRequest,
) -> AppResult<ForumPostResponse> {
    let prepared = polls_service::prepare_forum_proposal(
        database, server_id, channel_id, user_id, request,
    )
    .await?;
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(
        post.user_id,
        user_id,
        "Only the post author can create its proposal.",
    )?;
    if post.poll_id.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "This forum post already has a proposal.",
        ));
    }
    let proposal = polls_service::insert_prepared_poll(
        &transaction,
        None,
        user_id,
        prepared,
    )
    .await?;
    let mut active = post.into_active_model();
    active.poll_id = Set(Some(proposal.id));
    active.updated_at = Set(Utc::now().fixed_offset());
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    get_forum_post(database, channel_id, post_id, user_id).await
}

pub(crate) async fn close_forum_post(
    database: &DatabaseConnection,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostResponse> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(post.user_id, user_id, "Only the post author can close it.")?;
    if post.status != ForumPostStatus::Closed {
        let mut active = post.into_active_model();
        active.status = Set(ForumPostStatus::Closed);
        active.updated_at = Set(Utc::now().fixed_offset());
        active.update(&transaction).await.map_err(internal_error)?;
    }
    transaction.commit().await.map_err(internal_error)?;
    get_forum_post(database, channel_id, post_id, user_id).await
}

pub(crate) async fn create_forum_reply(
    database: &DatabaseConnection,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: CreateForumReplyRequest,
) -> AppResult<(MessageResponse, ForumPostSummaryResponse)> {
    messages_service::validate_message_content(
        Some(&request.body),
        request.image_count,
    )?;
    let body = sanitize_text(&request.body);
    let body = (!body.is_empty()).then_some(body);
    let encrypted = match body.as_deref() {
        Some(body) => {
            let (key, unwrapped_key) =
                channels::get_unwrapped_channel_key(database, channel_id)
                    .await?;
            Some((key.id, encryption::encrypt_text(body, &unwrapped_key)?))
        }
        None => None,
    };
    let reply_id = NativeUuid::new_v4();
    let now = Utc::now().fixed_offset();

    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    if post.status == ForumPostStatus::Closed {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "Closed forum posts cannot receive replies.",
        ));
    }
    let parent_message_id =
        request.parent_message_id.unwrap_or(post.root_message_id);
    validate_reply_parent(
        &transaction,
        channel_id,
        post.root_message_id,
        parent_message_id,
    )
    .await?;

    let reply = messages::ActiveModel {
        id: Set(reply_id),
        channel_id: Set(channel_id),
        user_id: Set(user_id),
        key_id: Set(encrypted.as_ref().map(|(key_id, _)| *key_id)),
        ciphertext: Set(encrypted
            .as_ref()
            .map(|(_, value)| value.ciphertext.clone())),
        iv: Set(encrypted.as_ref().map(|(_, value)| value.iv.clone())),
        tag: Set(encrypted.as_ref().map(|(_, value)| value.tag.clone())),
        thread_root_id: Set(Some(post.root_message_id)),
        parent_message_id: Set(Some(parent_message_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;
    for _ in 0..request.image_count {
        message_images::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            message_id: Set(reply.id),
            ..Default::default()
        }
        .insert(&transaction)
        .await
        .map_err(internal_error)?;
    }
    let mut active = post.into_active_model();
    active.latest_activity_at = Set(now);
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    let reply = messages_service::shape_messages(database, vec![reply])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Reply not found."))?;
    let post = load_post(database, channel_id, post_id).await?;
    let summary = shape_post_summaries(database, vec![post])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Post not found."))?;
    Ok((reply, summary))
}

pub(crate) async fn delete_forum_reply(
    database: &DatabaseConnection,
    channel_id: Uuid,
    post_id: Uuid,
    reply_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostSummaryResponse> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    let reply = messages::Entity::find_by_id(reply_id)
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::ThreadRootId.eq(post.root_message_id))
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Reply not found.")
        })?;
    ensure_owner(
        reply.user_id,
        user_id,
        "Only the reply author can delete it.",
    )?;
    reply.delete(&transaction).await.map_err(internal_error)?;

    let latest_reply = messages::Entity::find()
        .filter(messages::Column::ThreadRootId.eq(post.root_message_id))
        .order_by_desc(messages::Column::CreatedAt)
        .one(&transaction)
        .await
        .map_err(internal_error)?;
    let latest_activity_at = latest_reply
        .map(|reply| reply.created_at)
        .unwrap_or(post.created_at);
    let mut active = post.into_active_model();
    active.latest_activity_at = Set(latest_activity_at);
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    let post = load_post(database, channel_id, post_id).await?;
    shape_post_summaries(database, vec![post])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Post not found."))
}

async fn load_post<C>(
    database: &C,
    channel_id: Uuid,
    post_id: Uuid,
) -> AppResult<forum_posts::Model>
where
    C: ConnectionTrait,
{
    forum_posts::Entity::find_by_id(post_id)
        .filter(forum_posts::Column::ChannelId.eq(channel_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Post not found."))
}

async fn load_post_for_update<C>(
    database: &C,
    channel_id: Uuid,
    post_id: Uuid,
) -> AppResult<forum_posts::Model>
where
    C: ConnectionTrait,
{
    forum_posts::Entity::find_by_id(post_id)
        .filter(forum_posts::Column::ChannelId.eq(channel_id))
        .lock_exclusive()
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Post not found."))
}

async fn validate_reply_parent<C>(
    database: &C,
    channel_id: Uuid,
    root_message_id: Uuid,
    parent_message_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let parent = messages::Entity::find_by_id(parent_message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Parent message not found.")
        })?;
    if parent.channel_id != channel_id
        || (parent.id != root_message_id
            && parent.thread_root_id != Some(root_message_id))
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Parent message must belong to the same forum post.",
        ));
    }
    Ok(())
}

pub(super) fn validate_title(value: &str) -> AppResult<String> {
    let title = sanitize_text(value);
    let length = title.chars().count();
    if (1..=MAX_POST_TITLE_LENGTH).contains(&length) {
        Ok(title)
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Forum post title must be between 1 and {MAX_POST_TITLE_LENGTH} characters."
            ),
        ))
    }
}

pub(super) fn validate_body(
    value: &str,
    message: &'static str,
) -> AppResult<String> {
    let body = sanitize_text(value);
    if body.is_empty() {
        Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, message))
    } else {
        Ok(body)
    }
}

fn ensure_owner(
    owner_id: Uuid,
    user_id: Uuid,
    message: &'static str,
) -> AppResult<()> {
    if owner_id == user_id {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, message))
    }
}

#[derive(Clone, Copy)]
enum ForumPostSort {
    Recent,
    Newest,
}

fn parse_sort(value: Option<&str>) -> AppResult<ForumPostSort> {
    match value.unwrap_or("recent") {
        "recent" => Ok(ForumPostSort::Recent),
        "newest" => Ok(ForumPostSort::Newest),
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Forum post sort must be recent or newest.",
        )),
    }
}

fn parse_status_filter(
    value: Option<&str>,
) -> AppResult<Option<ForumPostStatus>> {
    match value {
        None => Ok(None),
        Some("open") => Ok(Some(ForumPostStatus::Open)),
        Some("closed") => Ok(Some(ForumPostStatus::Closed)),
        Some(_) => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Forum post status must be open or closed.",
        )),
    }
}

fn internal_consistency_error(message: &'static str) -> ApiError {
    tracing::error!("forum data is inconsistent: {message}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("forum request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
