use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use entity::messages;
use sea_orm::prelude::Uuid;

use super::{
    handlers::ChatState,
    service,
    types::{CallMessageImagePath, MessageImagePath},
};
use crate::{auth::AuthenticatedUser, channels, common::ApiError};

pub(super) struct MessageImageUploadContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) image_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) message: messages::Model,
}

pub(super) struct CallMessageImageUploadContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) call_id: Uuid,
    pub(super) image_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) message: messages::Model,
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
        let message = service::load_message(
            &state.database,
            path.server_id,
            path.channel_id,
            path.message_id,
        )
        .await?;

        channels::ensure_channel_membership(
            &state.database,
            path.channel_id,
            user_id,
        )
        .await?;
        if message.user_id != user_id {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
        }

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            image_id: path.image_id,
            user_id,
            message,
        })
    }
}

impl FromRequestParts<ChatState> for CallMessageImageUploadContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ChatState,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<CallMessageImagePath>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid route path.",
                    )
                })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        let message = service::load_call_message(
            &state.database,
            path.server_id,
            path.channel_id,
            path.call_id,
            path.message_id,
        )
        .await?;

        channels::ensure_channel_membership(
            &state.database,
            path.channel_id,
            user_id,
        )
        .await?;
        if message.user_id != user_id {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
        }

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            call_id: path.call_id,
            image_id: path.image_id,
            user_id,
            message,
        })
    }
}
