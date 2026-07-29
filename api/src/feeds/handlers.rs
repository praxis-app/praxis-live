use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    extractors::ChannelFeedAccessContext,
    pagination::{feed_response, parse_cursor},
    service,
    types::{FeedQuery, FeedResponse},
};
use crate::{
    auth::HasJwtSecret, channels::extractors::HasDatabase, common::AppResult,
};

#[derive(Clone, Debug)]
pub(super) struct FeedsState {
    pub(super) database: DatabaseConnection,
    jwt_secret: Arc<str>,
}

impl FeedsState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
        }
    }
}

impl HasJwtSecret for FeedsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

impl HasDatabase for FeedsState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

pub(super) async fn get_channel_feed(
    State(feeds_state): State<FeedsState>,
    context: ChannelFeedAccessContext,
    Query(query): Query<FeedQuery>,
) -> AppResult<Json<FeedResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let (cursor, direction) = parse_cursor(&query)?;
    let feed = service::get_channel_feed(
        &feeds_state.database,
        context.server_id,
        context.channel_id,
        cursor,
        direction,
        limit,
        context.user_id,
    )
    .await?;

    Ok(Json(feed_response(feed, limit)))
}

pub(super) async fn get_call_feed(
    State(feeds_state): State<FeedsState>,
    Path(path): Path<crate::calls::types::CallPath>,
    Query(query): Query<FeedQuery>,
) -> AppResult<Json<FeedResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let (cursor, direction) = parse_cursor(&query)?;
    let feed = service::get_call_feed(
        &feeds_state.database,
        path.server_id,
        path.channel_id,
        path.call_id,
        cursor,
        direction,
        limit,
    )
    .await?;

    Ok(Json(feed_response(feed, limit)))
}
