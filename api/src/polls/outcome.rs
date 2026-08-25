//! Owns decision-model evaluation and final poll outcomes, including
//! ratification and closure. Periodic discovery remains in `sync.rs`.

use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use entity::{
    channel_members,
    enums::{PollClosedReason, PollDecisionMakingModel, PollStage, VoteType},
    poll_configs, polls, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

use crate::{
    common::{ApiError, AppResult},
    poll_actions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProposalFinalization {
    Ratified,
    Closed(PollClosedReason),
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
) -> AppResult<ProposalFinalization> {
    if let Some(reason) = poll_actions::service::plan_event_closed_reason(
        transaction,
        poll_id,
        now,
    )
    .await?
    {
        close_poll_with_reason(transaction, poll_id, Some(reason)).await?;
        return Ok(ProposalFinalization::Closed(reason));
    }

    poll_actions::service::implement_poll_action_in_transaction(
        transaction,
        poll_id,
    )
    .await?;
    ratify_poll(transaction, poll_id).await?;
    Ok(ProposalFinalization::Ratified)
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use entity::enums::VoteType;
    use uuid::Uuid as NativeUuid;

    use super::*;

    fn timestamp(minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .expect("UTC offset should be valid")
            .with_ymd_and_hms(2026, 6, 29, 12, minute, 0)
            .single()
            .expect("timestamp should be valid")
    }

    fn consent_config(
        closing_at: Option<DateTime<FixedOffset>>,
    ) -> poll_configs::Model {
        poll_configs::Model {
            id: NativeUuid::new_v4(),
            poll_id: NativeUuid::new_v4(),
            decision_making_model: Some(PollDecisionMakingModel::Consent),
            disagreements_limit: Some(2),
            abstains_limit: Some(2),
            agreement_threshold: Some(51),
            quorum_enabled: Some(false),
            quorum_threshold: Some(25),
            multiple_choice: None,
            closing_at,
            created_at: timestamp(0),
            updated_at: timestamp(0),
        }
    }

    fn vote(vote_type: VoteType) -> votes::Model {
        votes::Model {
            id: NativeUuid::new_v4(),
            poll_id: NativeUuid::new_v4(),
            user_id: NativeUuid::new_v4(),
            vote_type: Some(vote_type),
            created_at: timestamp(0),
            updated_at: timestamp(0),
        }
    }

    fn votes_of(vote_type: VoteType, count: usize) -> Vec<votes::Model> {
        (0..count).map(|_| vote(vote_type)).collect()
    }

    #[test]
    fn consent_is_not_reached_before_the_deadline() {
        let config = consent_config(Some(timestamp(30)));

        let result = has_consent(&[], &config, timestamp(29)).unwrap();

        assert!(!result);
    }

    #[test]
    fn consent_is_reached_exactly_at_the_deadline() {
        let config = consent_config(Some(timestamp(30)));
        let votes = votes_of(VoteType::Agree, 1);

        let result = has_consent(&votes, &config, timestamp(30)).unwrap();

        assert!(result);
    }

    #[test]
    fn consent_is_reached_after_the_deadline() {
        let config = consent_config(Some(timestamp(30)));

        let result = has_consent(&[], &config, timestamp(31)).unwrap();

        assert!(result);
    }

    #[test]
    fn consent_is_never_reached_without_a_deadline() {
        let config = consent_config(None);

        let result = has_consent(&[], &config, timestamp(30)).unwrap();

        assert!(!result);
    }

    #[test]
    fn silence_counts_as_consent_at_the_deadline() {
        let config = consent_config(Some(timestamp(30)));

        let result = has_consent(&[], &config, timestamp(30)).unwrap();

        assert!(result);
    }

    #[test]
    fn disagreements_within_their_limit_still_reach_consent() {
        let config = consent_config(Some(timestamp(30)));

        let below = votes_of(VoteType::Disagree, 1);
        let at_limit = votes_of(VoteType::Disagree, 2);

        assert!(has_consent(&below, &config, timestamp(30)).unwrap());
        assert!(has_consent(&at_limit, &config, timestamp(30)).unwrap());
    }

    #[test]
    fn disagreements_above_their_limit_prevent_consent() {
        let config = consent_config(Some(timestamp(30)));
        let votes = votes_of(VoteType::Disagree, 3);

        assert!(!has_consent(&votes, &config, timestamp(30)).unwrap());
    }

    #[test]
    fn abstentions_within_their_limit_still_reach_consent() {
        let config = consent_config(Some(timestamp(30)));

        let below = votes_of(VoteType::Abstain, 1);
        let at_limit = votes_of(VoteType::Abstain, 2);

        assert!(has_consent(&below, &config, timestamp(30)).unwrap());
        assert!(has_consent(&at_limit, &config, timestamp(30)).unwrap());
    }

    #[test]
    fn abstentions_above_their_limit_prevent_consent() {
        let config = consent_config(Some(timestamp(30)));
        let votes = votes_of(VoteType::Abstain, 3);

        assert!(!has_consent(&votes, &config, timestamp(30)).unwrap());
    }

    #[test]
    fn a_single_block_prevents_consent_within_limits() {
        let config = consent_config(Some(timestamp(30)));
        let mut votes = votes_of(VoteType::Agree, 5);
        votes.push(vote(VoteType::Block));

        assert!(!has_consent(&votes, &config, timestamp(30)).unwrap());
    }

    #[test]
    fn unmet_quorum_does_not_block_consent() {
        let mut config = consent_config(Some(timestamp(30)));
        config.quorum_enabled = Some(true);
        config.quorum_threshold = Some(100);

        assert!(has_consent(&[], &config, timestamp(30)).unwrap());
    }

    #[test]
    fn agreement_threshold_does_not_apply_to_consent() {
        let mut config = consent_config(Some(timestamp(30)));
        config.agreement_threshold = Some(100);
        let votes = votes_of(VoteType::Disagree, 2);

        assert!(has_consent(&votes, &config, timestamp(30)).unwrap());
    }
}
