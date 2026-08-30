//! Owns decision-model evaluation and final poll outcomes, including
//! ratification and closure. Periodic discovery remains in `sync.rs`.

use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use entity::{
    channel_members, channels,
    enums::{
        NotificationKind, PollClosedReason, PollDecisionMakingModel, PollStage,
        VoteType,
    },
    notifications, poll_configs, polls, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

use crate::{
    common::{ApiError, AppResult},
    notifications as notifications_service, poll_actions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProposalFinalization {
    Ratified,
    Closed(PollClosedReason),
}

/// The outcome plus the notification rows it persisted, so callers can publish
/// them once their transaction commits.
#[derive(Clone, Debug)]
pub(crate) struct FinalizedProposal {
    pub(crate) finalization: ProposalFinalization,
    pub(crate) notifications: Vec<notifications::Model>,
}

pub(crate) async fn is_poll_ratifiable<C>(
    database: &C,
    poll_id: Uuid,
    now: DateTime<FixedOffset>,
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

    is_poll_ratifiable_with_context(database, &poll, &config, now).await
}

pub(super) async fn is_poll_ratifiable_with_context<C>(
    database: &C,
    poll: &polls::Model,
    config: &poll_configs::Model,
    now: DateTime<FixedOffset>,
) -> AppResult<bool>
where
    C: ConnectionTrait,
{
    if poll.stage != PollStage::Voting {
        return Ok(false);
    }
    let votes = votes::Entity::find()
        .filter(votes::Column::PollId.eq(poll.id))
        .all(database)
        .await
        .map_err(internal_error)?;

    match config.decision_making_model {
        Some(PollDecisionMakingModel::Consensus) => {
            let member_count =
                get_channel_member_count(database, poll.channel_id).await?;
            has_consensus(&votes, config, member_count, now)
        }
        Some(PollDecisionMakingModel::Consent) => {
            has_consent(&votes, config, now)
        }
        Some(PollDecisionMakingModel::MajorityVote) => {
            let member_count =
                get_channel_member_count(database, poll.channel_id).await?;
            has_majority_vote(&votes, config, member_count, now)
        }
        None => Ok(false),
    }
}

async fn ratify_poll<C>(database: &C, poll_id: Uuid) -> AppResult<()>
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
    active.closed_reason = Set(None);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn finalize_ratifiable_proposal(
    transaction: &sea_orm::DatabaseTransaction,
    poll_id: Uuid,
    now: DateTime<FixedOffset>,
) -> AppResult<FinalizedProposal> {
    if let Some(reason) = poll_actions::service::plan_event_closed_reason(
        transaction,
        poll_id,
        now,
    )
    .await?
    {
        close_poll_with_reason(transaction, poll_id, Some(reason)).await?;
        let notifications = notify_proposal_outcome(
            transaction,
            poll_id,
            NotificationKind::ProposalClosed,
        )
        .await?;
        return Ok(FinalizedProposal {
            finalization: ProposalFinalization::Closed(reason),
            notifications,
        });
    }

    let action_notifications =
        poll_actions::service::implement_poll_action_in_transaction(
            transaction,
            poll_id,
        )
        .await?;
    ratify_poll(transaction, poll_id).await?;
    let mut notifications = notify_proposal_outcome(
        transaction,
        poll_id,
        NotificationKind::ProposalRatified,
    )
    .await?;
    notifications.extend(action_notifications);

    Ok(FinalizedProposal {
        finalization: ProposalFinalization::Ratified,
        notifications,
    })
}

/// Both the vote-triggered path and the scheduler finalize through this
/// module, so notifying here reaches every path exactly once.
async fn notify_proposal_outcome<C>(
    database: &C,
    poll_id: Uuid,
    kind: NotificationKind,
) -> AppResult<Vec<notifications::Model>>
where
    C: ConnectionTrait,
{
    let Some(poll) = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
    else {
        return Ok(Vec::new());
    };
    let Some(channel) = channels::Entity::find_by_id(poll.channel_id)
        .one(database)
        .await
        .map_err(internal_error)?
    else {
        return Ok(Vec::new());
    };

    let mut recipient_ids = vec![poll.user_id];
    recipient_ids.extend(
        votes::Entity::find()
            .filter(votes::Column::PollId.eq(poll_id))
            .all(database)
            .await
            .map_err(internal_error)?
            .into_iter()
            .map(|vote| vote.user_id),
    );

    notifications_service::create_notifications(
        database,
        notifications_service::NewNotification {
            kind,
            server_id: channel.server_id,
            channel_id: Some(poll.channel_id),
            actor_user_id: None,
            target: notifications_service::NotificationTarget::Poll(poll_id),
            vote_type: None,
            recipient_ids,
        },
    )
    .await
}

async fn get_channel_member_count<C>(
    database: &C,
    channel_id: Uuid,
) -> AppResult<usize>
where
    C: ConnectionTrait,
{
    channel_members::Entity::find()
        .filter(channel_members::Column::ChannelId.eq(channel_id))
        .count(database)
        .await
        .map(|count| count as usize)
        .map_err(internal_error)
}

fn has_consensus(
    votes: &[votes::Model],
    config: &poll_configs::Model,
    member_count: usize,
    now: DateTime<FixedOffset>,
) -> AppResult<bool> {
    if config
        .closing_at
        .map(|closing_at| now < closing_at)
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
    now: DateTime<FixedOffset>,
) -> AppResult<bool> {
    if config
        .closing_at
        .map(|closing_at| now < closing_at)
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

/// Consent evaluates only at a finite deadline, so a missing `closing_at`
/// is never ratifiable.
fn has_consent(
    votes: &[votes::Model],
    config: &poll_configs::Model,
    now: DateTime<FixedOffset>,
) -> AppResult<bool> {
    let Some(closing_at) = config.closing_at else {
        return Ok(false);
    };
    if now < closing_at {
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

pub(super) async fn close_poll<C>(database: &C, poll_id: Uuid) -> AppResult<()>
where
    C: ConnectionTrait,
{
    close_poll_with_reason(database, poll_id, None).await
}

pub(super) async fn close_poll_with_reason<C>(
    database: &C,
    poll_id: Uuid,
    reason: Option<PollClosedReason>,
) -> AppResult<()>
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
    active.closed_reason = Set(reason);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll outcome processing failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
