use axum::{
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
};

use super::types::InviteAccessQuery;
use crate::common::ApiError;

const INVITE_TOKEN_HEADER: &str = "x-invite-token";

pub(crate) struct InviteAccessToken(pub(crate) Option<String>);

impl<S> FromRequestParts<S> for InviteAccessToken
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        if let Some(value) = parts.headers.get(INVITE_TOKEN_HEADER) {
            let token = value.to_str().map_err(|_| invalid_invite_token())?;
            return Ok(Self(Some(token.to_owned())));
        }

        let Query(query) =
            Query::<InviteAccessQuery>::from_request_parts(parts, state)
                .await
                .map_err(|_| invalid_invite_token())?;
        Ok(Self(query.invite_token))
    }
}

fn invalid_invite_token() -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "Invalid invite token.")
}
