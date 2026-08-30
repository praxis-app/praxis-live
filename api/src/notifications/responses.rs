use entity::{
    channel_members, channels, enums::NotificationKind, forum_posts, messages,
    notifications, polls, server_roles, users,
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
    readable_channels: HashSet<Uuid>,
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
    let context = load_context(database, viewer_id, &rows).await?;

    Ok(rows
        .into_iter()
        .map(|row| shape_notification(row, &context))
        .collect())
}

async fn load_context(
    database: &DatabaseConnection,
    viewer_id: Uuid,
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

    // A forum reply's post is the one whose unique root message is the reply's
    // thread root, so it is derived rather than stored.
    let thread_root_ids = unique(
        messages
            .values()
            .filter_map(|message| message.thread_root_id),
    );
    let forum_posts_by_root = load_by_id(
        forum_posts::Entity::find(),
        forum_posts::Column::RootMessageId,
        &thread_root_ids,
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
    let readable_channels =
        readable_channels(database, viewer_id, &channel_ids).await?;

    Ok(NotificationContext {
        actors,
        profile_pictures,
        channels,
        readable_channels,
        messages,
        forum_posts_by_root,
        polls,
        server_roles,
    })
}

fn shape_notification(
    row: notifications::Model,
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
        target: shape_target(&row, context),
    }
}

fn shape_target(
    row: &notifications::Model,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    if let Some(message_id) = row.message_id {
        return shape_message_target(row, message_id, context);
    }
    if let Some(poll_id) = row.poll_id {
        return shape_poll_target(poll_id, context);
    }
    if let Some(role_id) = row.server_role_id {
        return shape_server_role_target(role_id, context);
    }
    unavailable()
}

fn shape_message_target(
    row: &notifications::Model,
    message_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(message) = context.messages.get(&message_id) else {
        return unavailable();
    };
    if !context.readable_channels.contains(&message.channel_id) {
        return unavailable();
    }

    let (thread_root_id, thread_root_kind) =
        match (message.thread_root_id, message.thread_poll_id) {
            (Some(root_id), _) => (Some(root_id.to_string()), Some("message")),
            (_, Some(poll_id)) => (Some(poll_id.to_string()), Some("poll")),
            _ => (None, None),
        };
    let forum_post_id = (row.kind == NotificationKind::ForumReply)
        .then(|| {
            message
                .thread_root_id
                .and_then(|root_id| context.forum_posts_by_root.get(&root_id))
                .map(|post_id| post_id.to_string())
        })
        .flatten();

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
    poll_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(poll) = context.polls.get(&poll_id) else {
        return unavailable();
    };
    if !context.readable_channels.contains(&poll.channel_id) {
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
    role_id: Uuid,
    context: &NotificationContext,
) -> NotificationTargetResponse {
    let Some(role) = context.server_roles.get(&role_id) else {
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
    viewer_id: Uuid,
    channel_ids: &[Uuid],
) -> AppResult<HashSet<Uuid>> {
    if channel_ids.is_empty() {
        return Ok(HashSet::new());
    }

    Ok(channel_members::Entity::find()
        .filter(channel_members::Column::UserId.eq(viewer_id))
        .filter(
            channel_members::Column::ChannelId
                .is_in(channel_ids.iter().copied()),
        )
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|member| member.channel_id)
        .collect())
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
