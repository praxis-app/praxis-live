use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset};
use sea_orm::prelude::Uuid;

use crate::{
    calls,
    common::{ApiError, AppResult},
    messages, polls,
};

pub(crate) async fn get_channel_feed(
    database: &sea_orm::DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    offset: u64,
    limit: u64,
    user_id: Option<Uuid>,
) -> AppResult<Vec<serde_json::Value>> {
    let fetch_limit = offset.saturating_add(limit);
    let messages = messages::get_channel_message_feed(
        database,
        server_id,
        channel_id,
        0,
        fetch_limit,
    )
    .await?;
    let polls = polls::service::get_inline_polls(
        database,
        server_id,
        channel_id,
        0,
        fetch_limit,
        user_id,
    )
    .await?;
    let calls = calls::service::get_channel_call_artifacts(
        database,
        server_id,
        channel_id,
        0,
        fetch_limit,
    )
    .await?;

    let mut feed = messages
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    append_polls(&mut feed, polls)?;
    for call in calls {
        feed.push(serde_json::to_value(call).map_err(internal_error)?);
    }

    Ok(sort_and_page_feed(feed, offset, limit))
}

pub(crate) async fn get_call_feed(
    database: &sea_orm::DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    call_id: Uuid,
    offset: u64,
    limit: u64,
    user_id: Option<Uuid>,
) -> AppResult<Vec<serde_json::Value>> {
    let fetch_limit = offset.saturating_add(limit);
    let messages = messages::get_call_message_feed(
        database,
        server_id,
        channel_id,
        call_id,
        0,
        fetch_limit,
    )
    .await?;
    let polls = polls::service::get_inline_call_polls(
        database,
        server_id,
        channel_id,
        call_id,
        0,
        fetch_limit,
        user_id,
    )
    .await?;

    let mut feed = messages
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    append_polls(&mut feed, polls)?;

    Ok(sort_and_page_feed(feed, offset, limit))
}

fn append_polls(
    feed: &mut Vec<serde_json::Value>,
    polls: Vec<polls::types::PollResponse>,
) -> AppResult<()> {
    for poll in polls {
        let mut value = serde_json::to_value(poll).map_err(internal_error)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "type".to_owned(),
                serde_json::Value::String("poll".to_owned()),
            );
        }
        feed.push(value);
    }

    Ok(())
}

fn sort_and_page_feed(
    mut feed: Vec<serde_json::Value>,
    offset: u64,
    limit: u64,
) -> Vec<serde_json::Value> {
    feed.sort_by(|left, right| {
        timestamp_millis(right)
            .cmp(&timestamp_millis(left))
            .then_with(|| id_string(right).cmp(&id_string(left)))
    });

    feed.into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect()
}

fn timestamp_millis(value: &serde_json::Value) -> i64 {
    value
        .get("createdAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|timestamp| {
            DateTime::<FixedOffset>::parse_from_rfc3339(timestamp).ok()
        })
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_default()
}

fn id_string(value: &serde_json::Value) -> String {
    value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("feed request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
