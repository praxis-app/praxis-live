use axum::{extract::Multipart, http::StatusCode};
use sea_orm::prelude::Uuid;

use crate::common::{ApiError, AppResult};

pub(crate) struct MultipartFile {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) async fn multipart_file(
    mut multipart: Multipart,
    field_name: &str,
) -> AppResult<Option<MultipartFile>> {
    while let Some(field) =
        multipart.next_field().await.map_err(internal_error)?
    {
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
    value.parse().map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("{field} must be a UUID."),
        )
    })
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("multipart request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
