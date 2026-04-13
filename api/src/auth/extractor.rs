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

pub(crate) struct AuthenticatedUser(pub(crate) Uuid);

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

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret().as_bytes()),
            &Validation::default(),
        )
        .ok()
        .and_then(|claims| claims.claims.sub.parse::<Uuid>().ok())
        .map(Self)
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })
    }
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
