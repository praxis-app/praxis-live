use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service,
    types::{FeedQuery, FeedResponse},
};
use crate::{
    auth::{AuthenticatedUserOptional, HasJwtSecret},
    channels,
    common::AppResult,
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

pub(super) async fn get_channel_feed(
    State(feeds_state): State<FeedsState>,
    Path(path): Path<channels::types::ChannelPath>,
    Query(query): Query<FeedQuery>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Json<FeedResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let feed = service::get_channel_feed(
        &feeds_state.database,
        path.server_id,
        path.channel_id,
        offset,
        limit,
        user_id,
    )
    .await?;

    Ok(Json(FeedResponse { feed }))
}

pub(super) async fn get_call_feed(
    State(feeds_state): State<FeedsState>,
    Path(path): Path<crate::calls::types::CallPath>,
    Query(query): Query<FeedQuery>,
) -> AppResult<Json<FeedResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let feed = service::get_call_feed(
        &feeds_state.database,
        path.server_id,
        path.channel_id,
        path.call_id,
        offset,
        limit,
    )
    .await?;

    Ok(Json(FeedResponse { feed }))
}
