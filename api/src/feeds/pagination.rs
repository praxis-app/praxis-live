use axum::http::StatusCode;
use sea_orm::prelude::Uuid;

use super::{
    service,
    types::{FeedItem, FeedQuery, FeedResponse},
};
use crate::common::{
    pagination::{PaginationCursor, PaginationDirection},
    ApiError, AppResult,
};

pub(super) fn parse_cursor(
    query: &FeedQuery,
) -> AppResult<(Option<PaginationCursor>, PaginationDirection)> {
    match (&query.before, &query.after) {
        (Some(_), Some(_)) => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Use either before or after, not both.",
        )),
        (Some(cursor), None) => Ok((
            Some(PaginationCursor::parse(cursor)?),
            PaginationDirection::Older,
        )),
        (None, Some(cursor)) => Ok((
            Some(PaginationCursor::parse(cursor)?),
            PaginationDirection::Newer,
        )),
        (None, None) => Ok((None, PaginationDirection::Older)),
    }
}

pub(super) fn feed_response(
    mut feed: Vec<FeedItem>,
    limit: u64,
) -> FeedResponse {
    let has_more = feed.len() > limit as usize;
    if has_more {
        feed.pop();
    }
    let start_cursor = feed.first().and_then(item_cursor);
    let next_cursor = feed.last().and_then(item_cursor);

    FeedResponse {
        feed,
        start_cursor,
        next_cursor,
        has_more,
    }
}

fn item_cursor(item: &FeedItem) -> Option<String> {
    Some(
        PaginationCursor {
            created_at: chrono::DateTime::parse_from_rfc3339(
                service::created_at(item),
            )
            .ok()?,
            id: Uuid::parse_str(service::id_string(item)).ok()?,
        }
        .encode(),
    )
}
