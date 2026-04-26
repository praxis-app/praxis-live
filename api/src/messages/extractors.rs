use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use entity::messages;
use sea_orm::prelude::Uuid;
use serde::Deserialize;

use super::{handlers::ChatState, service};
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
struct MessageImagePath {
    #[serde(rename = "serverId")]
    server_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "imageId")]
    image_id: String,
}

pub(super) struct ChannelWriteContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) user_id: Uuid,
}

pub(super) struct MessageImageUploadContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) image_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) message: messages::Model,
}

impl FromRequestParts<ChatState> for ChannelWriteContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ChatState,
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

impl FromRequestParts<ChatState> for MessageImageUploadContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ChatState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<MessageImagePath>::from_request_parts(parts, state)
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
        let message_id = parse_uuid(&path.message_id, "messageId")?;
        let image_id = parse_uuid(&path.image_id, "imageId")?;
        let message = service::load_message(
            &state.database,
            server_id,
            channel_id,
            message_id,
        )
        .await?;

        channels::ensure_channel_membership(
            &state.database,
            channel_id,
            user_id,
        )
        .await?;
        if message.user_id != user_id {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
        }

        Ok(Self {
            server_id,
            channel_id,
            image_id,
            user_id,
            message,
        })
    }
}
