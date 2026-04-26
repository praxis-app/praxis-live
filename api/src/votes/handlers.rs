use axum::{extract::State, response::Json};

use super::{
    extractors::{
        ReadablePollOptionContext, VoteMutationContext, VoteRouteContext,
    },
    service,
    types::VoteRequest,
};
use crate::{common::AppResult, polls::handlers::PollsState};

pub(crate) async fn get_voters_by_poll_option(
    State(state): State<PollsState>,
    context: ReadablePollOptionContext,
) -> AppResult<Json<serde_json::Value>> {
    let voters = service::get_voters_by_poll_option(
        &state.database,
        context.poll_id,
        context.poll_option_id,
    )
    .await?;

    Ok(Json(serde_json::json!({ "voters": voters })))
}

pub(crate) async fn create_vote(
    State(state): State<PollsState>,
    context: VoteRouteContext,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let vote = service::create_vote(
        &state.database,
        context.poll,
        context.user_id,
        payload,
    )
    .await?;

    Ok(Json(serde_json::json!({ "vote": vote })))
}

pub(crate) async fn update_vote(
    State(state): State<PollsState>,
    context: VoteMutationContext,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let response = service::update_vote(
        &state.database,
        context.route.poll,
        context.vote_id,
        context.route.user_id,
        payload,
    )
    .await?;

    Ok(Json(serde_json::json!(response)))
}

pub(crate) async fn delete_vote(
    State(state): State<PollsState>,
    context: VoteMutationContext,
) -> AppResult<Json<serde_json::Value>> {
    service::delete_vote(
        &state.database,
        context.route.poll_id,
        context.vote_id,
        context.route.user_id,
    )
    .await?;

    Ok(Json(serde_json::json!({})))
}
