use axum::http::StatusCode;
use entity::{enums::ChannelType, forum_posts, messages};
use sea_orm::{
    prelude::{DateTimeWithTimeZone, Uuid},
    sea_query::Expr,
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use std::{collections::HashMap, path::Path};
use uuid::Uuid as NativeUuid;

use super::{
    service::{
        attach_message_creation_images, commit_message_creation,
        cursor_condition, internal_consistency_error, internal_error,
        shape_messages, validate_message_content,
    },
    types::{
        serialize_timestamp, CreateReplyContext, CreateReplyRequest,
        MessageResponse, ThreadResponse,
    },
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
};

#[derive(Debug)]
pub(crate) struct CreatedReply {
    pub(crate) reply: MessageResponse,
    pub(crate) reply_count: usize,
    pub(crate) latest_reply_at: String,
}

#[derive(FromQueryResult)]
struct ReplySummary {
    thread_root_id: Option<Uuid>,
    reply_count: i64,
    latest_reply_at: Option<DateTimeWithTimeZone>,
}

#[derive(FromQueryResult)]
struct ReplyParticipant {
    thread_root_id: Option<Uuid>,
    user_id: Uuid,
    latest_reply_at: DateTimeWithTimeZone,
}

#[derive(FromQueryResult)]
struct PollReplySummary {
    thread_poll_id: Option<Uuid>,
    reply_count: i64,
    latest_reply_at: Option<DateTimeWithTimeZone>,
}

#[derive(FromQueryResult)]
struct PollReplyParticipant {
    thread_poll_id: Option<Uuid>,
    user_id: Uuid,
    latest_reply_at: DateTimeWithTimeZone,
}

pub(super) async fn list_replies(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    root_message_id: Uuid,
    before: Option<&str>,
    after: Option<&str>,
    limit: u64,
) -> AppResult<ThreadResponse> {
    ensure_text_channel(database, server_id, channel_id).await?;
    let root = load_thread_root(database, channel_id, root_message_id).await?;
    if after.is_some() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Thread replies only support the before cursor.",
        ));
    }
    let cursor = before.map(PaginationCursor::parse).transpose()?;
    let mut query = messages::Entity::find()
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::ThreadRootId.eq(root_message_id));
    if let Some(cursor) = cursor {
        query =
            query.filter(cursor_condition(cursor, PaginationDirection::Older));
    }
    let mut replies = query
        .order_by_desc(messages::Column::CreatedAt)
        .order_by_desc(messages::Column::Id)
        .limit(limit.saturating_add(1))
        .all(database)
        .await
        .map_err(internal_error)?;
    let has_more = replies.len() > limit as usize;
    if has_more {
        replies.pop();
    }
    replies.reverse();

    let start_cursor = replies.last().map(message_cursor);
    let next_cursor = replies.first().map(message_cursor);
    let mut shaped = shape_messages(database, {
        let mut records = Vec::with_capacity(replies.len() + 1);
        records.push(root);
        records.extend(replies);
        records
    })
    .await?
    .into_iter();
    let root = shaped.next().ok_or_else(|| {
        internal_consistency_error("Thread root message not found.")
    })?;

    Ok(ThreadResponse {
        root,
        replies: shaped.collect(),
        start_cursor,
        next_cursor,
        has_more,
    })
}

pub(super) async fn create_reply(
    database: &DatabaseConnection,
    upload_root: &Path,
    context: CreateReplyContext,
    request: CreateReplyRequest,
    images: Vec<Vec<u8>>,
) -> AppResult<CreatedReply> {
    ensure_text_channel(database, context.server_id, context.channel_id)
        .await?;
    validate_message_content(request.body.as_deref(), images.len())?;
    let body = request
        .body
        .map(|value| sanitize_text(&value))
        .filter(|value| !value.is_empty());
    let encrypted = match body.as_deref() {
        Some(body) => {
            let (key, unwrapped_key) = channels::get_unwrapped_channel_key(
                database,
                context.channel_id,
            )
            .await?;
            Some((key.id, encryption::encrypt_text(body, &unwrapped_key)?))
        }
        None => None,
    };

    let transaction = database.begin().await.map_err(internal_error)?;
    load_thread_root(&transaction, context.channel_id, context.root_message_id)
        .await?;
    let parent_message_id =
        request.parent_message_id.unwrap_or(context.root_message_id);
    validate_reply_parent(
        &transaction,
        context.channel_id,
        context.root_message_id,
        parent_message_id,
    )
    .await?;
    let reply = messages::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        channel_id: Set(context.channel_id),
        user_id: Set(context.user_id),
        key_id: Set(encrypted.as_ref().map(|(key_id, _)| *key_id)),
        ciphertext: Set(encrypted
            .as_ref()
            .map(|(_, value)| value.ciphertext.clone())),
        iv: Set(encrypted.as_ref().map(|(_, value)| value.iv.clone())),
        tag: Set(encrypted.as_ref().map(|(_, value)| value.tag.clone())),
        thread_root_id: Set(Some(context.root_message_id)),
        parent_message_id: Set(Some(parent_message_id)),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;
    let image_paths = attach_message_creation_images(
        &transaction,
        upload_root,
        reply.id,
        images,
    )
    .await?;
    commit_message_creation(transaction, image_paths).await?;

    let (reply_count, latest_reply_at) =
        load_reply_summaries(database, vec![context.root_message_id])
            .await?
            .remove(&context.root_message_id)
            .ok_or_else(|| {
                internal_consistency_error("Reply summary not found.")
            })?;
    let latest_reply_at = serialize_timestamp(latest_reply_at);
    let reply = shape_messages(database, vec![reply])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Reply not found."))?;

    Ok(CreatedReply {
        reply,
        reply_count,
        latest_reply_at,
    })
}

pub(super) async fn broadcast_reply(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    created: &CreatedReply,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "threadReply",
        "rootKind": "message",
        "rootId": created.reply.thread_root_id,
        "rootMessageId": created.reply.thread_root_id,
        "reply": created.reply,
        "replyCount": created.reply_count,
        "latestReplyAt": created.latest_reply_at,
    });
    let members =
        channels::get_channel_member_user_ids(database, channel_id).await?;
    for member_id in members {
        let topic = PubSubTopic::new_message(server_id, channel_id, member_id)
            .to_string();
        pub_sub_service.publish(&topic, body.clone()).await?;
    }
    Ok(())
}

pub(super) async fn load_reply_summaries(
    database: &DatabaseConnection,
    root_ids: Vec<Uuid>,
) -> AppResult<HashMap<Uuid, (usize, DateTimeWithTimeZone)>> {
    if root_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(messages::Entity::find()
        .select_only()
        .column(messages::Column::ThreadRootId)
        .column_as(Expr::col(messages::Column::Id).count(), "reply_count")
        .column_as(
            Expr::col(messages::Column::CreatedAt).max(),
            "latest_reply_at",
        )
        .filter(messages::Column::ThreadRootId.is_in(root_ids))
        .group_by(messages::Column::ThreadRootId)
        .into_model::<ReplySummary>()
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .filter_map(|summary| {
            Some((
                summary.thread_root_id?,
                (summary.reply_count as usize, summary.latest_reply_at?),
            ))
        })
        .collect())
}

pub(super) async fn load_reply_participants(
    database: &DatabaseConnection,
    root_ids: Vec<Uuid>,
) -> AppResult<HashMap<Uuid, Vec<Uuid>>> {
    if root_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut participants = messages::Entity::find()
        .select_only()
        .column(messages::Column::ThreadRootId)
        .column(messages::Column::UserId)
        .column_as(
            Expr::col(messages::Column::CreatedAt).max(),
            "latest_reply_at",
        )
        .filter(messages::Column::ThreadRootId.is_in(root_ids))
        .group_by(messages::Column::ThreadRootId)
        .group_by(messages::Column::UserId)
        .into_model::<ReplyParticipant>()
        .all(database)
        .await
        .map_err(internal_error)?;
    participants.sort_by(|left, right| {
        right.latest_reply_at.cmp(&left.latest_reply_at)
    });

    let mut participants_by_root: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for participant in participants {
        let Some(root_id) = participant.thread_root_id else {
            continue;
        };
        let root_participants =
            participants_by_root.entry(root_id).or_default();
        if root_participants.len() < 3 {
            root_participants.push(participant.user_id);
        }
    }
    Ok(participants_by_root)
}

pub(crate) async fn load_poll_reply_summaries(
    database: &DatabaseConnection,
    poll_ids: Vec<Uuid>,
) -> AppResult<HashMap<Uuid, (usize, DateTimeWithTimeZone)>> {
    if poll_ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(messages::Entity::find()
        .select_only()
        .column(messages::Column::ThreadPollId)
        .column_as(Expr::col(messages::Column::Id).count(), "reply_count")
        .column_as(
            Expr::col(messages::Column::CreatedAt).max(),
            "latest_reply_at",
        )
        .filter(messages::Column::ThreadPollId.is_in(poll_ids))
        .group_by(messages::Column::ThreadPollId)
        .into_model::<PollReplySummary>()
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .filter_map(|summary| {
            Some((
                summary.thread_poll_id?,
                (summary.reply_count as usize, summary.latest_reply_at?),
            ))
        })
        .collect())
}

pub(crate) async fn load_poll_reply_participants(
    database: &DatabaseConnection,
    poll_ids: Vec<Uuid>,
) -> AppResult<HashMap<Uuid, Vec<Uuid>>> {
    if poll_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut participants = messages::Entity::find()
        .select_only()
        .column(messages::Column::ThreadPollId)
        .column(messages::Column::UserId)
        .column_as(
            Expr::col(messages::Column::CreatedAt).max(),
            "latest_reply_at",
        )
        .filter(messages::Column::ThreadPollId.is_in(poll_ids))
        .group_by(messages::Column::ThreadPollId)
        .group_by(messages::Column::UserId)
        .into_model::<PollReplyParticipant>()
        .all(database)
        .await
        .map_err(internal_error)?;
    participants.sort_by(|left, right| {
        right.latest_reply_at.cmp(&left.latest_reply_at)
    });

    let mut participants_by_poll: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for participant in participants {
        let Some(poll_id) = participant.thread_poll_id else {
            continue;
        };
        let poll_participants =
            participants_by_poll.entry(poll_id).or_default();
        if poll_participants.len() < 3 {
            poll_participants.push(participant.user_id);
        }
    }
    Ok(participants_by_poll)
}

async fn ensure_text_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<()> {
    let channel =
        channels::get_channel(database, server_id, channel_id).await?;
    if channel.channel_type != ChannelType::Text {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Message threads are only available in text channels.",
        ));
    }
    Ok(())
}

async fn load_thread_root<C>(
    database: &C,
    channel_id: Uuid,
    root_message_id: Uuid,
) -> AppResult<messages::Model>
where
    C: ConnectionTrait,
{
    let root = messages::Entity::find_by_id(root_message_id)
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::ThreadRootId.is_null())
        .filter(messages::Column::ThreadPollId.is_null())
        .filter(messages::Column::CallId.is_null())
        .filter(messages::Column::BotId.is_null())
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Thread root not found.")
        })?;
    let is_forum_root = forum_posts::Entity::find()
        .filter(forum_posts::Column::RootMessageId.eq(root_message_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some();
    if is_forum_root {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Thread root not found.",
        ));
    }
    Ok(root)
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
            "Parent message must belong to the same thread.",
        ));
    }
    Ok(())
}

fn message_cursor(message: &messages::Model) -> String {
    PaginationCursor {
        created_at: message.created_at,
        id: message.id,
    }
    .encode()
}
