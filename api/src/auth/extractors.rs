use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::prelude::Uuid;

use super::types::Claims;
use crate::common::ApiError;

pub(crate) trait HasJwtSecret {
    fn jwt_secret(&self) -> &str;
}

// TODO: Many handlers only need this as an auth gate and discard the id
// (`AuthenticatedUser(_user_id)`). Consider a zero-sized `RequireAuthenticatedUser`
// extractor for those cases so the binding doesn't imply an unused identity.
//
// This extractor validates a JWT, not whether the account is non-anonymous.
// Anonymous users receive valid JWTs when anonymous access is enabled for the
// relevant server, so they also satisfy `AuthenticatedUser`. Handlers that need
// a registered user or a permission boundary must enforce that separately.
pub(crate) struct AuthenticatedUser(pub(crate) Uuid);
pub(crate) struct AuthenticatedUserOptional(pub(crate) Option<Uuid>);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })?;

        authenticate_token(token, state.jwt_secret()).map(Self)
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUserOptional
where
    S: HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Some(token) = bearer_token(parts) else {
            return Ok(Self(None));
        };

        match authenticate_token(token, state.jwt_secret()) {
            Ok(user_id) => Ok(Self(Some(user_id))),
            Err(_) => Ok(Self(None)),
        }
    }
}

pub(crate) fn authenticate_token(
    token: &str,
    jwt_secret: &str,
) -> Result<Uuid, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()
    .and_then(|claims| claims.claims.sub.parse::<Uuid>().ok())
    .ok_or_else(|| {
        ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
    })
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let header_value =
        parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header_value.split_once(' ')?;

    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}
