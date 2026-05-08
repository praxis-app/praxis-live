use axum::http::StatusCode;
use chrono::{Duration, Utc};
use entity::{calls, users};
use livekit_api::{
    access_token::{AccessToken, VideoGrants},
    services::room::RoomClient,
};
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set, SqlErr,
    TransactionTrait,
};
use std::env;
use tokio::time::{self, MissedTickBehavior};

use super::types::{CallResponse, JoinCallResponse, StartCallResponse};
use crate::common::{ApiError, AppResult};

const TOKEN_TTL_MINUTES: i64 = 30;
const ACTIVE_STATUSES: [&str; 2] = ["starting", "active"];
const STALE_CALL_CLEANUP_INTERVAL_SECONDS: u64 = 60 * 5;
const STALE_STARTING_MINUTES: i64 = 15;
const STALE_ACTIVE_HOURS: i64 = 24;
const STALE_ENDING_MINUTES: i64 = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct StaleCallCleanupSummary {
    failed_starting: u64,
    ended_active: u64,
    ended_ending: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct LiveKitConfig {
    pub(crate) url: String,
    api_url: String,
    api_key: String,
    api_secret: String,
}

impl LiveKitConfig {
    pub(crate) fn from_env() -> Option<Self> {
        let url = livekit_url_from_env()?;
        let api_key = env::var("LIVEKIT_API_KEY").ok()?;
        let api_secret = env::var("LIVEKIT_API_SECRET").ok()?;

        if url.trim().is_empty()
            || api_key.trim().is_empty()
            || api_secret.trim().is_empty()
        {
            return None;
        }

        Some(Self {
            api_url: livekit_api_url(&url),
            url,
            api_key,
            api_secret,
        })
    }
}

pub(crate) fn spawn_stale_call_cleaner(database: DatabaseConnection) {
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
                        || summary.ended_active > 0
                        || summary.ended_ending > 0 =>
                {
                    tracing::info!(
                        failed_starting = summary.failed_starting,
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
        }
    });
}

fn livekit_url_from_env() -> Option<String> {
    let host = env::var("LIVEKIT_HOST").ok()?;
    let port = env::var("LIVEKIT_PORT").ok()?;

    if host.trim().is_empty() || port.trim().is_empty() {
        return None;
    }

    Some(format!("ws://{host}:{port}"))
}

pub(crate) async fn start_channel_call(
    database: &DatabaseConnection,
    server_id: uuid::Uuid,
    channel_id: uuid::Uuid,
    user_id: uuid::Uuid,
) -> AppResult<StartCallResponse> {
    let call =
        get_or_create_channel_call(database, server_id, channel_id, user_id)
            .await?;

    Ok(StartCallResponse {
        call: shape_call(call),
    })
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
        livekit_room_participant_count(livekit, &call.livekit_room).await?;

    if active_participants > 0 {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(shape_call(call));
    }

    let mut active_call = call.into_active_model();
    active_call.status = Set("ended".to_owned());
    active_call.ended_by = Set(Some(user_id));
    active_call.ended_reason = Set(Some("last_participant_left".to_owned()));
    active_call.updated_at = Set(Utc::now().fixed_offset());

    let call = active_call
        .update(&transaction)
        .await
        .map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    Ok(shape_call(call))
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
    RoomClient::with_api_key(
        &livekit.api_url,
        &livekit.api_key,
        &livekit.api_secret,
    )
    .list_participants(room_name)
    .await
    .map(|participants| participants.len())
    .map_err(internal_error)
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
        ended_active,
        ended_ending,
    })
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
