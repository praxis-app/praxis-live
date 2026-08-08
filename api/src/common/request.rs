use axum::{
    extract::{FromRequest, Multipart, Request},
    http::{header, StatusCode},
    Json,
};
use sea_orm::prelude::Uuid;
use serde::de::DeserializeOwned;

use crate::common::{ApiError, AppResult};

pub(crate) struct MultipartFile {
    pub(crate) content_type: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) struct JsonOrEventCoverPhoto<T> {
    pub(crate) payload: T,
    pub(crate) cover_photo: Option<Vec<u8>>,
}

impl<T, S> FromRequest<S> for JsonOrEventCoverPhoto<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(
        request: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let is_multipart = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("multipart/form-data"));

        if is_multipart {
            let multipart =
                Multipart::from_request(request, state).await.map_err(
                    |error| ApiError::new(error.status(), error.body_text()),
                )?;
            let (payload, cover_photo) =
                multipart_json_file(multipart, "payload", "coverPhoto").await?;
            return Ok(Self {
                payload,
                cover_photo: Some(cover_photo),
            });
        }

        let Json(payload) = Json::<T>::from_request(request, state)
            .await
            .map_err(|error| {
                ApiError::new(error.status(), error.body_text())
            })?;
        Ok(Self {
            payload,
            cover_photo: None,
        })
    }
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

pub(crate) async fn multipart_json_file<T: DeserializeOwned>(
    mut multipart: Multipart,
    json_field_name: &str,
    file_field_name: &str,
) -> AppResult<(T, Vec<u8>)> {
    let mut payload = None;
    let mut file = None;

    while let Some(field) =
        multipart.next_field().await.map_err(internal_error)?
    {
        match field.name() {
            Some(name) if name == json_field_name => {
                let text = field.text().await.map_err(internal_error)?;
                payload = Some(serde_json::from_str(&text).map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid multipart JSON payload.",
                    )
                })?);
            }
            Some(name) if name == file_field_name => {
                file =
                    Some(field.bytes().await.map_err(internal_error)?.to_vec());
            }
            _ => {}
        }
    }

    let payload = payload.ok_or_else(|| {
        ApiError::new(StatusCode::BAD_REQUEST, "Multipart payload is required.")
    })?;
    let file = file.ok_or_else(|| {
        ApiError::new(StatusCode::BAD_REQUEST, "Cover photo is required.")
    })?;
    Ok((payload, file))
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
