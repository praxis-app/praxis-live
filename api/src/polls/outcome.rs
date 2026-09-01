//! Owns decision-model evaluation and final poll outcomes, including
//! ratification and closure. Periodic discovery remains in `sync.rs`.

use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use entity::{
    channel_members, channels,
    enums::{PollClosedReason, PollDecisionMakingModel, PollStage, VoteType},
    poll_configs, polls, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use std::collections::HashSet;

use crate::{
    authz,
    common::{ApiError, AppResult},
    poll_actions,
};

/// Server-role subject that gates blocking when a server restricts it.
pub(crate) const PROPOSAL_BLOCK_SUBJECT: &str = "ProposalBlock";

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

    let ignored_blockers =
        get_ineligible_block_voters(database, poll, config, &votes).await?;
    let votes = filter_arithmetic_votes(votes, &ignored_blockers);

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

/// Blocks cast by members who no longer hold `ProposalBlock` are kept as vote
/// rows but stop carrying veto weight. Resolved once per evaluation over the
/// distinct block voters rather than per vote.
async fn get_ineligible_block_voters<C>(
    database: &C,
    poll: &polls::Model,
    config: &poll_configs::Model,
    votes: &[votes::Model],
) -> AppResult<HashSet<Uuid>>
where
    C: ConnectionTrait,
{
    if config.blocks_restricted != Some(true) {
        return Ok(HashSet::new());
    }

    let block_voters: HashSet<Uuid> = votes
        .iter()
        .filter(|vote| vote.vote_type == Some(VoteType::Block))
        .map(|vote| vote.user_id)
        .collect();
    if block_voters.is_empty() {
        return Ok(HashSet::new());
    }

    let server_id = get_server_id_by_channel(database, poll.channel_id).await?;
    let eligible = authz::filter_users_who_can(
        database,
        &block_voters,
        "create",
        PROPOSAL_BLOCK_SUBJECT,
        server_id,
    )
    .await?;
    Ok(block_voters.difference(&eligible).copied().collect())
}

fn filter_arithmetic_votes(
    votes: Vec<votes::Model>,
    ignored_blockers: &HashSet<Uuid>,
) -> Vec<votes::Model> {
    votes
        .into_iter()
        .filter(|vote| {
            vote.vote_type != Some(VoteType::Block)
                || !ignored_blockers.contains(&vote.user_id)
        })
        .collect()
}

async fn get_server_id_by_channel<C>(
    database: &C,
    channel_id: Uuid,
) -> AppResult<Uuid>
where
    C: ConnectionTrait,
{
    channels::Entity::find_by_id(channel_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .map(|channel| channel.server_id)
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Channel not found.")
        })
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
    use chrono::{FixedOffset, TimeZone};
    use entity::{
        enums::{PollDecisionMakingModel, PollStage, PollType, VoteType},
        poll_configs, polls, votes,
    };
    use sea_orm::{
        Database, DatabaseConnection, DbBackend, DbErr, ProxyDatabaseTrait,
        ProxyExecResult, ProxyRow, Statement,
    };
    use std::{
        collections::{BTreeMap, HashSet, VecDeque},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };
    use uuid::Uuid as NativeUuid;

    use super::{
        count_votes, filter_arithmetic_votes, get_ineligible_block_voters,
        has_consensus, has_consent,
    };

    fn timestamp(minute: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(0)
            .expect("UTC offset should be valid")
            .with_ymd_and_hms(2026, 6, 29, 12, minute, 0)
            .single()
            .expect("timestamp should be valid")
    }

    fn config(
        model: PollDecisionMakingModel,
        blocks_restricted: Option<bool>,
        closing_at: Option<chrono::DateTime<FixedOffset>>,
    ) -> poll_configs::Model {
        poll_configs::Model {
            id: NativeUuid::new_v4(),
            poll_id: NativeUuid::new_v4(),
            decision_making_model: Some(model),
            disagreements_limit: Some(2),
            abstains_limit: Some(2),
            agreement_threshold: Some(51),
            quorum_enabled: Some(false),
            quorum_threshold: Some(50),
            blocks_restricted,
            multiple_choice: None,
            closing_at,
            created_at: timestamp(0),
            updated_at: timestamp(0),
        }
    }

    fn vote(user_id: NativeUuid, vote_type: VoteType) -> votes::Model {
        votes::Model {
            id: NativeUuid::new_v4(),
            poll_id: NativeUuid::new_v4(),
            user_id,
            vote_type: Some(vote_type),
            created_at: timestamp(0),
            updated_at: timestamp(0),
        }
    }

    fn proposal(channel_id: NativeUuid) -> polls::Model {
        polls::Model {
            id: NativeUuid::new_v4(),
            ciphertext: None,
            iv: None,
            tag: None,
            key_id: None,
            stage: PollStage::Voting,
            closed_reason: None,
            poll_type: PollType::Proposal,
            user_id: NativeUuid::new_v4(),
            channel_id,
            call_id: None,
            created_at: timestamp(0),
            updated_at: timestamp(0),
        }
    }

    #[derive(Debug)]
    struct QueryQueue {
        results: Mutex<VecDeque<Result<Vec<ProxyRow>, DbErr>>>,
        query_count: Arc<AtomicUsize>,
    }

    #[sea_orm::entity::prelude::async_trait::async_trait]
    impl ProxyDatabaseTrait for QueryQueue {
        async fn query(
            &self,
            _statement: Statement,
        ) -> Result<Vec<ProxyRow>, DbErr> {
            self.query_count.fetch_add(1, Ordering::SeqCst);
            self.results
                .lock()
                .expect("query result queue should be available")
                .pop_front()
                .expect("expected a queued query result")
        }

        async fn execute(
            &self,
            _statement: Statement,
        ) -> Result<ProxyExecResult, DbErr> {
            Ok(ProxyExecResult::default())
        }
    }

    async fn proxy_database(
        results: Vec<Result<Vec<ProxyRow>, DbErr>>,
    ) -> (DatabaseConnection, Arc<AtomicUsize>) {
        let query_count = Arc::new(AtomicUsize::new(0));
        let proxy = QueryQueue {
            results: Mutex::new(results.into()),
            query_count: query_count.clone(),
        };
        let database = Database::connect_proxy(
            DbBackend::Postgres,
            Arc::new(Box::new(proxy)),
        )
        .await
        .expect("proxy database should connect");
        (database, query_count)
    }

    fn channel_row(id: NativeUuid, server_id: NativeUuid) -> ProxyRow {
        ProxyRow::new(BTreeMap::from([
            ("id".to_owned(), id.into()),
            ("server_id".to_owned(), server_id.into()),
            ("name".to_owned(), "general".to_owned().into()),
            ("description".to_owned(), Option::<String>::None.into()),
            ("channel_type".to_owned(), "text".to_owned().into()),
            ("sort_order".to_owned(), 0_i32.into()),
            ("created_at".to_owned(), timestamp(0).into()),
            ("updated_at".to_owned(), timestamp(0).into()),
        ]))
    }

    #[tokio::test]
    async fn restricted_block_eligibility_propagates_permission_query_errors() {
        let server_id = NativeUuid::new_v4();
        let channel_id = NativeUuid::new_v4();
        let blocker = NativeUuid::new_v4();
        let poll = proposal(channel_id);
        let config =
            config(PollDecisionMakingModel::Consensus, Some(true), None);
        let votes = vec![vote(blocker, VoteType::Block)];
        let (database, _) = proxy_database(vec![
            Ok(vec![channel_row(channel_id, server_id)]),
            Err(DbErr::Custom("permission lookup failed".to_owned())),
        ])
        .await;

        let error =
            get_ineligible_block_voters(&database, &poll, &config, &votes)
                .await
                .expect_err(
                    "permission query errors must stop outcome evaluation",
                );

        assert_eq!(
            error.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn restricted_block_eligibility_uses_a_fixed_query_count() {
        let server_id = NativeUuid::new_v4();
        let channel_id = NativeUuid::new_v4();
        let blocker_a = NativeUuid::new_v4();
        let blocker_b = NativeUuid::new_v4();
        let poll = proposal(channel_id);
        let config =
            config(PollDecisionMakingModel::Consensus, Some(true), None);
        let votes = vec![
            vote(blocker_a, VoteType::Block),
            vote(blocker_b, VoteType::Block),
        ];
        let (database, query_count) = proxy_database(vec![
            Ok(vec![channel_row(channel_id, server_id)]),
            Ok(vec![]),
            Ok(vec![]),
        ])
        .await;

        let ineligible =
            get_ineligible_block_voters(&database, &poll, &config, &votes)
                .await
                .expect("eligibility lookup should succeed");

        assert_eq!(ineligible, HashSet::from([blocker_a, blocker_b]));
        assert_eq!(query_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn ignored_blockers_are_left_out_of_the_block_tally() {
        let blocker = NativeUuid::new_v4();
        let votes = vec![
            vote(NativeUuid::new_v4(), VoteType::Agree),
            vote(blocker, VoteType::Block),
        ];

        let filtered =
            filter_arithmetic_votes(votes.clone(), &HashSet::from([blocker]));
        let (_, _, _, counted) = count_votes(&filtered);
        let (_, _, _, uncounted) = count_votes(&votes);

        assert_eq!(counted, 0);
        assert_eq!(uncounted, 1);
    }

    #[test]
    fn consensus_is_prevented_by_an_eligible_block() {
        let votes = vec![
            vote(NativeUuid::new_v4(), VoteType::Agree),
            vote(NativeUuid::new_v4(), VoteType::Block),
        ];

        let has_consensus = has_consensus(
            &votes,
            &config(PollDecisionMakingModel::Consensus, Some(true), None),
            2,
            timestamp(1),
        )
        .expect("consensus evaluation should succeed");

        assert!(!has_consensus);
    }

    #[test]
    fn consensus_ignores_a_block_from_an_ineligible_voter() {
        let blocker = NativeUuid::new_v4();
        let votes = vec![
            vote(NativeUuid::new_v4(), VoteType::Agree),
            vote(blocker, VoteType::Block),
        ];

        let votes = filter_arithmetic_votes(votes, &HashSet::from([blocker]));
        let has_consensus = has_consensus(
            &votes,
            &config(PollDecisionMakingModel::Consensus, Some(true), None),
            2,
            timestamp(1),
        )
        .expect("consensus evaluation should succeed");

        assert!(has_consensus);
    }

    #[test]
    fn consent_ignores_a_block_from_an_ineligible_voter() {
        let blocker = NativeUuid::new_v4();
        let votes = vec![vote(blocker, VoteType::Block)];
        let config = config(
            PollDecisionMakingModel::Consent,
            Some(true),
            Some(timestamp(1)),
        );

        let blocked = has_consent(&votes, &config, timestamp(2))
            .expect("consent evaluation should succeed");
        let votes = filter_arithmetic_votes(votes, &HashSet::from([blocker]));
        let ignored = has_consent(&votes, &config, timestamp(2))
            .expect("consent evaluation should succeed");

        assert!(!blocked);
        assert!(ignored);
    }

    #[test]
    fn an_ignored_block_does_not_count_toward_quorum() {
        let blocker = NativeUuid::new_v4();
        let votes = vec![
            vote(NativeUuid::new_v4(), VoteType::Agree),
            vote(blocker, VoteType::Block),
        ];
        let mut config =
            config(PollDecisionMakingModel::Consensus, Some(true), None);
        config.quorum_enabled = Some(true);
        config.quorum_threshold = Some(50);

        let votes = filter_arithmetic_votes(votes, &HashSet::from([blocker]));
        let has_consensus = has_consensus(&votes, &config, 4, timestamp(1))
            .expect("consensus evaluation should succeed");

        assert!(!has_consensus);
    }
}
