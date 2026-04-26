use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use entity::polls;
use sea_orm::prelude::Uuid;
use serde::Deserialize;

use super::{handlers::PollsState, service};
use crate::{
    auth::AuthenticatedUser,
    channels,
    common::{request::parse_uuid, ApiError},
};

#[derive(Debug, Deserialize)]
struct ChannelPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
}

#[derive(Debug, Deserialize)]
struct PollPath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
}

#[derive(Debug, Deserialize)]
struct PollImagePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "pollId")]
    poll_id: String,
    #[serde(rename = "imageId")]
    image_id: String,
}

pub(super) struct ChannelWriteContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) user_id: Uuid,
}

pub(super) struct PollImageUploadContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) image_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) poll: polls::Model,
}

pub(super) struct PollDeleteContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) poll: polls::Model,
}

impl FromRequestParts<PollsState> for ChannelWriteContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &PollsState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<ChannelPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let server_id = parse_uuid(&path.server_id, "serverId")?;
        let channel_id = parse_uuid(&path.channel_id, "channelId")?;

        channels::get_channel(&state.database, server_id, channel_id).await?;
        channels::ensure_channel_membership(
            &state.database,
            channel_id,
            user_id,
        )
        .await?;

        Ok(Self {
            server_id,
            channel_id,
            user_id,
        })
    }
}

impl FromRequestParts<PollsState> for PollImageUploadContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &PollsState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<PollImagePath>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid route path.",
                    )
                })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let server_id = parse_uuid(&path.server_id, "serverId")?;
        let channel_id = parse_uuid(&path.channel_id, "channelId")?;
        let poll_id = parse_uuid(&path.poll_id, "pollId")?;
        let image_id = parse_uuid(&path.image_id, "imageId")?;
        let poll =
            service::load_poll(&state.database, server_id, channel_id, poll_id)
                .await?;

        ensure_poll_owner(state, channel_id, user_id, &poll).await?;

        Ok(Self {
            server_id,
            channel_id,
            image_id,
            user_id,
            poll,
        })
    }
}

impl FromRequestParts<PollsState> for PollDeleteContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &PollsState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<PollPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let server_id = parse_uuid(&path.server_id, "serverId")?;
        let channel_id = parse_uuid(&path.channel_id, "channelId")?;
        let poll_id = parse_uuid(&path.poll_id, "pollId")?;
        let poll =
            service::load_poll(&state.database, server_id, channel_id, poll_id)
                .await?;

        ensure_poll_owner(state, channel_id, user_id, &poll).await?;

        Ok(Self {
            server_id,
            channel_id,
            user_id,
            poll,
        })
    }
}

async fn ensure_poll_owner(
    state: &PollsState,
    channel_id: Uuid,
    user_id: Uuid,
    poll: &polls::Model,
) -> Result<(), ApiError> {
    channels::ensure_channel_membership(&state.database, channel_id, user_id)
        .await?;
    if poll.user_id == user_id {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}
