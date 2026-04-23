use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{
    poll_configs, poll_images, poll_option_selections, poll_options, polls,
    users, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::time::{self, MissedTickBehavior};
use uuid::Uuid as NativeUuid;

use super::types::{
    CreatePollRequest, PollConfigResponse, PollImageResponse,
    PollOptionResponse, PollResponse, PollUserResponse, StoredPollImage,
};
use crate::{
    channels,
    common::{ApiError, AppResult},
    messages::types::serialize_timestamp,
    poll_actions::{self, types::CreatePollActionRequest},
    servers::{self, server_configs},
    users as users_service,
    votes::service as vote_service,
};

const MAX_IMAGE_COUNT: usize = 8;
const MAX_POLL_BODY_LENGTH: usize = 8_000;
const PROPOSAL_SYNC_BATCH_SIZE: usize = 20;
const PROPOSAL_SYNC_INTERVAL_SECONDS: u64 = 60 * 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ProposalSyncSummary {
    processed: usize,
    ratified: usize,
    closed: usize,
    failed: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProposalSyncAction {
    None,
    Ratify,
    Close,
}

pub(crate) fn spawn_proposal_synchronizer(database: DatabaseConnection) {
    tokio::spawn(async move {
        let mut interval = time::interval(std::time::Duration::from_secs(
            PROPOSAL_SYNC_INTERVAL_SECONDS,
        ));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            match synchronize_proposals(&database).await {
                Ok(summary) if summary.processed > 0 => {
                    tracing::info!(
                        processed = summary.processed,
                        ratified = summary.ratified,
                        closed = summary.closed,
                        failed = summary.failed,
                        "Synchronized proposals."
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("Failed to synchronize proposals: {error}");
                }
            }
        }
    });
}

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

async fn synchronize_proposals(
    database: &DatabaseConnection,
) -> AppResult<ProposalSyncSummary> {
    let configs = poll_configs::Entity::find()
        .filter(poll_configs::Column::ClosingAt.is_not_null())
        .all(database)
        .await
        .map_err(internal_error)?;
    if configs.is_empty() {
        return Ok(ProposalSyncSummary::default());
    }

    let configs_by_poll_id = configs
        .into_iter()
        .map(|config| (config.poll_id, config))
        .collect::<HashMap<_, _>>();
    let poll_ids = configs_by_poll_id.keys().copied().collect::<Vec<_>>();
    let proposals = polls::Entity::find()
        .filter(polls::Column::Id.is_in(poll_ids))
        .filter(polls::Column::PollType.eq("proposal"))
        .filter(polls::Column::Stage.eq("voting"))
        .all(database)
        .await
        .map_err(internal_error)?;
    if proposals.is_empty() {
        return Ok(ProposalSyncSummary::default());
    }

    let mut summary = ProposalSyncSummary::default();
    for batch in proposals.chunks(PROPOSAL_SYNC_BATCH_SIZE) {
        for poll in batch {
            summary.processed += 1;

            let Some(config) = configs_by_poll_id.get(&poll.id) else {
                summary.failed += 1;
                tracing::warn!(poll_id = %poll.id, "Poll config missing.");
                continue;
            };

            match synchronize_proposal(database, poll, config).await {
                Ok(ProposalSyncAction::Ratify) => summary.ratified += 1,
                Ok(ProposalSyncAction::Close) => summary.closed += 1,
                Ok(ProposalSyncAction::None) => {}
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(
                        poll_id = %poll.id,
                        "Failed to synchronize proposal: {error}"
                    );
                }
            }
        }
    }

    Ok(summary)
}

pub(crate) async fn is_public_channel_poll(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
) -> AppResult<bool> {
    load_poll(database, server_id, channel_id, poll_id).await?;
    let default_server_id = servers::default_server_id(database).await?;
    Ok(default_server_id == server_id)
}

pub(crate) async fn load_poll(
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
        .map(|vote| vote_service::shape_vote(vote, &selections))
        .collect::<Vec<_>>();
    let my_vote = current_user_id.and_then(|user_id| {
        votes
            .iter()
            .find(|vote| vote.user_id == user_id)
            .map(|vote| vote_service::shape_vote(vote, &selections))
    });

    let is_proposal = poll.poll_type == "proposal";

    Ok(PollResponse {
        id: poll.id.to_string(),
        body: poll.body,
        poll_type: poll.poll_type.clone(),
        stage: poll.stage,
        action: if is_proposal {
            poll_actions::service::shape_poll_action(database, poll.id).await?
        } else {
            None
        },
        config: shape_poll_config(config),
        options: if is_proposal {
            vec![]
        } else {
            options
                .into_iter()
                .map(|option| PollOptionResponse {
                    id: option.id.to_string(),
                    vote_count: selections
                        .iter()
                        .filter(|selection| {
                            selection.poll_option_id == option.id
                        })
                        .count(),
                    text: option.text,
                })
                .collect()
        },
        images: images.iter().map(shape_poll_image).collect(),
        user: PollUserResponse {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture,
        },
        agreement_vote_count: if is_proposal {
            votes
                .iter()
                .filter(|vote| vote.vote_type.as_deref() == Some("agree"))
                .count()
        } else {
            0
        },
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
        .unwrap_or(false)
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

async fn synchronize_proposal(
    database: &DatabaseConnection,
    poll: &polls::Model,
    config: &poll_configs::Model,
) -> AppResult<ProposalSyncAction> {
    let action = proposal_sync_action(
        config.closing_at,
        is_poll_ratifiable(database, poll.id).await?,
        Utc::now().fixed_offset(),
    );

    match action {
        ProposalSyncAction::None => Ok(action),
        ProposalSyncAction::Ratify => {
            ratify_poll(database, poll.id).await?;
            poll_actions::service::implement_poll_action(database, poll.id)
                .await?;
            Ok(action)
        }
        ProposalSyncAction::Close => {
            close_poll(database, poll.id).await?;
            Ok(action)
        }
    }
}

fn proposal_sync_action(
    closing_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    is_ratifiable: bool,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> ProposalSyncAction {
    let Some(closing_at) = closing_at else {
        return ProposalSyncAction::None;
    };

    if now < closing_at {
        return ProposalSyncAction::None;
    }

    if is_ratifiable {
        ProposalSyncAction::Ratify
    } else {
        ProposalSyncAction::Close
    }
}

async fn close_poll(
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
    active.stage = Set("closed".to_owned());
    active.update(database).await.map_err(internal_error)?;
    Ok(())
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

fn sanitize_text(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    let mut characters = value.trim().chars().peekable();
    while let Some(character) = characters.next() {
        if character != '<' {
            sanitized.push(character);
            continue;
        }

        let mut possible_tag = String::from("<");
        let mut found_tag_end = false;
        for character in characters.by_ref() {
            possible_tag.push(character);
            if character == '>' {
                found_tag_end = true;
                break;
            }
        }

        if !found_tag_end {
            sanitized.push_str(&possible_tag);
        }
    }
    sanitized
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
