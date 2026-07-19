use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::{
    service,
    types::{
        CreateForumPostRequest, CreateForumReplyRequest, ForumChannelPath,
        ForumPostPath, ForumReplyPath, ListForumPostsQuery,
        UpdateForumPostRequest,
    },
};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::AppResult,
    polls::{self, types::CreatePollRequest},
    pub_sub::PubSubService,
};

#[derive(Clone, Debug)]
pub(super) struct ForumState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub_sub_service: PubSubService,
}

impl ForumState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
        pub_sub_service: PubSubService,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            pub_sub_service,
        }
    }
}

impl HasJwtSecret for ForumState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn list_forum_posts(
    State(state): State<ForumState>,
    Path(path): Path<ForumChannelPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(query): Query<ListForumPostsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let posts = service::list_forum_posts(
        &state.database,
        path.server_id,
        path.channel_id,
        user_id,
        query.sort.as_deref(),
        query.status.as_deref(),
        offset,
        limit,
    )
    .await?;
    Ok(Json(serde_json::json!({ "posts": posts })))
}

pub(super) async fn create_forum_post(
    State(state): State<ForumState>,
    Path(path): Path<ForumChannelPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateForumPostRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::create_forum_post(
        &state.database,
        path.server_id,
        path.channel_id,
        user_id,
        payload,
    )
    .await?;
    let proposal_id = post
        .proposal
        .as_ref()
        .and_then(|proposal| proposal.id.parse().ok());
    service::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "created",
        &post,
    )
    .await;
    if let Some(proposal_id) = proposal_id {
        if let Err(error) = polls::service::broadcast_poll_update(
            &state.database,
            &state.pub_sub_service,
            path.server_id,
            path.channel_id,
            Some(user_id),
            proposal_id,
        )
        .await
        {
            tracing::warn!("failed to broadcast forum proposal: {error}");
        }
    }
    Ok(Json(serde_json::json!({ "post": post })))
}

pub(super) async fn create_forum_post_proposal(
    State(state): State<ForumState>,
    Path(path): Path<ForumPostPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreatePollRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::create_forum_post_proposal(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        user_id,
        payload,
    )
    .await?;
    service::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "updated",
        &post,
    )
    .await;
    if let Some(proposal_id) = post
        .proposal
        .as_ref()
        .and_then(|proposal| proposal.id.parse().ok())
    {
        if let Err(error) = polls::service::broadcast_poll_update(
            &state.database,
            &state.pub_sub_service,
            path.server_id,
            path.channel_id,
            Some(user_id),
            proposal_id,
        )
        .await
        {
            tracing::warn!("failed to broadcast forum proposal: {error}");
        }
    }
    Ok(Json(serde_json::json!({ "post": post })))
}

pub(super) async fn get_forum_post(
    State(state): State<ForumState>,
    Path(path): Path<ForumPostPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::get_forum_post(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        user_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "post": post })))
}

pub(super) async fn update_forum_post(
    State(state): State<ForumState>,
    Path(path): Path<ForumPostPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdateForumPostRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::update_forum_post(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        user_id,
        payload,
    )
    .await?;
    service::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "updated",
        &post,
    )
    .await;
    Ok(Json(serde_json::json!({ "post": post })))
}

pub(super) async fn close_forum_post(
    State(state): State<ForumState>,
    Path(path): Path<ForumPostPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::close_forum_post(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        user_id,
    )
    .await?;
    service::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "closed",
        &post,
    )
    .await;
    Ok(Json(serde_json::json!({ "post": post })))
}

pub(super) async fn create_forum_reply(
    State(state): State<ForumState>,
    Path(path): Path<ForumPostPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateForumReplyRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let (reply, post) = service::create_forum_reply(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        user_id,
        payload,
    )
    .await?;
    service::broadcast_forum_reply(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "created",
        path.post_id,
        Some(&reply),
        None,
        &post,
    )
    .await;
    Ok(Json(serde_json::json!({ "reply": reply })))
}

pub(super) async fn delete_forum_reply(
    State(state): State<ForumState>,
    Path(path): Path<ForumReplyPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let post = service::delete_forum_reply(
        &state.database,
        path.server_id,
        path.channel_id,
        path.post_id,
        path.reply_id,
        user_id,
    )
    .await?;
    service::broadcast_forum_reply(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        "deleted",
        path.post_id,
        None,
        Some(path.reply_id),
        &post,
    )
    .await;
    Ok(Json(serde_json::json!({})))
}
