use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use entity::enums::ChannelType;
use sea_orm::prelude::Uuid;

use super::types::{ForumChannelPath, ForumPostPath, ForumReplyPath};
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    channels::{self, extractors::HasDatabase},
    common::ApiError,
};

pub(super) struct ForumAccessContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) user_id: Uuid,
}

pub(super) struct ForumPostAccessContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) post_id: Uuid,
    pub(super) user_id: Uuid,
}

pub(super) struct ForumReplyAccessContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) post_id: Uuid,
    pub(super) reply_id: Uuid,
    pub(super) user_id: Uuid,
}

impl<S> FromRequestParts<S> for ForumAccessContext
where
    S: HasDatabase + HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<ForumChannelPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid route path.",
                    )
                })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        ensure_forum_access(state, path.server_id, path.channel_id, user_id)
            .await?;

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            user_id,
        })
    }
}

impl<S> FromRequestParts<S> for ForumPostAccessContext
where
    S: HasDatabase + HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<ForumPostPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid route path.",
                    )
                })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        ensure_forum_access(state, path.server_id, path.channel_id, user_id)
            .await?;

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            post_id: path.post_id,
            user_id,
        })
    }
}

impl<S> FromRequestParts<S> for ForumReplyAccessContext
where
    S: HasDatabase + HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<ForumReplyPath>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid route path.",
                    )
                })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;
        ensure_forum_access(state, path.server_id, path.channel_id, user_id)
            .await?;

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            post_id: path.post_id,
            reply_id: path.reply_id,
            user_id,
        })
    }
}

async fn ensure_forum_access<S>(
    state: &S,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError>
where
    S: HasDatabase + Send + Sync,
{
    let channel =
        channels::get_channel(state.database(), server_id, channel_id).await?;
    channels::ensure_channel_membership(state.database(), channel_id, user_id)
        .await?;
    if channel.channel_type == ChannelType::Forum {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Forum channel not found.",
        ))
    }
}
