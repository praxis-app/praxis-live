use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{
    channels as channel_entities,
    enums::{
        PollActionType, PollDecisionMakingModel, PollStage, PollType, VoteType,
    },
    poll_actions as poll_action_entities, poll_configs, poll_images,
    poll_option_selections, poll_options, polls, users, votes,
};
use sea_orm::{
    prelude::Uuid, sea_query::LockType, ActiveModelTrait, ColumnTrait,
    ConnectionTrait, DatabaseConnection, DeleteResult, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::time::{self, MissedTickBehavior};
use uuid::Uuid as NativeUuid;

use super::types::{
    CallDecisionResponse, CreatePollRequest, PollConfigResponse,
    PollImageResponse, PollOptionResponse, PollResponse, PollUserResponse,
    StoredPollImage,
};
use crate::{
    channels,
    common::{encryption, text::sanitize_text, ApiError, AppResult},
    messages::types::serialize_timestamp,
    poll_actions::{self, types::CreatePollActionRequest},
    pub_sub::{PubSubService, PubSubTopic},
    servers::{self, server_configs},
    users as users_service,
    votes::service as vote_service,
};

const MAX_IMAGE_COUNT: usize = 8;
const MAX_POLL_BODY_LENGTH: usize = 8_000;

const PROPOSAL_SYNC_BATCH_SIZE: usize = 20;
const PROPOSAL_SYNC_INTERVAL_SECONDS: u64 = 60 * 5;

const POLL_CLOSURE_BATCH_SIZE: usize = 20;
const POLL_CLOSURE_INTERVAL_SECONDS: u64 = 60 * 5;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ExpiredPollClosureSummary {
    processed: usize,
    closed: usize,
    failed: usize,
}

pub(crate) fn spawn_proposal_synchronizer(
    database: DatabaseConnection,
    pub_sub_service: PubSubService,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(std::time::Duration::from_secs(
            configured_interval_seconds(
                "PROPOSAL_SYNC_INTERVAL_SECONDS",
                PROPOSAL_SYNC_INTERVAL_SECONDS,
            ),
        ));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            match synchronize_proposals(&database, &pub_sub_service).await {
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

pub(crate) fn spawn_expired_poll_closer(
    database: DatabaseConnection,
    pub_sub_service: PubSubService,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(std::time::Duration::from_secs(
            configured_interval_seconds(
                "POLL_CLOSURE_INTERVAL_SECONDS",
                POLL_CLOSURE_INTERVAL_SECONDS,
            ),
        ));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            match close_expired_polls(&database, &pub_sub_service).await {
                Ok(summary) if summary.processed > 0 => {
                    tracing::info!(
                        processed = summary.processed,
                        closed = summary.closed,
                        failed = summary.failed,
                        "Closed expired polls."
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("Failed to close expired polls: {error}");
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
    create_poll_record(database, server_id, channel_id, None, user_id, request)
        .await
}

pub(crate) async fn create_call_poll(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    user_id: Uuid,
    request: CreatePollRequest,
) -> AppResult<PollResponse> {
    crate::calls::service::get_call(database, server_id, channel_id, call_id)
        .await?;
    create_poll_record(
        database,
        server_id,
        channel_id,
        Some(call_id),
        user_id,
        request,
    )
    .await
}

pub(crate) async fn broadcast_poll_update(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Option<Uuid>,
    poll_id: Uuid,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(database, channel_id).await?;

    for member_id in members {
        if Some(member_id) == sender_id {
            continue;
        }

        let poll = get_poll_response(
            database,
            server_id,
            channel_id,
            poll_id,
            Some(member_id),
        )
        .await?;
        let body = serde_json::json!({
            "type": "poll",
            "poll": poll,
        });
        let topic =
            PubSubTopic::new_poll(server_id, channel_id, member_id).to_string();
        pub_sub_service.publish(&topic, body).await?;
    }

    Ok(())
}

pub(crate) async fn broadcast_poll_image_upload(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    poll_id: &str,
    image_id: &str,
) -> AppResult<()> {
    let body = serde_json::json!({
        "type": "image",
        "isPlaceholder": false,
        "pollId": poll_id,
        "imageId": image_id,
    });

    broadcast_to_channel_members(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        sender_id,
        body,
    )
    .await
}

async fn create_poll_record(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Option<Uuid>,
    user_id: Uuid,
    request: CreatePollRequest,
) -> AppResult<PollResponse> {
    validate_create_poll(&request)?;

    let body = request
        .body
        .as_deref()
        .map(sanitize_text)
        .filter(|value| !value.is_empty());
    let poll_type = parse_poll_type(&request.poll_type)?;
    let is_proposal = poll_type == PollType::Proposal;
    ensure_allowed_to_create_proposal(database, user_id, &request).await?;
    let server_config =
        server_configs::service::ensure_server_config(database, server_id)
            .await?;
    if let Some(config_change) = request
        .action
        .as_ref()
        .filter(|action| action.action_type == PollActionType::ChangeSettings)
        .and_then(|action| action.server_config.as_ref())
    {
        poll_actions::service::validate_server_config_change(
            config_change,
            &server_config,
        )?;
    }
    let closing_at = resolve_poll_closing_at(
        is_proposal,
        server_config.voting_time_limit,
        request.closing_at,
        Utc::now().fixed_offset(),
    );

    let encrypted = match body.as_deref() {
        Some(body) => {
            let (key, unwrapped_key) =
                channels::get_unwrapped_channel_key(database, channel_id)
                    .await?;
            Some((key.id, encryption::encrypt_text(body, &unwrapped_key)?))
        }
        None => None,
    };

    let poll = polls::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        key_id: Set(encrypted.as_ref().map(|(key_id, _)| *key_id)),
        ciphertext: Set(encrypted
            .as_ref()
            .map(|(_, value)| value.ciphertext.clone())),
        iv: Set(encrypted.as_ref().map(|(_, value)| value.iv.clone())),
        tag: Set(encrypted.as_ref().map(|(_, value)| value.tag.clone())),
        poll_type: Set(poll_type),
        user_id: Set(user_id),
        channel_id: Set(channel_id),
        call_id: Set(call_id),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    poll_configs::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll.id),
        decision_making_model: Set(is_proposal
            .then_some(server_config.decision_making_model.into())),
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

pub(crate) async fn get_call_decision(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    user_id: Uuid,
) -> AppResult<CallDecisionResponse> {
    crate::calls::service::get_call(database, server_id, channel_id, call_id)
        .await?;

    let active_item = match polls::Entity::find()
        .filter(polls::Column::ChannelId.eq(channel_id))
        .filter(polls::Column::CallId.eq(call_id))
        .filter(polls::Column::Stage.eq(PollStage::Voting))
        .order_by_desc(polls::Column::CreatedAt)
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(poll) => Some(shape_poll(database, poll, Some(user_id)).await?),
        None => {
            let active_channel_poll = polls::Entity::find()
                .filter(polls::Column::ChannelId.eq(channel_id))
                .filter(polls::Column::Stage.eq(PollStage::Voting))
                .order_by_desc(polls::Column::CreatedAt)
                .one(database)
                .await
                .map_err(internal_error)?;

            match active_channel_poll {
                Some(poll) => {
                    Some(shape_poll(database, poll, Some(user_id)).await?)
                }
                None => None,
            }
        }
    };

    let recent_result = match polls::Entity::find()
        .filter(polls::Column::ChannelId.eq(channel_id))
        .filter(
            polls::Column::Stage
                .is_in([PollStage::Ratified, PollStage::Closed]),
        )
        .order_by_desc(polls::Column::UpdatedAt)
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(poll) => Some(shape_poll(database, poll, Some(user_id)).await?),
        None => None,
    };

    Ok(CallDecisionResponse {
        active_item,
        recent_result,
    })
}

pub(crate) async fn store_poll_image(
    database: &DatabaseConnection,
    upload_root: &Path,
    poll: &polls::Model,
    image_id: Uuid,
    content_type: Option<String>,
    bytes: Vec<u8>,
) -> AppResult<PollImageResponse> {
    if bytes.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No image uploaded",
        ));
    }

    let image = poll_images::Entity::find_by_id(image_id)
        .filter(poll_images::Column::PollId.eq(poll.id))
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

pub(crate) async fn delete_poll(
    database: &DatabaseConnection,
    upload_root: &Path,
    poll: &polls::Model,
) -> AppResult<DeleteResult> {
    let images = poll_images::Entity::find()
        .filter(poll_images::Column::PollId.eq(poll.id))
        .all(database)
        .await
        .map_err(internal_error)?;

    for storage_key in
        images.iter().filter_map(|image| image.storage_key.as_ref())
    {
        tokio::fs::remove_file(upload_root.join(storage_key))
            .await
            .map_err(internal_error)?;
    }

    polls::Entity::delete_by_id(poll.id)
        .exec(database)
        .await
        .map_err(internal_error)
}

pub(crate) fn upload_root() -> PathBuf {
    crate::common::storage::upload_root()
}

pub(crate) async fn is_poll_ratifiable<C>(
    database: &C,
    poll_id: Uuid,
) -> AppResult<bool>
where
    C: ConnectionTrait,
{
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

    match config.decision_making_model {
        Some(PollDecisionMakingModel::Consensus) => {
            has_consensus(&votes, &config, member_count)
        }
        Some(PollDecisionMakingModel::Consent) => has_consent(&votes, &config),
        Some(PollDecisionMakingModel::MajorityVote) => {
            has_majority_vote(&votes, &config, member_count)
        }
        None => Ok(false),
    }
}

pub(crate) async fn ratify_poll<C>(database: &C, poll_id: Uuid) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    let mut active = poll.into_active_model();
    active.stage = Set(PollStage::Ratified);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

async fn synchronize_proposals(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
) -> AppResult<ProposalSyncSummary> {
    let mut summary =
        retry_pending_poll_actions(database, pub_sub_service).await?;
    let configs = poll_configs::Entity::find()
        .filter(poll_configs::Column::ClosingAt.is_not_null())
        .all(database)
        .await
        .map_err(internal_error)?;
    if configs.is_empty() {
        return Ok(summary);
    }

    let configs_by_poll_id = configs
        .into_iter()
        .map(|config| (config.poll_id, config))
        .collect::<HashMap<_, _>>();
    let poll_ids = configs_by_poll_id.keys().copied().collect::<Vec<_>>();
    let proposals = polls::Entity::find()
        .filter(polls::Column::Id.is_in(poll_ids))
        .filter(polls::Column::PollType.eq(PollType::Proposal))
        .filter(polls::Column::Stage.eq(PollStage::Voting))
        .all(database)
        .await
        .map_err(internal_error)?;
    if proposals.is_empty() {
        return Ok(summary);
    }

    for batch in proposals.chunks(PROPOSAL_SYNC_BATCH_SIZE) {
        for poll in batch {
            summary.processed += 1;

            let Some(config) = configs_by_poll_id.get(&poll.id) else {
                summary.failed += 1;
                tracing::warn!(poll_id = %poll.id, "Poll config missing.");
                continue;
            };

            match synchronize_proposal(database, poll, config).await {
                Ok(ProposalSyncAction::Ratify) => {
                    broadcast_stored_poll_update(
                        database,
                        pub_sub_service,
                        poll,
                        None,
                    )
                    .await?;
                    summary.ratified += 1;
                }
                Ok(ProposalSyncAction::Close) => {
                    broadcast_stored_poll_update(
                        database,
                        pub_sub_service,
                        poll,
                        None,
                    )
                    .await?;
                    summary.closed += 1;
                }
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

async fn retry_pending_poll_actions(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
) -> AppResult<ProposalSyncSummary> {
    let actions = poll_action_entities::Entity::find()
        .filter(poll_action_entities::Column::ExecutedAt.is_null())
        .all(database)
        .await
        .map_err(internal_error)?;
    let mut summary = ProposalSyncSummary::default();

    for action in actions {
        let Some(poll) = polls::Entity::find_by_id(action.poll_id)
            .filter(polls::Column::Stage.eq(PollStage::Ratified))
            .one(database)
            .await
            .map_err(internal_error)?
        else {
            continue;
        };

        summary.processed += 1;
        match poll_actions::service::implement_poll_action(database, poll.id)
            .await
        {
            Ok(true) => {
                broadcast_stored_poll_update(
                    database,
                    pub_sub_service,
                    &poll,
                    None,
                )
                .await?;
            }
            Ok(false) => {}
            Err(error) => {
                summary.failed += 1;
                tracing::warn!(
                    poll_id = %poll.id,
                    "Failed to retry poll action: {error}"
                );
            }
        }
    }

    Ok(summary)
}

async fn close_expired_polls(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
) -> AppResult<ExpiredPollClosureSummary> {
    let configs = poll_configs::Entity::find()
        .filter(poll_configs::Column::ClosingAt.is_not_null())
        .all(database)
        .await
        .map_err(internal_error)?;
    if configs.is_empty() {
        return Ok(ExpiredPollClosureSummary::default());
    }

    let now = Utc::now().fixed_offset();
    let poll_ids = configs
        .into_iter()
        .filter_map(|config| {
            config
                .closing_at
                .filter(|closing_at| *closing_at <= now)
                .map(|_| config.poll_id)
        })
        .collect::<Vec<_>>();
    if poll_ids.is_empty() {
        return Ok(ExpiredPollClosureSummary::default());
    }

    let expired_polls = polls::Entity::find()
        .filter(polls::Column::Id.is_in(poll_ids))
        .filter(polls::Column::PollType.eq(PollType::Poll))
        .filter(polls::Column::Stage.eq(PollStage::Voting))
        .all(database)
        .await
        .map_err(internal_error)?;
    if expired_polls.is_empty() {
        return Ok(ExpiredPollClosureSummary::default());
    }

    let mut summary = ExpiredPollClosureSummary::default();
    for batch in expired_polls.chunks(POLL_CLOSURE_BATCH_SIZE) {
        for poll in batch {
            summary.processed += 1;

            match close_poll(database, poll.id).await {
                Ok(()) => {
                    broadcast_stored_poll_update(
                        database,
                        pub_sub_service,
                        poll,
                        None,
                    )
                    .await?;
                    summary.closed += 1;
                }
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(
                        poll_id = %poll.id,
                        "Failed to close expired poll: {error}"
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

pub(crate) async fn get_poll_response(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    current_user_id: Option<Uuid>,
) -> AppResult<PollResponse> {
    let poll = load_poll(database, server_id, channel_id, poll_id).await?;
    shape_poll(database, poll, current_user_id).await
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
        body: decrypt_poll_body(database, &poll).await?,
        poll_type: poll.poll_type.to_string(),
        stage: poll.stage.to_string(),
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
                .filter(|vote| vote.vote_type == Some(VoteType::Agree))
                .count()
        } else {
            0
        },
        votes: shaped_votes,
        my_vote,
        member_count: get_poll_member_count(database, poll.id).await?,
        source_call_id: poll.call_id.map(|call_id| call_id.to_string()),
        created_at: serialize_timestamp(poll.created_at),
    })
}

fn shape_poll_config(config: poll_configs::Model) -> PollConfigResponse {
    PollConfigResponse {
        decision_making_model: config
            .decision_making_model
            .map(|value| value.to_string()),
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

async fn decrypt_poll_body(
    database: &DatabaseConnection,
    poll: &polls::Model,
) -> AppResult<Option<String>> {
    let (Some(ciphertext), Some(iv), Some(tag), Some(key_id)) = (
        poll.ciphertext.as_ref(),
        poll.iv.as_ref(),
        poll.tag.as_ref(),
        poll.key_id,
    ) else {
        return Ok(None);
    };
    let key_map =
        channels::get_unwrapped_channel_key_map(database, vec![key_id]).await?;
    Ok(key_map.get(&key_id).and_then(|key| {
        encryption::decrypt_text(ciphertext, iv, tag, key).ok()
    }))
}

async fn ensure_allowed_to_create_proposal(
    database: &DatabaseConnection,
    user_id: Uuid,
    request: &CreatePollRequest,
) -> AppResult<()> {
    if request.poll_type != "proposal" {
        return Ok(());
    }
    if request
        .action
        .as_ref()
        .map(|action| action.action_type.as_str())
        == Some("test")
    {
        return Ok(());
    }

    let user = users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })?;

    if user.anonymous {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Only registered users can create non-test proposals.",
        ));
    }

    Ok(())
}

async fn get_poll_member_count<C>(
    database: &C,
    poll_id: Uuid,
) -> AppResult<usize>
where
    C: ConnectionTrait,
{
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
        match vote.vote_type {
            Some(VoteType::Agree) => agreements += 1,
            Some(VoteType::Disagree) => disagreements += 1,
            Some(VoteType::Abstain) => abstains += 1,
            Some(VoteType::Block) => blocks += 1,
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

fn configured_interval_seconds(env_key: &str, default: u64) -> u64 {
    std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

async fn synchronize_proposal(
    database: &DatabaseConnection,
    poll: &polls::Model,
    config: &poll_configs::Model,
) -> AppResult<ProposalSyncAction> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let locked_poll = polls::Entity::find_by_id(poll.id)
        .lock(LockType::Update)
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;

    if locked_poll.stage != PollStage::Voting {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(ProposalSyncAction::None);
    }

    let action = proposal_sync_action(
        config.closing_at,
        is_poll_ratifiable(&transaction, poll.id).await?,
        Utc::now().fixed_offset(),
    );

    match action {
        ProposalSyncAction::None => {
            transaction.commit().await.map_err(internal_error)?;
        }
        ProposalSyncAction::Ratify => {
            ratify_poll(&transaction, poll.id).await?;
            transaction.commit().await.map_err(internal_error)?;
            poll_actions::service::implement_poll_action(database, poll.id)
                .await?;
        }
        ProposalSyncAction::Close => {
            close_poll(&transaction, poll.id).await?;
            transaction.commit().await.map_err(internal_error)?;
        }
    }

    Ok(action)
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

fn resolve_poll_closing_at(
    is_proposal: bool,
    voting_time_limit: i32,
    requested_closing_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    if !is_proposal {
        return requested_closing_at;
    }

    (voting_time_limit > 0)
        .then(|| now + Duration::minutes(i64::from(voting_time_limit)))
}

async fn close_poll<C>(database: &C, poll_id: Uuid) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    let mut active = poll.into_active_model();
    active.stage = Set(PollStage::Closed);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

fn validate_create_poll(request: &CreatePollRequest) -> AppResult<()> {
    parse_poll_type(&request.poll_type)?;
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

fn parse_poll_type(value: &str) -> AppResult<PollType> {
    value.parse().map_err(|_| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Poll type is invalid.")
    })
}

fn validate_action(
    action: Option<&CreatePollActionRequest>,
    body: Option<&str>,
) -> AppResult<()> {
    let action = action.ok_or_else(|| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Action is required.")
    })?;
    if matches!(
        action.action_type,
        PollActionType::General | PollActionType::Test
    ) && body.map(str::trim).map(str::is_empty).unwrap_or(true)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Polls with this action must include a body.",
        ));
    }
    let payload_matches_action = match action.action_type {
        PollActionType::ChangeSettings => {
            action.server_role.is_none() && action.server_config.is_some()
        }
        PollActionType::ChangeRole | PollActionType::CreateRole => {
            action.server_role.is_some() && action.server_config.is_none()
        }
        _ => action.server_role.is_none() && action.server_config.is_none(),
    };
    if !payload_matches_action {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll action payload does not match its action type.",
        ));
    }
    if action.action_type == PollActionType::ChangeSettings {
        let config = action.server_config.as_ref().expect("checked above");
        if config.anonymous_users_enabled.is_none()
            && config.decision_making_model.is_none()
            && config.disagreements_limit.is_none()
            && config.abstains_limit.is_none()
            && config.agreement_threshold.is_none()
            && config.quorum_enabled.is_none()
            && config.quorum_threshold.is_none()
            && config.voting_time_limit.is_none()
        {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls to change server settings must include at least 1 change.",
            ));
        }
    }
    if action.action_type == PollActionType::ChangeRole {
        let role = action.server_role.as_ref().expect("checked above");

        if role.server_role_to_update_id.is_none() {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Polls to change server roles must include a server role to update.",
            ));
        }

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

async fn broadcast_to_channel_members(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    body: serde_json::Value,
) -> AppResult<()> {
    let members =
        channels::get_channel_member_user_ids(database, channel_id).await?;

    for member_id in members {
        if member_id == sender_id {
            continue;
        }

        let topic =
            PubSubTopic::new_poll(server_id, channel_id, member_id).to_string();
        pub_sub_service.publish(&topic, body.clone()).await?;
    }

    Ok(())
}

async fn broadcast_stored_poll_update(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    poll: &polls::Model,
    sender_id: Option<Uuid>,
) -> AppResult<()> {
    let channel = channel_entities::Entity::find_by_id(poll.channel_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Channel not found.")
        })?;

    broadcast_poll_update(
        database,
        pub_sub_service,
        channel.server_id,
        poll.channel_id,
        sender_id,
        poll.id,
    )
    .await
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, TimeZone};

    use super::resolve_poll_closing_at;

    fn timestamp(minute: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .expect("UTC offset should be valid")
            .with_ymd_and_hms(2026, 6, 29, 12, minute, 0)
            .single()
            .expect("timestamp should be valid")
    }

    #[test]
    fn proposal_deadline_comes_from_server_voting_duration() {
        let now = timestamp(0);
        let requested_closing_at = timestamp(5);

        let closing_at =
            resolve_poll_closing_at(true, 30, Some(requested_closing_at), now);

        assert_eq!(closing_at, Some(timestamp(30)));
    }

    #[test]
    fn unlimited_proposal_ignores_requested_deadline() {
        let closing_at =
            resolve_poll_closing_at(true, 0, Some(timestamp(5)), timestamp(0));

        assert_eq!(closing_at, None);
    }

    #[test]
    fn regular_poll_keeps_requested_deadline() {
        let requested_closing_at = timestamp(5);

        let closing_at = resolve_poll_closing_at(
            false,
            30,
            Some(requested_closing_at),
            timestamp(0),
        );

        assert_eq!(closing_at, Some(requested_closing_at));
    }
}
