use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{calls, enums::PollType, messages, polls, users};
use livekit_api::{
    access_token::{AccessToken, TokenVerifier, VideoGrants},
    services::{room::RoomClient, ServiceError, TwirpError, TwirpErrorCode},
    webhooks::WebhookReceiver,
};
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, SqlErr, TransactionTrait,
};
use std::collections::BTreeSet;
use std::env;
use tokio::time::{self, MissedTickBehavior};

use super::types::{
    CallArtifactResponse, CallResponse, CallSummaryResponse, CallUserResponse,
    JoinCallResponse,
};
use crate::{
    common::{ApiError, AppResult},
    messages::types::serialize_timestamp,
    users as users_service,
};

const TOKEN_TTL_MINUTES: i64 = 30;
const ACTIVE_STATUSES: [&str; 2] = ["starting", "active"];
const STALE_CALL_CLEANUP_INTERVAL_SECONDS: u64 = 10;
const STALE_STARTING_MINUTES: i64 = 15;
const STALE_ACTIVE_HOURS: i64 = 24;
const STALE_ENDING_MINUTES: i64 = 5;
const EMPTY_ROOM_CLEANUP_GRACE_SECONDS: i64 = 30;
const LEAVE_PARTICIPANT_SETTLE_MILLIS: u64 = 500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct StaleCallCleanupSummary {
    failed_starting: u64,
    ended_empty: u64,
    ended_active: u64,
    ended_ending: u64,
}

#[derive(Clone, Debug)]
pub struct LiveKitConfig {
    pub(crate) url: String,
    api_url: String,
    api_key: String,
    api_secret: String,
}

impl LiveKitConfig {
    pub(crate) fn from_env() -> Option<Self> {
        let url = livekit_url_from_env()?;
        let api_url =
            livekit_api_url_from_env().unwrap_or_else(|| livekit_api_url(&url));
        let api_key = env::var("LIVEKIT_API_KEY").ok()?;
        let api_secret = env::var("LIVEKIT_API_SECRET").ok()?;

        if url.trim().is_empty()
            || api_key.trim().is_empty()
            || api_secret.trim().is_empty()
        {
            return None;
        }

        Some(Self {
            api_url,
            url,
            api_key,
            api_secret,
        })
    }

    pub(crate) fn webhook_receiver(&self) -> WebhookReceiver {
        WebhookReceiver::new(TokenVerifier::with_api_key(
            &self.api_key,
            &self.api_secret,
        ))
    }
}

pub(crate) fn spawn_stale_call_cleaner(
    database: DatabaseConnection,
    livekit: Option<LiveKitConfig>,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(std::time::Duration::from_secs(
            configured_interval_seconds(
                "STALE_CALL_CLEANUP_INTERVAL_SECONDS",
                STALE_CALL_CLEANUP_INTERVAL_SECONDS,
            ),
        ));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            match cleanup_stale_calls(&database).await {
                Ok(summary)
                    if summary.failed_starting > 0
                        || summary.ended_empty > 0
                        || summary.ended_active > 0
                        || summary.ended_ending > 0 =>
                {
                    tracing::info!(
                        failed_starting = summary.failed_starting,
                        ended_empty = summary.ended_empty,
                        ended_active = summary.ended_active,
                        ended_ending = summary.ended_ending,
                        "Cleaned up stale calls."
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("Failed to clean up stale calls: {error}");
                }
            }

            let Some(livekit) = livekit.as_ref() else {
                continue;
            };

            match cleanup_empty_active_calls(&database, livekit).await {
                Ok(0) => {}
                Ok(ended_empty) => {
                    tracing::info!(
                        ended_empty,
                        "Ended active calls with empty LiveKit rooms."
                    );
                }
                Err(error) => {
                    tracing::warn!("Failed to clean up empty calls: {error}");
                }
            }
        }
    });
}

fn livekit_url_from_env() -> Option<String> {
    if let Ok(url) = env::var("LIVEKIT_URL") {
        if !url.trim().is_empty() {
            return Some(url);
        }
    }

    let host = env::var("LIVEKIT_HOST").ok()?;
    let port = env::var("LIVEKIT_PORT").ok()?;

    if host.trim().is_empty() || port.trim().is_empty() {
        return None;
    }

    Some(format!("ws://{host}:{port}"))
}

fn livekit_api_url_from_env() -> Option<String> {
    env::var("LIVEKIT_API_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

pub(crate) async fn start_channel_call(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<JoinCallResponse> {
    let call =
        get_or_create_channel_call(database, server_id, channel_id, user_id)
            .await?;

    join_channel_call(
        database, livekit, server_id, channel_id, call.id, user_id,
    )
    .await
}

pub(crate) async fn join_channel_call(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<JoinCallResponse> {
    let user = get_user(database, user_id).await?;
    let call =
        activate_channel_call(database, server_id, channel_id, call_id).await?;
    let room_name = call.livekit_room.clone();
    let token = create_livekit_token(livekit, &room_name, &user)?;
    let call = shape_call(call);

    Ok(JoinCallResponse {
        livekit_url: livekit.url.to_owned(),
        room_name,
        token,
        call,
    })
}

pub(crate) async fn leave_channel_call(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<CallResponse> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let call = find_call(&transaction, server_id, channel_id, call_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Call not found.")
        })?;

    if call.status == "ended" || call.status == "failed" {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(shape_call(call));
    }

    let active_participants =
        settled_livekit_room_participant_count(livekit, &call.livekit_room)
            .await?;

    if active_participants > 0 {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(shape_call(call));
    }

    let call =
        end_call(&transaction, call, Some(user_id), "last_participant_left")
            .await?;
    transaction.commit().await.map_err(internal_error)?;

    Ok(shape_call(call))
}

pub(crate) async fn handle_livekit_webhook(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    body: &str,
    authorization: &str,
) -> AppResult<()> {
    let auth_token = authorization
        .strip_prefix("Bearer ")
        .unwrap_or(authorization)
        .trim();
    let event = livekit
        .webhook_receiver()
        .receive(body, auth_token)
        .map_err(|error| {
            tracing::warn!("invalid LiveKit webhook: {error}");
            ApiError::new(StatusCode::UNAUTHORIZED, "Invalid webhook.")
        })?;

    match event.event.as_str() {
        "room_finished" => {
            if let Some(room) = event.room {
                end_active_call_by_room(
                    database,
                    &room.name,
                    "livekit_room_finished",
                )
                .await?;
            }
        }
        "participant_left" | "participant_connection_aborted" => {
            let Some(room) = event.room else {
                return Ok(());
            };

            if room.num_participants > 0 {
                return Ok(());
            }

            end_call_by_room_if_empty(
                database,
                livekit,
                &room.name,
                "livekit_room_empty",
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

fn room_name(
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
) -> String {
    format!("praxis-server-{server_id}-channel-{channel_id}-call-{call_id}")
}

pub(crate) async fn get_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
) -> AppResult<calls::Model> {
    find_call(database, server_id, channel_id, call_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Call not found."))
}

pub(crate) async fn get_channel_call_artifacts(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<CallArtifactResponse>> {
    crate::channels::get_channel(database, server_id, channel_id).await?;

    let calls = calls::Entity::find()
        .filter(calls::Column::ServerId.eq(server_id))
        .filter(calls::Column::ChannelId.eq(channel_id))
        .order_by_desc(calls::Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut artifacts = Vec::with_capacity(calls.len());
    for call in calls {
        artifacts.push(shape_call_artifact(database, call).await?);
    }

    Ok(artifacts)
}

async fn find_call<C>(
    database: &C,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
) -> Result<Option<calls::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    calls::Entity::find_by_id(call_id)
        .filter(calls::Column::ServerId.eq(server_id))
        .filter(calls::Column::ChannelId.eq(channel_id))
        .one(database)
        .await
}

async fn get_or_create_channel_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<calls::Model> {
    let transaction = database.begin().await.map_err(internal_error)?;

    if let Some(call) = find_active_channel_call(&transaction, channel_id)
        .await
        .map_err(internal_error)?
    {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(call);
    }

    let call_id = uuid::Uuid::new_v4();
    let call = calls::ActiveModel {
        id: Set(call_id),
        server_id: Set(server_id),
        channel_id: Set(channel_id),
        livekit_room: Set(room_name(server_id, channel_id, call_id)),
        status: Set("starting".to_owned()),
        started_by: Set(user_id),
        ..Default::default()
    }
    .insert(&transaction)
    .await;

    match call {
        Ok(call) => {
            transaction.commit().await.map_err(internal_error)?;
            Ok(call)
        }
        Err(error)
            if matches!(
                error.sql_err(),
                Some(SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            transaction.rollback().await.map_err(internal_error)?;
            find_active_channel_call(database, channel_id)
                .await
                .map_err(internal_error)?
                .ok_or_else(|| internal_error("active call conflict"))
        }
        Err(error) => Err(internal_error(error)),
    }
}

async fn find_active_channel_call<C>(
    database: &C,
    channel_id: uuid::Uuid,
) -> Result<Option<calls::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    calls::Entity::find()
        .filter(calls::Column::ChannelId.eq(channel_id))
        .filter(calls::Column::Status.is_in(ACTIVE_STATUSES))
        .one(database)
        .await
}

async fn find_active_call_by_room<C>(
    database: &C,
    room_name: &str,
) -> Result<Option<calls::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    calls::Entity::find()
        .filter(calls::Column::LivekitRoom.eq(room_name))
        .filter(calls::Column::Status.is_in(ACTIVE_STATUSES))
        .one(database)
        .await
}

async fn activate_channel_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    call_id: uuid::Uuid,
) -> AppResult<calls::Model> {
    let call = get_call(database, server_id, channel_id, call_id).await?;

    match call.status.as_str() {
        "starting" => {
            let mut call = call.into_active_model();
            call.status = Set("active".to_owned());
            call.updated_at = Set(Utc::now().fixed_offset());

            call.update(database).await.map_err(internal_error)
        }
        "active" => Ok(call),
        "ending" | "ended" | "failed" => Err(ApiError::new(
            StatusCode::CONFLICT,
            "Call has already ended.",
        )),
        _ => Err(ApiError::new(StatusCode::CONFLICT, "Call is not joinable.")),
    }
}

async fn get_user(
    database: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> AppResult<users::Model> {
    users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })
}

async fn livekit_room_participant_count(
    livekit: &LiveKitConfig,
    room_name: &str,
) -> AppResult<usize> {
    let participants = RoomClient::with_api_key(
        &livekit.api_url,
        &livekit.api_key,
        &livekit.api_secret,
    )
    .list_participants(room_name)
    .await;

    match participants {
        Ok(participants) => Ok(participants.len()),
        Err(error) if is_livekit_not_found(&error) => Ok(0),
        Err(error) => Err(internal_error(error)),
    }
}

fn is_livekit_not_found(error: &ServiceError) -> bool {
    matches!(
        error,
        ServiceError::Twirp(TwirpError::Twirp(code))
            if code.code == TwirpErrorCode::NOT_FOUND
    )
}

async fn settled_livekit_room_participant_count(
    livekit: &LiveKitConfig,
    room_name: &str,
) -> AppResult<usize> {
    let count = livekit_room_participant_count(livekit, room_name).await?;

    if count == 0 {
        return Ok(0);
    }

    time::sleep(std::time::Duration::from_millis(
        LEAVE_PARTICIPANT_SETTLE_MILLIS,
    ))
    .await;

    livekit_room_participant_count(livekit, room_name).await
}

async fn end_call<C>(
    database: &C,
    call: calls::Model,
    ended_by: Option<uuid::Uuid>,
    reason: &str,
) -> AppResult<calls::Model>
where
    C: ConnectionTrait,
{
    let mut active_call = call.into_active_model();
    active_call.status = Set("ended".to_owned());
    active_call.ended_by = Set(ended_by);
    active_call.ended_reason = Set(Some(reason.to_owned()));
    active_call.updated_at = Set(Utc::now().fixed_offset());

    active_call.update(database).await.map_err(internal_error)
}

async fn end_active_call_by_room(
    database: &DatabaseConnection,
    room_name: &str,
    reason: &str,
) -> AppResult<bool> {
    let Some(call) = find_active_call_by_room(database, room_name)
        .await
        .map_err(internal_error)?
    else {
        return Ok(false);
    };

    end_call(database, call, None, reason).await?;
    Ok(true)
}

async fn end_call_by_room_if_empty(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
    room_name: &str,
    reason: &str,
) -> AppResult<bool> {
    let participant_count =
        settled_livekit_room_participant_count(livekit, room_name).await?;

    if participant_count > 0 {
        return Ok(false);
    }

    end_active_call_by_room(database, room_name, reason).await
}

fn create_livekit_token(
    livekit: &LiveKitConfig,
    room_name: &str,
    user: &users::Model,
) -> AppResult<String> {
    let name = user
        .display_name
        .clone()
        .unwrap_or_else(|| user.name.clone());

    let metadata = serde_json::json!({
        "userId": user.id,
        "name": user.name,
        "displayName": user.display_name,
    })
    .to_string();

    AccessToken::with_api_key(&livekit.api_key, &livekit.api_secret)
        .with_identity(&user.id.to_string())
        .with_name(&name)
        .with_metadata(&metadata)
        .with_ttl(std::time::Duration::from_secs(
            (TOKEN_TTL_MINUTES * 60) as u64,
        ))
        .with_grants(VideoGrants {
            room: room_name.to_owned(),
            room_join: true,
            can_subscribe: true,
            can_publish: true,
            can_publish_data: false,
            ..Default::default()
        })
        .to_jwt()
        .map_err(internal_error)
}

async fn cleanup_stale_calls(
    database: &DatabaseConnection,
) -> AppResult<StaleCallCleanupSummary> {
    let now = Utc::now().fixed_offset();
    let failed_starting = calls::Entity::update_many()
        .col_expr(calls::Column::Status, Expr::value("failed"))
        .col_expr(calls::Column::EndedReason, Expr::value("stale_starting"))
        .col_expr(calls::Column::UpdatedAt, Expr::value(now))
        .filter(calls::Column::Status.eq("starting"))
        .filter(
            calls::Column::UpdatedAt
                .lt(now - Duration::minutes(STALE_STARTING_MINUTES)),
        )
        .exec(database)
        .await
        .map_err(internal_error)?
        .rows_affected;

    let ended_active = calls::Entity::update_many()
        .col_expr(calls::Column::Status, Expr::value("ended"))
        .col_expr(calls::Column::EndedReason, Expr::value("stale_active"))
        .col_expr(calls::Column::UpdatedAt, Expr::value(now))
        .filter(calls::Column::Status.eq("active"))
        .filter(
            calls::Column::UpdatedAt
                .lt(now - Duration::hours(STALE_ACTIVE_HOURS)),
        )
        .exec(database)
        .await
        .map_err(internal_error)?
        .rows_affected;

    let ended_ending = calls::Entity::update_many()
        .col_expr(calls::Column::Status, Expr::value("ended"))
        .col_expr(calls::Column::EndedReason, Expr::value("stale_ending"))
        .col_expr(calls::Column::UpdatedAt, Expr::value(now))
        .filter(calls::Column::Status.eq("ending"))
        .filter(
            calls::Column::UpdatedAt
                .lt(now - Duration::minutes(STALE_ENDING_MINUTES)),
        )
        .exec(database)
        .await
        .map_err(internal_error)?
        .rows_affected;

    Ok(StaleCallCleanupSummary {
        failed_starting,
        ended_empty: 0,
        ended_active,
        ended_ending,
    })
}

async fn cleanup_empty_active_calls(
    database: &DatabaseConnection,
    livekit: &LiveKitConfig,
) -> AppResult<u64> {
    let active_calls = calls::Entity::find()
        .filter(calls::Column::Status.is_in(ACTIVE_STATUSES))
        .filter(calls::Column::UpdatedAt.lt(Utc::now().fixed_offset()
            - Duration::seconds(EMPTY_ROOM_CLEANUP_GRACE_SECONDS)))
        .all(database)
        .await
        .map_err(internal_error)?;
    let mut ended_empty = 0;

    for call in active_calls {
        let participant_count =
            livekit_room_participant_count(livekit, &call.livekit_room).await?;

        if participant_count > 0 {
            continue;
        }

        end_call(database, call, None, "empty_livekit_room").await?;
        ended_empty += 1;
    }

    Ok(ended_empty)
}

fn shape_call(call: calls::Model) -> CallResponse {
    CallResponse {
        id: call.id.to_string(),
        server_id: call.server_id.to_string(),
        channel_id: call.channel_id.to_string(),
        room_name: call.livekit_room,
        status: call.status,
    }
}

async fn shape_call_artifact(
    database: &DatabaseConnection,
    call: calls::Model,
) -> AppResult<CallArtifactResponse> {
    let message_count = messages::Entity::find()
        .filter(messages::Column::CallId.eq(call.id))
        .count(database)
        .await
        .map_err(internal_error)?;
    let proposal_count = polls::Entity::find()
        .filter(polls::Column::CallId.eq(call.id))
        .filter(polls::Column::PollType.eq(PollType::Proposal))
        .count(database)
        .await
        .map_err(internal_error)?;
    let poll_count = polls::Entity::find()
        .filter(polls::Column::CallId.eq(call.id))
        .filter(polls::Column::PollType.eq(PollType::Poll))
        .count(database)
        .await
        .map_err(internal_error)?;

    let call_messages = messages::Entity::find()
        .filter(messages::Column::CallId.eq(call.id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let call_polls = polls::Entity::find()
        .filter(polls::Column::CallId.eq(call.id))
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut participant_ids = BTreeSet::new();
    participant_ids.insert(call.started_by);
    if let Some(ended_by) = call.ended_by {
        participant_ids.insert(ended_by);
    }
    participant_ids.extend(call_messages.iter().map(|message| message.user_id));
    participant_ids.extend(call_polls.iter().map(|poll| poll.user_id));

    let user_ids = participant_ids.into_iter().collect::<Vec<_>>();
    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.clone()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;

    let participants = users
        .iter()
        .map(|user| shape_call_user(user, &profile_pictures))
        .collect::<Vec<_>>();
    let started_by = users
        .iter()
        .find(|user| user.id == call.started_by)
        .map(|user| shape_call_user(user, &profile_pictures))
        .ok_or_else(|| internal_error("call starter not found"))?;

    let ended_at = (call.status == "ended" || call.status == "failed")
        .then(|| serialize_timestamp(call.updated_at));
    let duration_end = if ended_at.is_some() {
        call.updated_at
    } else {
        Utc::now().fixed_offset()
    };
    let duration_seconds = duration_end
        .signed_duration_since(call.created_at)
        .num_seconds()
        .max(0);

    Ok(CallArtifactResponse {
        kind: "call",
        id: call.id.to_string(),
        server_id: call.server_id.to_string(),
        channel_id: call.channel_id.to_string(),
        room_name: call.livekit_room,
        status: call.status,
        started_by,
        participant_count: participants.len(),
        participants,
        duration_seconds,
        summary: CallSummaryResponse {
            messages: message_count,
            proposals: proposal_count,
            polls: poll_count,
        },
        created_at: serialize_timestamp(call.created_at),
        ended_at,
    })
}

fn shape_call_user(
    user: &users::Model,
    profile_pictures: &std::collections::BTreeMap<
        uuid::Uuid,
        crate::users::UserImageRef,
    >,
) -> CallUserResponse {
    CallUserResponse {
        id: user.id.to_string(),
        name: user.name.clone(),
        display_name: user.display_name.clone(),
        profile_picture: profile_pictures.get(&user.id).cloned(),
    }
}

fn configured_interval_seconds(env_key: &str, default: u64) -> u64 {
    env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn livekit_api_url(livekit_url: &str) -> String {
    livekit_url
        .strip_prefix("wss://")
        .map(|host| format!("https://{host}"))
        .or_else(|| {
            livekit_url
                .strip_prefix("ws://")
                .map(|host| format!("http://{host}"))
        })
        .unwrap_or_else(|| livekit_url.to_owned())
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("call request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
