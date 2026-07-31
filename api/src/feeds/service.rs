use chrono::{DateTime, FixedOffset};
use sea_orm::prelude::Uuid;

use super::types::{
    FeedItem, FeedMessageResponse, FeedPollResponse,
    FeedProposalForumReferenceResponse,
};
use crate::{
    calls,
    common::{
        pagination::{PaginationCursor, PaginationDirection},
        AppResult,
    },
    forum, messages, polls,
};

pub(crate) async fn get_channel_feed(
    database: &sea_orm::DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    cursor: Option<PaginationCursor>,
    direction: PaginationDirection,
    limit: u64,
    user_id: Option<Uuid>,
) -> AppResult<Vec<FeedItem>> {
    let fetch_limit = limit.saturating_add(1);
    let messages = messages::get_channel_message_feed(
        database,
        server_id,
        channel_id,
        cursor,
        direction,
        fetch_limit,
    )
    .await?;
    let polls = polls::service::get_inline_polls(
        database,
        server_id,
        channel_id,
        cursor,
        direction,
        fetch_limit,
        user_id,
    )
    .await?;
    let calls = calls::service::get_channel_call_artifacts(
        database,
        server_id,
        channel_id,
        cursor,
        direction,
        fetch_limit,
    )
    .await?;
    let proposal_references =
        forum::proposal_moves::list_proposal_forum_references(
            database,
            channel_id,
            cursor,
            direction,
            fetch_limit,
        )
        .await?;

    let mut feed = messages
        .into_iter()
        .map(FeedMessageResponse::new)
        .map(FeedItem::Message)
        .collect();
    append_polls(&mut feed, polls);
    for reference in proposal_references {
        feed.push(FeedItem::ProposalForumReference(
            FeedProposalForumReferenceResponse::new(reference),
        ));
    }
    for call in calls {
        feed.push(FeedItem::Call(call));
    }

    Ok(sort_feed(feed, direction, fetch_limit))
}

pub(crate) async fn get_call_feed(
    database: &sea_orm::DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    cursor: Option<PaginationCursor>,
    direction: PaginationDirection,
    limit: u64,
) -> AppResult<Vec<FeedItem>> {
    let fetch_limit = limit.saturating_add(1);
    let messages = messages::get_call_message_feed(
        database,
        server_id,
        channel_id,
        call_id,
        cursor,
        direction,
        fetch_limit,
    )
    .await?;

    Ok(messages
        .into_iter()
        .map(FeedMessageResponse::new)
        .map(FeedItem::Message)
        .collect())
}

fn append_polls(
    feed: &mut Vec<FeedItem>,
    polls: Vec<polls::types::PollResponse>,
) {
    for poll in polls {
        feed.push(FeedItem::Poll(FeedPollResponse::new(poll)));
    }
}

fn sort_feed(
    mut feed: Vec<FeedItem>,
    direction: PaginationDirection,
    limit: u64,
) -> Vec<FeedItem> {
    feed.sort_by(|left, right| match direction {
        PaginationDirection::Older => timestamp_millis(right)
            .cmp(&timestamp_millis(left))
            .then_with(|| id_string(right).cmp(id_string(left))),
        PaginationDirection::Newer => timestamp_millis(left)
            .cmp(&timestamp_millis(right))
            .then_with(|| id_string(left).cmp(id_string(right))),
    });

    feed.into_iter().take(limit as usize).collect()
}

fn timestamp_millis(item: &FeedItem) -> i64 {
    DateTime::<FixedOffset>::parse_from_rfc3339(created_at(item))
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_default()
}

pub(super) fn created_at(item: &FeedItem) -> &str {
    match item {
        FeedItem::Message(message) => &message.message.created_at,
        FeedItem::Poll(poll) => &poll.poll.created_at,
        FeedItem::ProposalForumReference(reference) => {
            &reference.reference.created_at
        }
        FeedItem::Call(call) => &call.created_at,
    }
}

pub(super) fn id_string(item: &FeedItem) -> &str {
    match item {
        FeedItem::Message(message) => &message.message.id,
        FeedItem::Poll(poll) => &poll.poll.id,
        FeedItem::ProposalForumReference(reference) => &reference.reference.id,
        FeedItem::Call(call) => &call.id,
    }
}
