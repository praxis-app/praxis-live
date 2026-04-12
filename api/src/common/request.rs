use axum::{
    extract::{FromRequestParts, Multipart},
    http::{header, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::prelude::Uuid;
use serde::Deserialize;

use crate::messages::types::{ApiError, AppResult};

pub(crate) trait HasJwtSecret {
    fn jwt_secret(&self) -> &str;
}

pub(crate) struct AuthenticatedUser(pub(crate) Uuid);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: HasJwtSecret + Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)
            .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))?;

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret().as_bytes()),
            &Validation::default(),
        )
        .ok()
        .and_then(|claims| claims.claims.sub.parse::<Uuid>().ok())
        .map(Self)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required."))
    }
}

pub(crate) struct MultipartFile {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) async fn multipart_file(
    mut multipart: Multipart,
    field_name: &str,
) -> AppResult<Option<MultipartFile>> {
    while let Some(field) = multipart.next_field().await.map_err(internal_error)? {
        if field.name() == Some(field_name) {
            let content_type = field.content_type().map(ToOwned::to_owned);
            let bytes = field.bytes().await.map_err(internal_error)?.to_vec();
            return Ok(Some(MultipartFile {
                content_type,
                bytes,
            }));
        }
    }

    Ok(None)
}

pub(crate) fn parse_uuid(value: &str, field: &str) -> AppResult<Uuid> {
    value
        .parse()
        .map_err(|_| ApiError::new(StatusCode::BAD_REQUEST, format!("{field} must be a UUID.")))
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let header_value = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = header_value.split_once(' ')?;

    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("multipart request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
}
