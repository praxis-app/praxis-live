use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::{prelude::Uuid, DatabaseConnection};

use super::types::ChannelPath;
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::ApiError,
};

pub(crate) trait HasDatabase {
    fn database(&self) -> &DatabaseConnection;
}

pub(crate) struct ChannelWriteContext {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) user_id: Uuid,
}

impl<S> FromRequestParts<S> for ChannelWriteContext
where
    S: HasDatabase + HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<ChannelPath>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::BAD_REQUEST, "Invalid route path.")
            })?;
        let AuthenticatedUser(user_id) =
            AuthenticatedUser::from_request_parts(parts, state).await?;

        super::get_channel(state.database(), path.server_id, path.channel_id)
            .await?;
        super::is_channel_member(state.database(), path.channel_id, user_id)
            .await?;

        Ok(Self {
            server_id: path.server_id,
            channel_id: path.channel_id,
            user_id,
        })
    }
}
