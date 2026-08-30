use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    channel_members, channels, enums::NotificationKind, notifications,
    server_members, users,
};
use sea_orm::{
    prelude::Uuid,
    sea_query::{Expr, OnConflict},
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
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

    if input.kind == NotificationKind::NewMessage {
        upsert_new_messages(database, &input, recipient_ids).await
    } else {
        insert_notifications(database, &input, recipient_ids).await
    }
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

    let readers: HashSet<Uuid> = match input.channel_id {
        Some(channel_id) => {
            let channel_exists = channels::Entity::find_by_id(channel_id)
                .filter(channels::Column::ServerId.eq(input.server_id))
                .one(database)
                .await
                .map_err(internal_error)?
                .is_some();
            if !channel_exists {
                HashSet::new()
            } else {
                channel_members::Entity::find()
                    .filter(channel_members::Column::ChannelId.eq(channel_id))
                    .filter(
                        channel_members::Column::UserId
                            .is_in(candidates.iter().copied()),
                    )
                    .all(database)
                    .await
                    .map_err(internal_error)?
                    .into_iter()
                    .map(|membership| membership.user_id)
                    .collect()
            }
        }
        None => server_members::Entity::find()
            .filter(server_members::Column::ServerId.eq(input.server_id))
            .filter(
                server_members::Column::UserId
                    .is_in(candidates.iter().copied()),
            )
            .all(database)
            .await
            .map_err(internal_error)?
            .into_iter()
            .map(|membership| membership.user_id)
            .collect(),
    };

    Ok(candidates
        .into_iter()
        .filter(|id| registered.contains(id) && readers.contains(id))
        .collect())
}

/// Inserts a fresh unread row or atomically bumps the existing unread row.
async fn upsert_new_messages<C>(
    database: &C,
    input: &NewNotification,
    recipient_ids: Vec<Uuid>,
) -> AppResult<Vec<notifications::Model>>
where
    C: ConnectionTrait,
{
    if input.channel_id.is_none() {
        return Ok(Vec::new());
    };
    let rows = notification_rows(input, recipient_ids);
    notifications::Entity::insert_many(rows)
        .on_conflict(
            OnConflict::columns([
                notifications::Column::RecipientUserId,
                notifications::Column::ChannelId,
            ])
            .target_and_where(Expr::cust(
                "kind = 'new_message' AND read_at IS NULL",
            ))
            .value(
                notifications::Column::UnreadCount,
                Expr::col((
                    notifications::Entity,
                    notifications::Column::UnreadCount,
                ))
                .add(1),
            )
            .update_columns([
                notifications::Column::ActorUserId,
                notifications::Column::MessageId,
                notifications::Column::CreatedAt,
            ])
            .to_owned(),
        )
        .exec_with_returning_many(database)
        .await
        .map_err(internal_error)
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

    let rows = notification_rows(input, recipient_ids);

    // The per-kind partial unique indexes make repeated processing of the same
    // event a no-op rather than a second row.
    let result = notifications::Entity::insert_many(rows)
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .exec_with_returning_many(database)
        .await;
    if let Err(DbErr::RecordNotInserted) = result {
        return Ok(Vec::new());
    }
    result.map_err(internal_error)
}

fn notification_rows(
    input: &NewNotification,
    recipient_ids: Vec<Uuid>,
) -> impl Iterator<Item = notifications::ActiveModel> + '_ {
    let unread_count =
        (input.kind == NotificationKind::NewMessage).then_some(1);
    recipient_ids.into_iter().map(move |recipient_id| {
        notifications::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            recipient_user_id: Set(recipient_id),
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
    })
}

/// WebSockets only accelerate delivery, so a failed publish is logged and the
/// client recovers the row from the API.
pub(crate) async fn publish_notifications(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    created: &[notifications::Model],
) {
    let shaped = super::responses::shape_notifications_for_recipients(
        database,
        created.to_vec(),
    )
    .await;
    let Ok(shaped) = shaped else {
        tracing::warn!("failed to shape created notifications");
        return;
    };
    for (notification, shaped) in created.iter().zip(shaped) {
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

    let result = notifications::Entity::delete_many()
        .filter(notifications::Column::Id.eq(notification_id))
        .filter(notifications::Column::RecipientUserId.eq(user_id))
        .filter(notifications::Column::ServerId.eq(server_id))
        .exec(database)
        .await
        .map_err(internal_error)?;

    (result.rows_affected > 0)
        .then_some(())
        .ok_or_else(not_found)
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
    let registered = users::Entity::find_by_id(user_id)
        .filter(users::Column::Anonymous.eq(false))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some();
    if registered
        && servers::is_server_member(database, server_id, user_id).await?
    {
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
