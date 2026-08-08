use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use std::{path::PathBuf, sync::Arc};

use super::{
    extractors::PollDeleteContext,
    service,
    types::{
        ActiveDecisionsResponse, CallDecisionResponse, CreatePollRequest,
        DeletePollResponse, ListActiveDecisionsQuery, PollImagePath, PollPath,
        PollPayload,
    },
};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    calls::extractors::CallWriteContext,
    channels::{self, extractors::ChannelWriteContext},
    common::{
        request::JsonOrEventCoverPhoto, storage::upload_root, ApiError,
        AppResult,
    },
    pub_sub::PubSubService,
    servers::types::ServerPath,
};

#[derive(Clone, Debug)]
pub(crate) struct PollsState {
    pub(crate) database: DatabaseConnection,
    jwt_secret: Arc<str>,
    pub(crate) pub_sub_service: PubSubService,
    upload_root: Arc<PathBuf>,
}

impl PollsState {
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

impl HasJwtSecret for PollsState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

impl channels::extractors::HasDatabase for PollsState {
    fn database(&self) -> &DatabaseConnection {
        &self.database
    }
}

pub(super) async fn create_poll(
    State(state): State<PollsState>,
    context: ChannelWriteContext,
    JsonOrEventCoverPhoto {
        payload,
        cover_photo,
    }: JsonOrEventCoverPhoto<CreatePollRequest>,
) -> AppResult<Json<PollPayload>> {
    let poll = service::create_poll(
        &state.database,
        &state.upload_root,
        context.server_id,
        context.channel_id,
        context.user_id,
        payload,
        cover_photo,
    )
    .await?;

    if let Err(error) = service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        Some(context.user_id),
        poll.id.parse().expect("created poll id is valid"),
    )
    .await
    {
        tracing::warn!("failed to broadcast created poll: {error}");
    }

    Ok(Json(PollPayload { poll }))
}

pub(super) async fn move_proposal_to_forum(
    State(state): State<PollsState>,
    Path(path): Path<PollPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<crate::forum::types::MoveProposalToForumRequest>,
) -> AppResult<Json<crate::forum::types::MoveProposalToForumResponse>> {
    let result = crate::forum::proposal_moves::move_proposal_to_forum(
        &state.database,
        path.server_id,
        path.channel_id,
        path.poll_id,
        user_id,
        payload,
    )
    .await?;
    let destination_channel_id = result.destination_channel_id;

    crate::forum::events::broadcast_proposal_forum_reference(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        path.channel_id,
        user_id,
        &result.source_reference,
    )
    .await;
    crate::forum::events::broadcast_forum_post(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        destination_channel_id,
        user_id,
        "created",
        &result.post,
    )
    .await;
    if let Err(error) = service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        path.server_id,
        destination_channel_id,
        Some(user_id),
        path.poll_id,
    )
    .await
    {
        tracing::warn!("failed to broadcast moved proposal: {error}");
    }

    Ok(Json(result))
}

pub(super) async fn create_call_poll(
    State(state): State<PollsState>,
    context: CallWriteContext,
    JsonOrEventCoverPhoto {
        payload,
        cover_photo,
    }: JsonOrEventCoverPhoto<CreatePollRequest>,
) -> AppResult<Json<PollPayload>> {
    let poll = service::create_call_poll(
        &state.database,
        &state.upload_root,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
        payload,
        cover_photo,
    )
    .await?;

    if let Err(error) = service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        context.server_id,
        context.channel_id,
        Some(context.user_id),
        poll.id.parse().expect("created poll id is valid"),
    )
    .await
    {
        tracing::warn!("failed to broadcast created in-call poll: {error}");
    }

    Ok(Json(PollPayload { poll }))
}

pub(super) async fn get_call_decision(
    State(state): State<PollsState>,
    context: CallWriteContext,
) -> AppResult<Json<CallDecisionResponse>> {
    let decision = service::get_call_decision(
        &state.database,
        context.server_id,
        context.channel_id,
        context.call_id,
        context.user_id,
    )
    .await?;

    Ok(Json(decision))
}

pub(super) async fn get_active_decisions(
    State(state): State<PollsState>,
    Path(path): Path<ServerPath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
    Query(query): Query<ListActiveDecisionsQuery>,
) -> AppResult<Json<ActiveDecisionsResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let decisions = service::get_active_decisions(
        &state.database,
        path.server_id,
        user_id,
        query.before.as_deref(),
        limit,
    )
    .await?;

    Ok(Json(decisions))
}

pub(super) async fn get_poll_action_event_cover_photo(
    State(state): State<PollsState>,
    Path(path): Path<PollImagePath>,
    AuthenticatedUserOptional(user_id): AuthenticatedUserOptional,
) -> AppResult<Response<Body>> {
    let image = service::get_poll_action_event_cover_photo(
        &state.database,
        &state.upload_root,
        path.server_id,
        path.channel_id,
        path.poll_id,
        path.image_id,
        user_id,
    )
    .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            image
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .header("x-content-type-options", "nosniff")
        .body(Body::from(image.bytes))
        .map_err(internal_error)
}

pub(super) async fn get_poll_image(
    State(state): State<PollsState>,
    Path(path): Path<PollImagePath>,
) -> AppResult<Response<Body>> {
    let image = service::get_poll_image(
        &state.database,
        &state.upload_root,
        path.server_id,
        path.channel_id,
        path.poll_id,
        path.image_id,
    )
    .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            image
                .content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .body(Body::from(image.bytes))
        .map_err(internal_error)
}

pub(super) async fn delete_poll(
    State(state): State<PollsState>,
    context: PollDeleteContext,
) -> AppResult<Json<DeletePollResponse>> {
    let result = service::delete_poll(
        &state.database,
        &state.upload_root,
        &context.poll,
    )
    .await?;

    Ok(Json(DeletePollResponse {
        affected: result.rows_affected,
    }))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
