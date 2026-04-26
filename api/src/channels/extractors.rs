use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
};
use sea_orm::{prelude::Uuid, DatabaseConnection};

use super::types::ChannelRoutePath;
use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::{request::parse_uuid, ApiError},
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
        let Path(path) =
            Path::<ChannelRoutePath>::from_request_parts(parts, state)
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

        super::get_channel(state.database(), server_id, channel_id).await?;
        super::ensure_channel_membership(state.database(), channel_id, user_id)
            .await?;

        Ok(Self {
            server_id,
            channel_id,
            user_id,
        })
    }
}
