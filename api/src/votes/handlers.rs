use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;

use super::{service, types::VoteRequest};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional},
    common::{request::parse_uuid, AppResult},
    polls::handlers::PollsState,
};

#[derive(Debug, Deserialize)]
pub(crate) struct PollPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VotePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
    #[serde(rename = "voteId")]
    vote_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PollOptionPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
    #[serde(rename = "pollOptionId")]
    poll_option_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollOptionQuery {
    invite_token: Option<String>,
}

pub(crate) async fn get_voters_by_poll_option(
    State(state): State<PollsState>,
    Path(path): Path<PollOptionPath>,
    Query(query): Query<PollOptionQuery>,
    AuthenticatedUserOptional(current_user_id): AuthenticatedUserOptional,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let poll_option_id = parse_uuid(&path.poll_option_id, "pollOptionId")?;

    let voters = service::get_voters_by_poll_option(
        &state.database,
        server_id,
        channel_id,
        poll_id,
        poll_option_id,
        current_user_id,
        query.invite_token.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "voters": voters })))
}

pub(crate) async fn create_vote(
    State(state): State<PollsState>,
    Path(path): Path<PollPath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let vote = service::create_vote(
        &state.database,
        server_id,
        channel_id,
        poll_id,
        user_id,
        payload,
    )
    .await?;

    Ok(Json(serde_json::json!({ "vote": vote })))
}

pub(crate) async fn update_vote(
    State(state): State<PollsState>,
    Path(path): Path<VotePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<VoteRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let vote_id = parse_uuid(&path.vote_id, "voteId")?;
    let response = service::update_vote(
        &state.database,
        server_id,
        channel_id,
        poll_id,
        vote_id,
        user_id,
        payload,
    )
    .await?;

    Ok(Json(serde_json::json!(response)))
}

pub(crate) async fn delete_vote(
    State(state): State<PollsState>,
    Path(path): Path<VotePath>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let server_id = parse_uuid(&path.server_id, "serverId")?;
    let channel_id = parse_uuid(&path.channel_id, "channelId")?;
    let poll_id = parse_uuid(&path.poll_id, "pollId")?;
    let vote_id = parse_uuid(&path.vote_id, "voteId")?;
    service::delete_vote(
        &state.database,
        server_id,
        channel_id,
        poll_id,
        vote_id,
        user_id,
    )
    .await?;

    Ok(Json(serde_json::json!({})))
}
