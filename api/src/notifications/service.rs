use axum::http::StatusCode;
use chrono::Utc;
use entity::{enums::NotificationKind, notifications, users};
use sea_orm::{
    prelude::Uuid, sea_query::OnConflict, ColumnTrait, Condition,
    ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::collections::HashSet;
use uuid::Uuid as NativeUuid;

use super::{
    responses::shape_notifications,
    types::{
        ListNotificationsQuery, NewNotification, NotificationResponse,
        NotificationTarget, NotificationsResponse, UnreadCountResponse,
    },
};
use crate::{
    channels as channels_service,
    common::{pagination::PaginationCursor, ApiError, AppResult},
    pub_sub::{PubSubService, PubSubTopic},
    servers,
};

const DEFAULT_LIMIT: u64 = 25;
const MAX_LIMIT: u64 = 50;

#[cfg(test)]
mod tests;

/// The single creation seam. Callers pass their own transaction so the rows
/// land with the domain transition that produced them, and publish only after
/// that transaction commits.
pub(crate) async fn create_notifications<C>(
    database: &C,
    input: NewNotification,
) -> AppResult<Vec<notifications::Model>>
where
    C: ConnectionTrait,
{
    let recipient_ids = eligible_recipients(database, &input).await?;
    if recipient_ids.is_empty() {
        return Ok(Vec::new());
    }

    let (coalesced, remaining) = if input.kind == NotificationKind::NewMessage {
        coalesce_new_messages(database, &input, recipient_ids).await?
    } else {
        (Vec::new(), recipient_ids)
    };

    let mut created = coalesced;
    created.extend(insert_notifications(database, &input, remaining).await?);
    Ok(created)
}

/// Selects the recipients that should actually receive the notification:
/// never the actor, never anonymous users, and never someone who cannot
/// currently read the target.
async fn eligible_recipients<C>(
    database: &C,
    input: &NewNotification,
) -> AppResult<Vec<Uuid>>
where
    C: ConnectionTrait,
{
    let mut seen = HashSet::new();
    let candidates: Vec<Uuid> = input
        .recipient_ids
        .iter()
        .copied()
        .filter(|id| Some(*id) != input.actor_user_id && seen.insert(*id))
        .collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let registered: HashSet<Uuid> = users::Entity::find()
        .filter(users::Column::Id.is_in(candidates.iter().copied()))
        .filter(users::Column::Anonymous.eq(false))
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|user| user.id)
        .collect();

    let readers = match input.channel_id {
        Some(channel_id) => {
            channels_service::get_channel_member_user_ids(database, channel_id)
                .await?
                .into_iter()
                .collect::<HashSet<_>>()
        }
        None => {
            let mut members = HashSet::new();
            for candidate in &candidates {
                if servers::is_server_member(
                    database,
                    input.server_id,
                    *candidate,
                )
                .await?
                {
                    members.insert(*candidate);
                }
            }
            members
        }
    };

    Ok(candidates
        .into_iter()
        .filter(|id| registered.contains(id) && readers.contains(id))
        .collect())
}

/// While an unread `new_message` row exists for a `(recipient, channel)` pair,
/// bump it instead of inserting another, so a busy channel produces one inbox
/// entry rather than hundreds.
async fn coalesce_new_messages<C>(
    database: &C,
    input: &NewNotification,
    recipient_ids: Vec<Uuid>,
) -> AppResult<(Vec<notifications::Model>, Vec<Uuid>)>
where
    C: ConnectionTrait,
{
    let Some(channel_id) = input.channel_id else {
        return Ok((Vec::new(), recipient_ids));
    };

    let existing = notifications::Entity::find()
        .filter(
            notifications::Column::RecipientUserId
                .is_in(recipient_ids.iter().copied()),
        )
        .filter(notifications::Column::Kind.eq(NotificationKind::NewMessage))
        .filter(notifications::Column::ChannelId.eq(channel_id))
        .filter(notifications::Column::ReadAt.is_null())
        .all(database)
        .await
        .map_err(internal_error)?;
    if existing.is_empty() {
        return Ok((Vec::new(), recipient_ids));
    }

    let now = Utc::now().fixed_offset();
    let coalesced_ids: Vec<Uuid> = existing.iter().map(|row| row.id).collect();
    notifications::Entity::update_many()
        .col_expr(
            notifications::Column::UnreadCount,
            sea_orm::sea_query::Expr::col(notifications::Column::UnreadCount)
                .add(1),
        )
        .col_expr(
            notifications::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .col_expr(
            notifications::Column::MessageId,
            sea_orm::sea_query::Expr::value(target_message_id(input.target)),
        )
        .col_expr(
            notifications::Column::ActorUserId,
            sea_orm::sea_query::Expr::value(input.actor_user_id),
        )
        .filter(notifications::Column::Id.is_in(coalesced_ids.iter().copied()))
        .exec(database)
        .await
        .map_err(internal_error)?;

    let bumped: HashSet<Uuid> =
        existing.iter().map(|row| row.recipient_user_id).collect();
    let coalesced = notifications::Entity::find()
        .filter(notifications::Column::Id.is_in(coalesced_ids))
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok((
        coalesced,
        recipient_ids
            .into_iter()
            .filter(|id| !bumped.contains(id))
            .collect(),
    ))
}

async fn insert_notifications<C>(
    database: &C,
    input: &NewNotification,
    recipient_ids: Vec<Uuid>,
) -> AppResult<Vec<notifications::Model>>
where
    C: ConnectionTrait,
{
    if recipient_ids.is_empty() {
        return Ok(Vec::new());
    }

    let unread_count =
        (input.kind == NotificationKind::NewMessage).then_some(1);
    let ids: Vec<Uuid> =
        recipient_ids.iter().map(|_| NativeUuid::new_v4()).collect();
    let rows = recipient_ids.iter().zip(&ids).map(|(recipient_id, id)| {
        notifications::ActiveModel {
            id: Set(*id),
            recipient_user_id: Set(*recipient_id),
            actor_user_id: Set(input.actor_user_id),
            server_id: Set(input.server_id),
            channel_id: Set(input.channel_id),
            kind: Set(input.kind),
            message_id: Set(target_message_id(input.target)),
            poll_id: Set(target_poll_id(input.target)),
            server_role_id: Set(target_server_role_id(input.target)),
            vote_type: Set(input.vote_type),
            unread_count: Set(unread_count),
            ..Default::default()
        }
    });

    // The per-kind partial unique indexes make repeated processing of the same
    // event a no-op rather than a second row.
    let result = notifications::Entity::insert_many(rows)
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .exec_without_returning(database)
        .await;
    if let Err(DbErr::RecordNotInserted) = result {
        return Ok(Vec::new());
    }
    result.map_err(internal_error)?;

    notifications::Entity::find()
        .filter(notifications::Column::Id.is_in(ids))
        .all(database)
        .await
        .map_err(internal_error)
}

/// WebSockets only accelerate delivery, so a failed publish is logged and the
/// client recovers the row from the API.
pub(crate) async fn publish_notifications(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    created: &[notifications::Model],
) {
    for notification in created {
        let shaped = shape_notifications(
            database,
            notification.recipient_user_id,
            vec![notification.clone()],
        )
        .await;
        let Ok(mut shaped) = shaped else {
            tracing::warn!("failed to shape created notification");
            continue;
        };
        let Some(shaped) = shaped.pop() else {
            continue;
        };
        let topic = PubSubTopic::notification(
            notification.server_id,
            notification.recipient_user_id,
        )
        .to_string();
        let body = serde_json::json!({
            "type": "notification",
            "notification": shaped,
        });
        if let Err(error) = pub_sub_service.publish(&topic, body).await {
            tracing::warn!("failed to publish notification: {error}");
        }
    }
}

pub(super) async fn list_notifications(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
    query: ListNotificationsQuery,
) -> AppResult<NotificationsResponse> {
    ensure_member(database, server_id, user_id).await?;

    let limit = resolve_limit(query.limit);
    let mut select = scoped(server_id, user_id);
    if let Some(cursor) = query.before.as_deref() {
        select =
            select.filter(before_condition(PaginationCursor::parse(cursor)?));
    }
    let mut rows = select
        .order_by_desc(notifications::Column::CreatedAt)
        .order_by_desc(notifications::Column::Id)
        .limit(limit + 1)
        .all(database)
        .await
        .map_err(internal_error)?;

    let has_more = rows.len() as u64 > limit;
    rows.truncate(limit as usize);
    let next_cursor = has_more.then(|| rows.last()).flatten().map(|row| {
        PaginationCursor {
            created_at: row.created_at,
            id: row.id,
        }
        .encode()
    });

    Ok(NotificationsResponse {
        notifications: shape_notifications(database, user_id, rows).await?,
        next_cursor,
        has_more,
    })
}

pub(super) async fn unread_count(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<UnreadCountResponse> {
    ensure_member(database, server_id, user_id).await?;

    scoped(server_id, user_id)
        .filter(notifications::Column::ReadAt.is_null())
        .count(database)
        .await
        .map(|unread_count| UnreadCountResponse { unread_count })
        .map_err(internal_error)
}

pub(super) async fn set_read_state(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
    notification_id: Uuid,
    read: bool,
) -> AppResult<NotificationResponse> {
    ensure_member(database, server_id, user_id).await?;

    let notification = scoped(server_id, user_id)
        .filter(notifications::Column::Id.eq(notification_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    let read_at = read.then(|| Utc::now().fixed_offset());
    notifications::Entity::update_many()
        .col_expr(
            notifications::Column::ReadAt,
            sea_orm::sea_query::Expr::value(read_at),
        )
        .filter(notifications::Column::Id.eq(notification.id))
        .exec(database)
        .await
        .map_err(internal_error)?;

    let notification = notifications::Entity::find_by_id(notification.id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found)?;

    shape_notifications(database, user_id, vec![notification])
        .await?
        .pop()
        .ok_or_else(not_found)
}

pub(super) async fn mark_all_read(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<UnreadCountResponse> {
    ensure_member(database, server_id, user_id).await?;

    notifications::Entity::update_many()
        .col_expr(
            notifications::Column::ReadAt,
            sea_orm::sea_query::Expr::value(Some(Utc::now().fixed_offset())),
        )
        .filter(notifications::Column::RecipientUserId.eq(user_id))
        .filter(notifications::Column::ServerId.eq(server_id))
        .filter(notifications::Column::ReadAt.is_null())
        .exec(database)
        .await
        .map_err(internal_error)?;

    Ok(UnreadCountResponse { unread_count: 0 })
}

pub(super) async fn delete_notification(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
    notification_id: Uuid,
) -> AppResult<()> {
    ensure_member(database, server_id, user_id).await?;

    notifications::Entity::delete_many()
        .filter(notifications::Column::Id.eq(notification_id))
        .filter(notifications::Column::RecipientUserId.eq(user_id))
        .filter(notifications::Column::ServerId.eq(server_id))
        .exec(database)
        .await
        .map_err(internal_error)?;

    Ok(())
}

pub(super) async fn clear_notifications(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    ensure_member(database, server_id, user_id).await?;

    notifications::Entity::delete_many()
        .filter(notifications::Column::RecipientUserId.eq(user_id))
        .filter(notifications::Column::ServerId.eq(server_id))
        .exec(database)
        .await
        .map_err(internal_error)?;

    Ok(())
}

async fn ensure_member(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    if servers::is_server_member(database, server_id, user_id).await? {
        return Ok(());
    }
    Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
}

fn scoped(
    server_id: Uuid,
    user_id: Uuid,
) -> sea_orm::Select<notifications::Entity> {
    notifications::Entity::find()
        .filter(notifications::Column::RecipientUserId.eq(user_id))
        .filter(notifications::Column::ServerId.eq(server_id))
}

fn before_condition(cursor: PaginationCursor) -> Condition {
    Condition::any()
        .add(notifications::Column::CreatedAt.lt(cursor.created_at))
        .add(
            Condition::all()
                .add(notifications::Column::CreatedAt.eq(cursor.created_at))
                .add(notifications::Column::Id.lt(cursor.id)),
        )
}

fn resolve_limit(limit: Option<u64>) -> u64 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn target_message_id(target: NotificationTarget) -> Option<Uuid> {
    match target {
        NotificationTarget::Message(id) => Some(id),
        _ => None,
    }
}

fn target_poll_id(target: NotificationTarget) -> Option<Uuid> {
    match target {
        NotificationTarget::Poll(id) => Some(id),
        _ => None,
    }
}

fn target_server_role_id(target: NotificationTarget) -> Option<Uuid> {
    match target {
        NotificationTarget::ServerRole(id) => Some(id),
        _ => None,
    }
}

fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "Notification not found.")
}

pub(super) fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("notification request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
