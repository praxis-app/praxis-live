//! Owns periodic discovery and processing of polls that need deadline or event
//! lifecycle updates. Outcome evaluation and final state transitions remain in outcome.

use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    enums::{PollActionType, PollStage, PollType},
    poll_action_events, poll_actions as poll_action_entities, poll_configs,
    polls,
};
use sea_orm::{
    prelude::Uuid,
    sea_query::{JoinType, LockType},
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, TransactionTrait,
};
use std::collections::HashMap;
use tokio::time::{self, MissedTickBehavior};

use super::{
    outcome::{
        close_poll, close_poll_with_reason, finalize_ratifiable_proposal,
        is_poll_ratifiable, ProposalFinalization,
    },
    service::broadcast_stored_poll_update,
};
use crate::{
    common::{ApiError, AppResult},
    poll_actions,
    pub_sub::PubSubService,
};

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
                Ok(summary) if summary.ratified > 0 || summary.closed > 0 => {
                    tracing::info!(
                        checked = summary.processed,
                        ratified = summary.ratified,
                        closed = summary.closed,
                        failed = summary.failed,
                        "Synchronized proposals"
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

async fn synchronize_proposals(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
) -> AppResult<ProposalSyncSummary> {
    let mut summary = ProposalSyncSummary::default();
    expire_stale_event_proposals(database, pub_sub_service, &mut summary)
        .await?;
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

async fn expire_stale_event_proposals(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    summary: &mut ProposalSyncSummary,
) -> AppResult<()> {
    let now = Utc::now().fixed_offset();
    let proposals = polls::Entity::find()
        .join(JoinType::InnerJoin, polls::Relation::Action.def())
        .join(
            JoinType::InnerJoin,
            poll_action_entities::Relation::ProposedEvent.def(),
        )
        .filter(polls::Column::PollType.eq(PollType::Proposal))
        .filter(polls::Column::Stage.eq(PollStage::Voting))
        .filter(
            poll_action_entities::Column::ActionType
                .eq(PollActionType::PlanEvent),
        )
        .order_by_asc(poll_action_events::Column::StartsAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    for poll in proposals {
        summary.processed += 1;
        match expire_stale_event_proposal(database, poll.id, now).await {
            Ok(true) => {
                broadcast_stored_poll_update(
                    database,
                    pub_sub_service,
                    &poll,
                    None,
                )
                .await?;
                summary.closed += 1;
            }
            Ok(false) => {}
            Err(error) => {
                summary.failed += 1;
                tracing::warn!(
                    poll_id = %poll.id,
                    "Failed to expire stale event proposal: {error}"
                );
            }
        }
    }

    Ok(())
}

async fn expire_stale_event_proposal(
    database: &DatabaseConnection,
    poll_id: Uuid,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<bool> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let poll = polls::Entity::find_by_id(poll_id)
        .lock(LockType::Update)
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;

    let reason = poll_actions::service::plan_event_closed_reason(
        &transaction,
        poll_id,
        now,
    )
    .await?;
    if poll.stage != PollStage::Voting || reason.is_none() {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(false);
    }

    close_poll_with_reason(&transaction, poll_id, reason).await?;
    transaction.commit().await.map_err(internal_error)?;
    Ok(true)
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
            let finalization = finalize_ratifiable_proposal(
                &transaction,
                poll.id,
                Utc::now().fixed_offset(),
            )
            .await?;
            transaction.commit().await.map_err(internal_error)?;
            return Ok(match finalization {
                ProposalFinalization::Ratified => ProposalSyncAction::Ratify,
                ProposalFinalization::Closed(_) => ProposalSyncAction::Close,
            });
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

fn configured_interval_seconds(env_key: &str, default: u64) -> u64 {
    std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll synchronization failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
