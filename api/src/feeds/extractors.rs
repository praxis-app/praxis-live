use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::prelude::Uuid;

use crate::{
    auth::{AuthenticatedUserOptional, HasJwtSecret},
    channels::{self, extractors::HasDatabase},
    common::ApiError,
    servers,
};

pub(super) struct ChannelFeedAccessContext {
    pub(super) server_id: Uuid,
    pub(super) channel_id: Uuid,
    pub(super) user_id: Option<Uuid>,
}

impl<S> FromRequestParts<S> for ChannelFeedAccessContext
where
    S: HasDatabase + HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) =
            Path::<channels::types::ChannelPath>::from_request_parts(
                parts, state,
            )
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUserOptional(user_id) =
            AuthenticatedUserOptional::from_request_parts(parts, state).await?;

        channels::get_channel(
            state.database(),
            path.server_id,
            path.channel_id,
        )
        .await?;

        if let Some(user_id) = user_id {
            channels::ensure_channel_membership(
                state.database(),
                path.channel_id,
                user_id,
            )
            .await?;
        } else if servers::default_server_id(state.database()).await?
            != path.server_id
        {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
        }

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            user_id,
        })
    }
}
