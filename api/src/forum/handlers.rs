use axum::{
    extract::{Query, State},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

use super::{
    events,
    extractors::{
        ForumAccessContext, ForumPostAccessContext, ForumPostReadContext,
        ForumReadContext, ForumReplyAccessContext,
    },
    service,
    types::{
        CreateForumPostRequest, CreateForumReplyRequest, ForumPostPayload,
        ForumPostsResponse, ForumReplyPayload, ListForumPostsQuery,
        UpdateForumPostRequest,
    },
};
use crate::{
    auth::HasJwtSecret,
    channels::extractors::HasDatabase,
    common::{
        request::JsonOrMultipartFile, response::EmptyResponse,
        storage::upload_root, AppResult,
    },
    polls::{self, types::CreatePollRequest},
    pub_sub::PubSubService,
};

#[derive(Clone, Debug)]
pub(super) struct ForumState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub_sub_service: PubSubService,
    upload_root: Arc<PathBuf>,
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
            upload_root: Arc::new(upload_root()),
        }
    }
}

impl HasJwtSecret for ForumState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

impl HasDatabase for ForumState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

pub(super) async fn list_forum_posts(
    State(state): State<ForumState>,
    context: ForumReadContext,
    Query(query): Query<ListForumPostsQuery>,
) -> AppResult<Json<ForumPostsResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let posts = service::list_forum_posts(
        &state.database,
        context.channel_id,
        query.sort.as_deref(),
        query.status.as_deref(),
        query.before.as_deref(),
        limit,
    )
    .await?;
    Ok(Json(posts))
}

pub(super) async fn create_forum_post(
    State(state): State<ForumState>,
    context: ForumAccessContext,
    JsonOrMultipartFile { payload, file }: JsonOrMultipartFile<
        CreateForumPostRequest,
    >,
) -> AppResult<Json<ForumPostPayload>> {
    let cover_photo = file.map(|file| file.bytes);
    let post = service::create_forum_post(
        &state.database,
        &state.upload_root,
        context.server_id,
        context.channel_id,
        context.user_id,
        payload,
        cover_photo,
    )
    .await?;
    let proposal_id = post
        .proposal
        .as_ref()
        .and_then(|proposal| proposal.id.parse().ok());
    events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "created",
        &post,
    )
    .await;
    if let Some(proposal_id) = proposal_id {
        if let Err(error) = polls::service::broadcast_poll_update(
            &state.database,
            &state.pub_sub_service,
            context.server_id,
            context.channel_id,
            Some(context.user_id),
            proposal_id,
        )
        .await
        {
            tracing::warn!("failed to broadcast forum proposal: {error}");
        }
    }
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn create_forum_post_proposal(
    State(state): State<ForumState>,
    context: ForumPostAccessContext,
    JsonOrMultipartFile { payload, file }: JsonOrMultipartFile<
        CreatePollRequest,
    >,
) -> AppResult<Json<ForumPostPayload>> {
    let cover_photo = file.map(|file| file.bytes);
    let post = service::create_forum_post_proposal(
        &state.database,
        &state.upload_root,
        context.server_id,
        context.channel_id,
        context.post_id,
        context.user_id,
        payload,
        cover_photo,
    )
    .await?;
    events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
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
            context.server_id,
            context.channel_id,
            Some(context.user_id),
            proposal_id,
        )
        .await
        {
            tracing::warn!("failed to broadcast forum proposal: {error}");
        }
    }
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn get_forum_post(
    State(state): State<ForumState>,
    context: ForumPostReadContext,
) -> AppResult<Json<ForumPostPayload>> {
    let post = service::get_forum_post(
        &state.database,
        context.channel_id,
        context.post_id,
        context.user_id,
    )
    .await?;
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn update_forum_post(
    State(state): State<ForumState>,
    context: ForumPostAccessContext,
    Json(payload): Json<UpdateForumPostRequest>,
) -> AppResult<Json<ForumPostPayload>> {
    let post = service::update_forum_post(
        &state.database,
        context.channel_id,
        context.post_id,
        context.user_id,
        payload,
    )
    .await?;
    events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "updated",
        &post,
    )
    .await;
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn close_forum_post(
    State(state): State<ForumState>,
    context: ForumPostAccessContext,
) -> AppResult<Json<ForumPostPayload>> {
    let post = service::close_forum_post(
        &state.database,
        context.channel_id,
        context.post_id,
        context.user_id,
    )
    .await?;
    events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "closed",
        &post,
    )
    .await;
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn reopen_forum_post(
    State(state): State<ForumState>,
    context: ForumPostAccessContext,
) -> AppResult<Json<ForumPostPayload>> {
    let post = service::reopen_forum_post(
        &state.database,
        context.channel_id,
        context.post_id,
        context.user_id,
    )
    .await?;
    events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "reopened",
        &post,
    )
    .await;
    Ok(Json(ForumPostPayload { post }))
}

pub(super) async fn create_forum_reply(
    State(state): State<ForumState>,
    context: ForumPostAccessContext,
    Json(payload): Json<CreateForumReplyRequest>,
) -> AppResult<Json<ForumReplyPayload>> {
    let (reply, post) = service::create_forum_reply(
        &state.database,
        context.channel_id,
        context.post_id,
        context.user_id,
        payload,
    )
    .await?;
    events::broadcast_forum_reply(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "created",
        context.post_id,
        Some(&reply),
        None,
        &post,
    )
    .await;
    Ok(Json(ForumReplyPayload { reply }))
}

pub(super) async fn delete_forum_reply(
    State(state): State<ForumState>,
    context: ForumReplyAccessContext,
) -> AppResult<Json<EmptyResponse>> {
    let post = service::delete_forum_reply(
        &state.database,
        context.channel_id,
        context.post_id,
        context.reply_id,
        context.user_id,
    )
    .await?;
    events::broadcast_forum_reply(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        context.user_id,
        "deleted",
        context.post_id,
        None,
        Some(context.reply_id),
        &post,
    )
    .await;
    Ok(Json(EmptyResponse {}))
}
