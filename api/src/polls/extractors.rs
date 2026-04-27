use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use entity::polls;
use sea_orm::prelude::Uuid;

use super::{
    handlers::PollsState,
    service,
    types::{PollImagePath, PollPath},
};
use crate::{auth::AuthenticatedUser, channels, common::ApiError};

pub(super) struct PollImageUploadContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) image_id: Uuid,
    pub(super) user_id: Uuid,
    pub(super) poll: polls::Model,
}

pub(super) struct PollDeleteContext {
    pub(super) poll: polls::Model,
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
        let poll = service::load_poll(
            &state.database,
            path.server_id,
            path.channel_id,
            path.poll_id,
        )
        .await?;

        ensure_poll_owner(state, path.channel_id, user_id, &poll).await?;

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            image_id: path.image_id,
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
        let poll = service::load_poll(
            &state.database,
            path.server_id,
            path.channel_id,
            path.poll_id,
        )
        .await?;

        ensure_poll_owner(state, path.channel_id, user_id, &poll).await?;

        Ok(Self { poll })
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
