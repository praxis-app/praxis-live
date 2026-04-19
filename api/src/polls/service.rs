use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{
    poll_configs, poll_images, poll_option_selections, poll_options, polls,
    users, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::path::{Path, PathBuf};
use uuid::Uuid as NativeUuid;

use super::types::{
    CreatePollRequest, CreateVoteResponse, PollConfigResponse,
    PollImageResponse, PollOptionResponse, PollOptionVoterResponse,
    PollResponse, PollUserResponse, StoredPollImage, UpdateVoteResponse,
    VoteRequest, VoteResponse,
};
use crate::{
    channels,
    common::{request::parse_uuid, ApiError, AppResult},
    messages::types::serialize_timestamp,
    poll_actions::{self, types::CreatePollActionRequest},
    servers::server_configs,
    users as users_service,
};

const MAX_IMAGE_COUNT: usize = 8;
const MAX_POLL_BODY_LENGTH: usize = 8_000;

pub(crate) async fn create_poll(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    request: CreatePollRequest,
) -> AppResult<PollResponse> {
    validate_create_poll(&request)?;
    channels::get_channel(database, server_id, channel_id).await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;

    let body = request
        .body
        .as_deref()
        .map(sanitize_text)
        .filter(|value| !value.is_empty());
    let is_proposal = request.poll_type == "proposal";
    let server_config =
        server_configs::service::ensure_server_config(database, server_id)
            .await?;
    let closing_at = request.closing_at.or_else(|| {
        (server_config.voting_time_limit > 0).then(|| {
            (Utc::now()
                + Duration::minutes(server_config.voting_time_limit as i64))
            .fixed_offset()
        })
    });

    let poll = polls::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        body: Set(body.clone()),
        poll_type: Set(request.poll_type.clone()),
        user_id: Set(user_id),
        channel_id: Set(channel_id),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    poll_configs::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll.id),
        decision_making_model: Set(is_proposal
            .then_some(server_config.decision_making_model)),
        disagreements_limit: Set(is_proposal
            .then_some(server_config.disagreements_limit)),
        abstains_limit: Set(is_proposal.then_some(server_config.abstains_limit)),
        agreement_threshold: Set(is_proposal
            .then_some(server_config.agreement_threshold)),
        quorum_enabled: Set(is_proposal.then_some(server_config.quorum_enabled)),
        quorum_threshold: Set(is_proposal.then_some(server_config.quorum_threshold)),
        multiple_choice: Set((!is_proposal)
            .then_some(request.multiple_choice.unwrap_or(false))),
        closing_at: Set(closing_at),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    if is_proposal {
        if let Some(action) = request.action {
            poll_actions::service::create_poll_action(
                database, poll.id, action,
            )
            .await?;
        }
    } else if let Some(options) = request.options {
        for option in options {
            poll_options::ActiveModel {
                id: Set(NativeUuid::new_v4()),
                poll_id: Set(poll.id),
                text: Set(sanitize_text(&option)),
                ..Default::default()
            }
            .insert(database)
            .await
            .map_err(internal_error)?;
        }
    }

    for _ in 0..request.image_count {
        poll_images::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            poll_id: Set(poll.id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    shape_poll(database, poll, Some(user_id)).await
}

pub(crate) async fn get_inline_polls(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    offset: u64,
    limit: u64,
    current_user_id: Option<Uuid>,
) -> AppResult<Vec<PollResponse>> {
    channels::get_channel(database, server_id, channel_id).await?;
    let polls = polls::Entity::find()
        .filter(polls::Column::ChannelId.eq(channel_id))
        .order_by_desc(polls::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut responses = Vec::with_capacity(polls.len());
    for poll in polls {
        responses.push(shape_poll(database, poll, current_user_id).await?);
    }
    Ok(responses)
}

pub(crate) async fn create_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<CreateVoteResponse> {
    let poll = validate_vote_request(
        database, server_id, channel_id, poll_id, &request,
    )
    .await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;

    if votes::Entity::find()
        .filter(votes::Column::PollId.eq(poll_id))
        .filter(votes::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "You have already voted on this poll.",
        ));
    }

    let poll_option_ids = parse_poll_option_ids(&request)?;
    validate_poll_option_ids(database, poll_id, &poll_option_ids).await?;
    let vote = votes::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll_id),
        user_id: Set(user_id),
        vote_type: Set(request.vote_type.clone()),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    save_poll_option_selections(database, vote.id, &poll_option_ids).await?;
    let is_ratifying_vote =
        synchronize_ratification_after_vote(database, &poll).await?;

    Ok(CreateVoteResponse {
        id: vote.id.to_string(),
        poll_id: vote.poll_id.to_string(),
        user_id: vote.user_id.to_string(),
        vote_type: vote.vote_type,
        poll_option_ids: (!poll_option_ids.is_empty())
            .then(|| poll_option_ids.iter().map(ToString::to_string).collect()),
        is_ratifying_vote,
    })
}

pub(crate) async fn update_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    vote_id: Uuid,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<UpdateVoteResponse> {
    let poll = validate_vote_request(
        database, server_id, channel_id, poll_id, &request,
    )
    .await?;
    let vote = votes::Entity::find_by_id(vote_id)
        .filter(votes::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Vote not found.")
        })?;
    if vote.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }

    let poll_option_ids = parse_poll_option_ids(&request)?;
    validate_poll_option_ids(database, poll_id, &poll_option_ids).await?;
    let mut active = vote.into_active_model();
    active.vote_type = Set(request.vote_type);
    active.update(database).await.map_err(internal_error)?;

    poll_option_selections::Entity::delete_many()
        .filter(poll_option_selections::Column::VoteId.eq(vote_id))
        .exec(database)
        .await
        .map_err(internal_error)?;
    save_poll_option_selections(database, vote_id, &poll_option_ids).await?;

    let is_ratifying_vote =
        synchronize_ratification_after_vote(database, &poll).await?;
    Ok(UpdateVoteResponse { is_ratifying_vote })
}

pub(crate) async fn delete_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    vote_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let poll = load_poll(database, server_id, channel_id, poll_id).await?;
    if poll.stage != "voting" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll is no longer accepting votes.",
        ));
    }

    let vote = votes::Entity::find_by_id(vote_id)
        .filter(votes::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Vote not found.")
        })?;
    if vote.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }
    votes::Entity::delete_by_id(vote_id)
        .exec(database)
        .await
        .map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn get_voters_by_poll_option(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    poll_option_id: Uuid,
) -> AppResult<Vec<PollOptionVoterResponse>> {
    load_poll(database, server_id, channel_id, poll_id).await?;
    let option = poll_options::Entity::find_by_id(poll_option_id)
        .filter(poll_options::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll option not found.")
        })?;

    let selections = poll_option_selections::Entity::find()
        .filter(poll_option_selections::Column::PollOptionId.eq(option.id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let vote_ids: Vec<Uuid> = selections
        .iter()
        .map(|selection| selection.vote_id)
        .collect();
    if vote_ids.is_empty() {
        return Ok(vec![]);
    }

    let votes = votes::Entity::find()
        .filter(votes::Column::Id.is_in(vote_ids))
        .all(database)
        .await
        .map_err(internal_error)?;
    let user_ids: Vec<Uuid> = votes.iter().map(|vote| vote.user_id).collect();
    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.clone()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;

    Ok(users
        .into_iter()
        .map(|user| PollOptionVoterResponse {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture: profile_pictures.get(&user.id).cloned(),
        })
        .collect())
}

pub(crate) async fn store_poll_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    image_id: Uuid,
    user_id: Uuid,
    content_type: Option<String>,
    bytes: Vec<u8>,
) -> AppResult<PollImageResponse> {
    if bytes.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No image uploaded",
        ));
    }

    let poll = load_poll(database, server_id, channel_id, poll_id).await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;
    if poll.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }

    let image = poll_images::Entity::find_by_id(image_id)
        .filter(poll_images::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;

    let storage_key = format!("poll-images/{image_id}");
    let destination = upload_root.join(&storage_key);
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_error)?;
    }
    tokio::fs::write(&destination, bytes)
        .await
        .map_err(internal_error)?;

    let mut active = image.into_active_model();
    active.storage_key = Set(Some(storage_key));
    active.content_type = Set(content_type);
    let image = active.update(database).await.map_err(internal_error)?;
    Ok(shape_poll_image(&image))
}

pub(crate) async fn get_poll_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    image_id: Uuid,
) -> AppResult<StoredPollImage> {
    load_poll(database, server_id, channel_id, poll_id).await?;
    let image = poll_images::Entity::find_by_id(image_id)
        .filter(poll_images::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;
    let storage_key = image.storage_key.ok_or_else(|| {
        ApiError::new(StatusCode::NOT_FOUND, "Image file not found.")
    })?;
    let bytes = tokio::fs::read(upload_root.join(storage_key))
        .await
        .map_err(|_| {
            ApiError::new(StatusCode::NOT_FOUND, "Image file not found.")
        })?;
    Ok(StoredPollImage {
        content_type: image.content_type,
        bytes,
    })
}

pub(crate) fn upload_root() -> PathBuf {
    std::env::var("UPLOAD_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".uploads"))
}

async fn validate_vote_request(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    request: &VoteRequest,
) -> AppResult<polls::Model> {
    let poll = load_poll(database, server_id, channel_id, poll_id).await?;
    if poll.stage != "voting" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll is no longer accepting votes.",
        ));
    }
    if poll.poll_type == "proposal" {
        validate_vote_type(request.vote_type.as_deref())?;
    } else if request
        .poll_option_ids
        .as_ref()
        .map(Vec::is_empty)
        .unwrap_or(true)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "At least one poll option must be selected.",
        ));
    }
    Ok(poll)
}

async fn save_poll_option_selections(
    database: &DatabaseConnection,
    vote_id: Uuid,
    poll_option_ids: &[Uuid],
) -> AppResult<()> {
    for poll_option_id in poll_option_ids {
        poll_option_selections::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            vote_id: Set(vote_id),
            poll_option_id: Set(*poll_option_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }
    Ok(())
}

async fn synchronize_ratification_after_vote(
    database: &DatabaseConnection,
    poll: &polls::Model,
) -> AppResult<bool> {
    if poll.poll_type != "proposal"
        || !is_poll_ratifiable(database, poll.id).await?
    {
        return Ok(false);
    }
    ratify_poll(database, poll.id).await?;
    poll_actions::service::implement_poll_action(database, poll.id).await?;
    Ok(true)
}

pub(crate) async fn is_poll_ratifiable(
    database: &DatabaseConnection,
    poll_id: Uuid,
) -> AppResult<bool> {
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    if poll.stage != "voting" {
        return Ok(false);
    }
    let config = poll_configs::Entity::find()
        .filter(poll_configs::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Poll config not found.",
            )
        })?;
    let votes = votes::Entity::find()
        .filter(votes::Column::PollId.eq(poll_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let member_count = get_poll_member_count(database, poll_id).await?;

    match config.decision_making_model.as_deref() {
        Some("consensus") => has_consensus(&votes, &config, member_count),
        Some("consent") => has_consent(&votes, &config),
        Some("majority-vote") => {
            has_majority_vote(&votes, &config, member_count)
        }
        _ => Ok(false),
    }
}

pub(crate) async fn ratify_poll(
    database: &DatabaseConnection,
    poll_id: Uuid,
) -> AppResult<()> {
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    let mut active = poll.into_active_model();
    active.stage = Set("ratified".to_owned());
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

async fn load_poll(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
) -> AppResult<polls::Model> {
    if server_id != Uuid::nil() {
        channels::get_channel(database, server_id, channel_id).await?;
    }
    polls::Entity::find_by_id(poll_id)
        .filter(polls::Column::ChannelId.eq(channel_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Poll not found."))
}

async fn shape_poll(
    database: &DatabaseConnection,
    poll: polls::Model,
    current_user_id: Option<Uuid>,
) -> AppResult<PollResponse> {
    let user = users::Entity::find_by_id(poll.user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "User not found.")
        })?;
    let profile_picture =
        users_service::get_user_profile_picture(database, poll.user_id).await?;
    let config = poll_configs::Entity::find()
        .filter(poll_configs::Column::PollId.eq(poll.id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Poll config not found.",
            )
        })?;
    let votes = votes::Entity::find()
        .filter(votes::Column::PollId.eq(poll.id))
        .order_by_asc(votes::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let vote_ids: Vec<Uuid> = votes.iter().map(|vote| vote.id).collect();
    let selections = if vote_ids.is_empty() {
        vec![]
    } else {
        poll_option_selections::Entity::find()
            .filter(poll_option_selections::Column::VoteId.is_in(vote_ids))
            .all(database)
            .await
            .map_err(internal_error)?
    };
    let options = poll_options::Entity::find()
        .filter(poll_options::Column::PollId.eq(poll.id))
        .order_by_asc(poll_options::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let images = poll_images::Entity::find()
        .filter(poll_images::Column::PollId.eq(poll.id))
        .order_by_asc(poll_images::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    let shaped_votes = votes
        .iter()
        .map(|vote| shape_vote(vote, &selections))
        .collect::<Vec<_>>();
    let my_vote = current_user_id.and_then(|user_id| {
        votes
            .iter()
            .find(|vote| vote.user_id == user_id)
            .map(|vote| shape_vote(vote, &selections))
    });

    Ok(PollResponse {
        id: poll.id.to_string(),
        body: poll.body,
        poll_type: poll.poll_type.clone(),
        stage: poll.stage,
        action: if poll.poll_type == "proposal" {
            poll_actions::service::shape_poll_action(database, poll.id).await?
        } else {
            None
        },
        config: shape_poll_config(config),
        options: options
            .into_iter()
            .map(|option| PollOptionResponse {
                id: option.id.to_string(),
                vote_count: selections
                    .iter()
                    .filter(|selection| selection.poll_option_id == option.id)
                    .count(),
                text: option.text,
            })
            .collect(),
        images: images.iter().map(shape_poll_image).collect(),
        user: PollUserResponse {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture,
        },
        agreement_vote_count: votes
            .iter()
            .filter(|vote| vote.vote_type.as_deref() == Some("agree"))
            .count(),
        votes: shaped_votes,
        my_vote,
        member_count: get_poll_member_count(database, poll.id).await?,
        created_at: serialize_timestamp(poll.created_at),
    })
}

fn shape_poll_config(config: poll_configs::Model) -> PollConfigResponse {
    PollConfigResponse {
        decision_making_model: config.decision_making_model,
        agreement_threshold: config.agreement_threshold,
        quorum_enabled: config.quorum_enabled,
        quorum_threshold: config.quorum_threshold,
        disagreements_limit: config.disagreements_limit,
        abstains_limit: config.abstains_limit,
        closing_at: config.closing_at.map(serialize_timestamp),
        multiple_choice: config.multiple_choice,
    }
}

fn shape_poll_image(image: &poll_images::Model) -> PollImageResponse {
    PollImageResponse {
        id: image.id.to_string(),
        is_placeholder: image.storage_key.is_none(),
        created_at: serialize_timestamp(image.created_at),
    }
}

fn shape_vote(
    vote: &votes::Model,
    selections: &[poll_option_selections::Model],
) -> VoteResponse {
    let poll_option_ids = selections
        .iter()
        .filter(|selection| selection.vote_id == vote.id)
        .map(|selection| selection.poll_option_id.to_string())
        .collect::<Vec<_>>();
    VoteResponse {
        id: vote.id.to_string(),
        vote_type: vote.vote_type.clone(),
        poll_option_ids: (!poll_option_ids.is_empty())
            .then_some(poll_option_ids),
    }
}

async fn get_poll_member_count(
    database: &DatabaseConnection,
    poll_id: Uuid,
) -> AppResult<usize> {
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    channels::get_channel_member_user_ids(database, poll.channel_id)
        .await
        .map(|members| members.len())
}

fn has_consensus(
    votes: &[votes::Model],
    config: &poll_configs::Model,
    member_count: usize,
) -> AppResult<bool> {
    if config
        .closing_at
        .map(|closing_at| Utc::now().fixed_offset() < closing_at)
        .unwrap_or(false)
    {
        return Ok(false);
    }
    if quorum_missing(votes, config, member_count)? {
        return Ok(false);
    }
    let (agreements, disagreements, abstains, blocks) = count_votes(votes);
    let participants = agreements + disagreements;
    Ok(participants > 0
        && agreements
            >= get_required_count(
                participants,
                required(config.agreement_threshold)?,
            )
        && disagreements <= required(config.disagreements_limit)? as usize
        && abstains <= required(config.abstains_limit)? as usize
        && blocks == 0)
}

fn has_majority_vote(
    votes: &[votes::Model],
    config: &poll_configs::Model,
    member_count: usize,
) -> AppResult<bool> {
    if config
        .closing_at
        .map(|closing_at| Utc::now().fixed_offset() < closing_at)
        .unwrap_or(false)
    {
        return Ok(false);
    }
    if quorum_missing(votes, config, member_count)? {
        return Ok(false);
    }
    let (agreements, disagreements, _, _) = count_votes(votes);
    let participants = agreements + disagreements;
    Ok(participants > 0
        && agreements
            >= get_required_count(
                participants,
                required(config.agreement_threshold)?,
            ))
}

fn has_consent(
    votes: &[votes::Model],
    config: &poll_configs::Model,
) -> AppResult<bool> {
    if config
        .closing_at
        .map(|closing_at| Utc::now().fixed_offset() < closing_at)
        .unwrap_or(true)
    {
        return Ok(false);
    }
    let (_, disagreements, abstains, blocks) = count_votes(votes);
    Ok(
        disagreements <= required(config.disagreements_limit)? as usize
            && abstains <= required(config.abstains_limit)? as usize
            && blocks == 0,
    )
}

fn quorum_missing(
    votes: &[votes::Model],
    config: &poll_configs::Model,
    member_count: usize,
) -> AppResult<bool> {
    if config.quorum_enabled.unwrap_or(false) {
        let threshold = required(config.quorum_threshold)?;
        return Ok(votes.len() < get_required_count(member_count, threshold));
    }
    Ok(false)
}

fn count_votes(votes: &[votes::Model]) -> (usize, usize, usize, usize) {
    let mut agreements = 0;
    let mut disagreements = 0;
    let mut abstains = 0;
    let mut blocks = 0;
    for vote in votes {
        match vote.vote_type.as_deref() {
            Some("agree") => agreements += 1,
            Some("disagree") => disagreements += 1,
            Some("abstain") => abstains += 1,
            Some("block") => blocks += 1,
            _ => {}
        }
    }
    (agreements, disagreements, abstains, blocks)
}

fn required(value: Option<i32>) -> AppResult<i32> {
    value.ok_or_else(|| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Missing poll configuration.",
        )
    })
}

fn get_required_count(member_count: usize, threshold: i32) -> usize {
    ((member_count as f64) * (threshold as f64 * 0.01)).ceil() as usize
}

fn validate_create_poll(request: &CreatePollRequest) -> AppResult<()> {
    if !matches!(request.poll_type.as_str(), "proposal" | "poll") {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll type is invalid.",
        ));
    }
    if request.image_count > MAX_IMAGE_COUNT {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Too many images.",
        ));
    }
    if request
        .body
        .as_ref()
        .map(|body| body.chars().count() > MAX_POLL_BODY_LENGTH)
        .unwrap_or(false)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Polls must be 8000 characters or less.",
        ));
    }

    if request.poll_type == "poll" {
        let body_missing = request
            .body
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);
        if body_missing {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls must include a question.",
            ));
        }
        let options = request.options.as_deref().unwrap_or(&[]);
        if options.len() < 2 {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls must have at least 2 options.",
            ));
        }
        if options.len() > 10 {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls cannot have more than 10 options.",
            ));
        }
        if options.iter().any(|option| option.chars().count() > 200) {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Poll options must be 200 characters or less.",
            ));
        }
    } else {
        validate_action(request.action.as_ref(), request.body.as_deref())?;
    }

    Ok(())
}

fn validate_action(
    action: Option<&CreatePollActionRequest>,
    body: Option<&str>,
) -> AppResult<()> {
    let action = action.ok_or_else(|| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Action is required.")
    })?;
    if !matches!(
        action.action_type.as_str(),
        "general"
            | "change-settings"
            | "change-role"
            | "create-role"
            | "plan-event"
            | "test"
    ) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Action type is invalid.",
        ));
    }
    if matches!(action.action_type.as_str(), "general" | "test")
        && body.map(str::trim).map(str::is_empty).unwrap_or(true)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Polls with this action must include a body.",
        ));
    }
    if action.action_type == "change-role" && action.server_role.is_none() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Polls to change server roles must include a server role.",
        ));
    }
    if action.action_type == "change-role" {
        let role = action.server_role.as_ref().expect("checked above");
        let has_change = role.name.is_some()
            || role.color.is_some()
            || role
                .members
                .as_ref()
                .map(|members| !members.is_empty())
                .unwrap_or(false)
            || role
                .permissions
                .as_ref()
                .map(|permissions| !permissions.is_empty())
                .unwrap_or(false);
        if !has_change {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls to change server roles must include at least 1 change.",
            ));
        }
    }
    Ok(())
}

fn validate_vote_type(vote_type: Option<&str>) -> AppResult<()> {
    if matches!(vote_type, Some("agree" | "disagree" | "abstain" | "block")) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid vote type.",
        ))
    }
}

fn parse_poll_option_ids(request: &VoteRequest) -> AppResult<Vec<Uuid>> {
    request
        .poll_option_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|id| parse_uuid(id, "pollOptionId"))
        .collect()
}

async fn validate_poll_option_ids(
    database: &DatabaseConnection,
    poll_id: Uuid,
    poll_option_ids: &[Uuid],
) -> AppResult<()> {
    if poll_option_ids.is_empty() {
        return Ok(());
    }

    let count = poll_options::Entity::find()
        .filter(poll_options::Column::PollId.eq(poll_id))
        .filter(poll_options::Column::Id.is_in(poll_option_ids.to_vec()))
        .count(database)
        .await
        .map_err(internal_error)?;

    if count as usize == poll_option_ids.len() {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll option is invalid.",
        ))
    }
}

fn sanitize_text(value: &str) -> String {
    value.trim().to_owned()
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
