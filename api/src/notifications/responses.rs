use entity::{
    channel_members, channels, forum_posts, messages, notifications, polls,
    server_roles, users,
};
use sea_orm::{
    prelude::Uuid, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::collections::{HashMap, HashSet};

use super::{
    service::internal_error,
    types::{
        NotificationResponse, NotificationTargetResponse,
        NotificationUserResponse,
    },
};
use crate::{
    common::AppResult, messages::types::serialize_timestamp,
    users as users_service,
};

/// Everything a page of notifications needs to render, loaded once rather than
/// per row.
struct NotificationContext {
    actors: HashMap<Uuid, users::Model>,
    profile_pictures:
        std::collections::BTreeMap<Uuid, users_service::UserImageRef>,
    channels: HashMap<Uuid, channels::Model>,
    readable_channels_by_viewer: HashMap<Uuid, HashSet<Uuid>>,
    messages: HashMap<Uuid, messages::Model>,
    forum_posts_by_root: HashMap<Uuid, Uuid>,
    polls: HashMap<Uuid, polls::Model>,
    server_roles: HashMap<Uuid, server_roles::Model>,
}

pub(super) async fn shape_notifications(
    database: &DatabaseConnection,
    viewer_id: Uuid,
    rows: Vec<notifications::Model>,
) -> AppResult<Vec<NotificationResponse>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let context = load_context(database, &[viewer_id], &rows).await?;

    Ok(rows
        .into_iter()
        .map(|row| shape_notification(row, viewer_id, &context))
        .collect())
}

pub(super) async fn shape_notifications_for_recipients(
    database: &DatabaseConnection,
    rows: Vec<notifications::Model>,
) -> AppResult<Vec<NotificationResponse>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let viewer_ids = unique(rows.iter().map(|row| row.recipient_user_id));
    let context = load_context(database, &viewer_ids, &rows).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let viewer_id = row.recipient_user_id;
            shape_notification(row, viewer_id, &context)
        })
        .collect())
}

async fn load_context(
    database: &DatabaseConnection,
    viewer_ids: &[Uuid],
    rows: &[notifications::Model],
) -> AppResult<NotificationContext> {
    let actor_ids = unique(rows.iter().filter_map(|row| row.actor_user_id));
    let message_ids = unique(rows.iter().filter_map(|row| row.message_id));
    let poll_ids = unique(rows.iter().filter_map(|row| row.poll_id));
    let role_ids = unique(rows.iter().filter_map(|row| row.server_role_id));

    let actors = load_by_id(
        users::Entity::find(),
        users::Column::Id,
        &actor_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|user| (user.id, user))
    .collect();
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &actor_ids)
            .await?;

    let messages = load_by_id(
        messages::Entity::find(),
        messages::Column::Id,
        &message_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|message| (message.id, message))
    .collect::<HashMap<Uuid, messages::Model>>();

    let polls = load_by_id(
        polls::Entity::find(),
        polls::Column::Id,
        &poll_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|poll| (poll.id, poll))
    .collect::<HashMap<Uuid, polls::Model>>();

    let server_roles = load_by_id(
        server_roles::Entity::find(),
        server_roles::Column::Id,
        &role_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|role| (role.id, role))
    .collect();

    // A forum post is the one whose unique root message is the notified
    // message, or that message's thread root, so it is derived rather than
    // stored.
    let root_message_ids = unique(messages.values().flat_map(|message| {
        [Some(message.id), message.thread_root_id]
            .into_iter()
            .flatten()
    }));
    let forum_posts_by_root = load_by_id(
        forum_posts::Entity::find(),
        forum_posts::Column::RootMessageId,
        &root_message_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|post| (post.root_message_id, post.id))
    .collect();

    let channel_ids = unique(
        rows.iter()
            .filter_map(|row| row.channel_id)
            .chain(messages.values().map(|message| message.channel_id))
            .chain(polls.values().map(|poll| poll.channel_id)),
    );
    let channels = load_by_id(
        channels::Entity::find(),
        channels::Column::Id,
        &channel_ids,
        database,
    )
    .await?
    .into_iter()
    .map(|channel| (channel.id, channel))
    .collect();
    let readable_channels_by_viewer =
        readable_channels(database, viewer_ids, &channel_ids).await?;

    Ok(NotificationContext {
        actors,
        profile_pictures,
        channels,
        readable_channels_by_viewer,
        messages,
        forum_posts_by_root,
        polls,
        server_roles,
    })
}

fn shape_notification(
    row: notifications::Model,
    viewer_id: Uuid,
    context: &NotificationContext,
) -> NotificationResponse {
    let actor = row.actor_user_id.and_then(|actor_id| {
        context
            .actors
            .get(&actor_id)
            .map(|actor| NotificationUserResponse {
                id: actor.id.to_string(),
                name: actor.name.clone(),
                display_name: actor.display_name.clone(),
                profile_picture: context
                    .profile_pictures
                    .get(&actor_id)
                    .cloned(),
            })
    });

    NotificationResponse {
        id: row.id.to_string(),
        kind: row.kind.as_str(),
        server_id: row.server_id.to_string(),
        channel_id: row.channel_id.map(|id| id.to_string()),
        actor,
        vote_type: row.vote_type.map(|vote_type| vote_type.as_str()),
        unread_count: row.unread_count,
        read_at: row.read_at.map(serialize_timestamp),
        created_at: serialize_timestamp(row.created_at),
        target: shape_target(&row, viewer_id, context),
    }
}

fn shape_target(
    row: &notifications::Model,
    viewer_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    if let Some(message_id) = row.message_id {
        return shape_message_target(row, message_id, viewer_id, context);
    }
    if let Some(poll_id) = row.poll_id {
        return shape_poll_target(row, poll_id, viewer_id, context);
    }
    if let Some(role_id) = row.server_role_id {
        return shape_server_role_target(row, role_id, context);
    }
    unavailable()
}

fn shape_message_target(
    row: &notifications::Model,
    message_id: Uuid,
    viewer_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(message) = context.messages.get(&message_id) else {
        return unavailable();
    };
    if row.channel_id != Some(message.channel_id)
        || !channel_is_in_server(message.channel_id, row.server_id, context)
        || !viewer_can_read_channel(viewer_id, message.channel_id, context)
    {
        return unavailable();
    }

    let (thread_root_id, thread_root_kind) =
        match (message.thread_root_id, message.thread_poll_id) {
            (Some(root_id), _) => (Some(root_id.to_string()), Some("message")),
            (_, Some(poll_id)) => (Some(poll_id.to_string()), Some("poll")),
            _ => (None, None),
        };
    let forum_post_id = message.thread_root_id.unwrap_or(message.id);
    let forum_post_id = context
        .forum_posts_by_root
        .get(&forum_post_id)
        .map(|post_id| post_id.to_string());

    NotificationTargetResponse {
        kind: "message",
        available: true,
        channel_id: Some(message.channel_id.to_string()),
        channel_name: channel_name(message.channel_id, context),
        message_id: Some(message.id.to_string()),
        thread_root_id,
        thread_root_kind,
        forum_post_id,
        ..Default::default()
    }
}

fn shape_poll_target(
    row: &notifications::Model,
    poll_id: Uuid,
    viewer_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(poll) = context.polls.get(&poll_id) else {
        return unavailable();
    };
    if row.channel_id != Some(poll.channel_id)
        || !channel_is_in_server(poll.channel_id, row.server_id, context)
        || !viewer_can_read_channel(viewer_id, poll.channel_id, context)
    {
        return unavailable();
    }

    NotificationTargetResponse {
        kind: "poll",
        available: true,
        channel_id: Some(poll.channel_id.to_string()),
        channel_name: channel_name(poll.channel_id, context),
        poll_id: Some(poll.id.to_string()),
        ..Default::default()
    }
}

fn shape_server_role_target(
    row: &notifications::Model,
    role_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(role) = context
        .server_roles
        .get(&role_id)
        .filter(|role| role.server_id == row.server_id)
    else {
        return unavailable();
    };

    NotificationTargetResponse {
        kind: "serverRole",
        available: true,
        server_role_id: Some(role.id.to_string()),
        server_role_name: Some(role.name.clone()),
        ..Default::default()
    }
}

fn unavailable() -> NotificationTargetResponse {
    NotificationTargetResponse {
        kind: "unavailable",
        available: false,
        ..Default::default()
    }
}

fn channel_name(
    channel_id: Uuid,
    context: &NotificationContext,
) -> Option<String> {
    context
        .channels
        .get(&channel_id)
        .map(|channel| channel.name.clone())
}

async fn readable_channels(
    database: &DatabaseConnection,
    viewer_ids: &[Uuid],
    channel_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, HashSet<Uuid>>> {
    if viewer_ids.is_empty() || channel_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let memberships = channel_members::Entity::find()
        .filter(
            channel_members::Column::UserId.is_in(viewer_ids.iter().copied()),
        )
        .filter(
            channel_members::Column::ChannelId
                .is_in(channel_ids.iter().copied()),
        )
        .all(database)
        .await
        .map_err(internal_error)?;
    let mut readable_by_viewer: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    for membership in memberships {
        readable_by_viewer
            .entry(membership.user_id)
            .or_default()
            .insert(membership.channel_id);
    }
    Ok(readable_by_viewer)
}

fn viewer_can_read_channel(
    viewer_id: Uuid,
    channel_id: Uuid,
    context: &NotificationContext,
) -> bool {
    context
        .readable_channels_by_viewer
        .get(&viewer_id)
        .is_some_and(|channel_ids| channel_ids.contains(&channel_id))
}

fn channel_is_in_server(
    channel_id: Uuid,
    server_id: Uuid,
    context: &NotificationContext,
) -> bool {
    context
        .channels
        .get(&channel_id)
        .is_some_and(|channel| channel.server_id == server_id)
}

async fn load_by_id<E, C>(
    select: sea_orm::Select<E>,
    column: C,
    ids: &[Uuid],
    database: &DatabaseConnection,
) -> AppResult<Vec<E::Model>>
where
    E: EntityTrait,
    C: ColumnTrait,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    select
        .filter(column.is_in(ids.iter().copied()))
        .all(database)
        .await
        .map_err(internal_error)
}

fn unique(ids: impl Iterator<Item = Uuid>) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    ids.filter(|id| seen.insert(*id)).collect()
}
