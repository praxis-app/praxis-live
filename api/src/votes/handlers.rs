use axum::{extract::State, response::Json};

use super::{
    extractors::{
        ReadablePollOptionContext, VoteMutationContext, VoteRouteContext,
    },
    service,
    types::{UpdateVoteResponse, VotePayload, VoteRequest, VotersPayload},
};
use crate::{
    common::{response::EmptyResponse, AppResult},
    polls::{handlers::PollsState, service as polls_service},
};

pub(crate) async fn get_voters_by_poll_option(
    State(state): State<PollsState>,
    context: ReadablePollOptionContext,
) -> AppResult<Json<VotersPayload>> {
    let voters = service::get_voters_by_poll_option(
        &state.database,
        context.poll_id,
        context.poll_option_id,
    )
    .await?;

    Ok(Json(VotersPayload { voters }))
}

pub(crate) async fn create_vote(
    State(state): State<PollsState>,
    context: VoteRouteContext,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<VotePayload>> {
    let server_id = context.server_id;
    let channel_id = context.channel_id;
    let poll_id = context.poll_id;
    let user_id = context.user_id;
    let vote =
        service::create_vote(&state.database, context.poll, user_id, payload)
            .await?;

    if let Err(error) = polls_service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        server_id,
        channel_id,
        Some(user_id),
        poll_id,
    )
    .await
    {
        tracing::warn!("failed to broadcast vote update: {error}");
    }

    Ok(Json(VotePayload { vote }))
}

pub(crate) async fn update_vote(
    State(state): State<PollsState>,
    context: VoteMutationContext,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<UpdateVoteResponse>> {
    let server_id = context.route.server_id;
    let channel_id = context.route.channel_id;
    let poll_id = context.route.poll_id;
    let user_id = context.route.user_id;
    let response = service::update_vote(
        &state.database,
        context.route.poll,
        context.vote_id,
        user_id,
        payload,
    )
    .await?;

    if let Err(error) = polls_service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        server_id,
        channel_id,
        Some(user_id),
        poll_id,
    )
    .await
    {
        tracing::warn!("failed to broadcast vote update: {error}");
    }

    Ok(Json(response))
}

pub(crate) async fn delete_vote(
    State(state): State<PollsState>,
    context: VoteMutationContext,
) -> AppResult<Json<EmptyResponse>> {
    let server_id = context.route.server_id;
    let channel_id = context.route.channel_id;
    let poll_id = context.route.poll_id;
    let user_id = context.route.user_id;
    service::delete_vote(
        &state.database,
        &context.route.poll,
        context.vote_id,
        user_id,
    )
    .await?;

    if let Err(error) = polls_service::broadcast_poll_update(
        &state.database,
        &state.pub_sub_service,
        server_id,
        channel_id,
        Some(user_id),
        poll_id,
    )
    .await
    {
        tracing::warn!("failed to broadcast vote update: {error}");
    }

    Ok(Json(EmptyResponse {}))
}
