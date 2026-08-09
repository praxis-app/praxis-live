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

pub(crate) struct JsonOrMultipartFiles<T> {
    pub(crate) payload: T,
    pub(crate) file: Option<MultipartFile>,
    pub(crate) files: Vec<MultipartFile>,
}

impl<T, S> FromRequest<S> for JsonOrMultipartFiles<T>
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
            let (payload, file, files) =
                multipart_json_files(multipart).await?;
            return Ok(Self {
                payload,
                file,
                files,
            });
        }

        let Json(payload) = Json::<T>::from_request(request, state)
            .await
            .map_err(|error| {
                ApiError::new(error.status(), error.body_text())
            })?;
        Ok(Self {
            payload,
            file: None,
            files: vec![],
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

async fn multipart_json_files<T: DeserializeOwned>(
    mut multipart: Multipart,
) -> AppResult<(T, Option<MultipartFile>, Vec<MultipartFile>)> {
    let mut payload = None;
    let mut file = None;
    let mut files = vec![];

    while let Some(field) =
        multipart.next_field().await.map_err(internal_error)?
    {
        match field.name() {
            Some("payload") => {
                let text = field.text().await.map_err(internal_error)?;
                payload = Some(serde_json::from_str(&text).map_err(|_| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "Invalid multipart JSON payload.",
                    )
                })?);
            }
            Some("file") => {
                let content_type = field.content_type().map(ToOwned::to_owned);
                let bytes =
                    field.bytes().await.map_err(internal_error)?.to_vec();
                file = Some(MultipartFile {
                    content_type,
                    bytes,
                });
            }
            Some("files") => {
                let content_type = field.content_type().map(ToOwned::to_owned);
                let bytes =
                    field.bytes().await.map_err(internal_error)?.to_vec();
                files.push(MultipartFile {
                    content_type,
                    bytes,
                });
            }
            _ => {}
        }
    }

    let payload = payload.ok_or_else(|| {
        ApiError::new(StatusCode::BAD_REQUEST, "Multipart payload is required.")
    })?;
    if file.is_none() && files.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "At least one multipart file is required.",
        ));
    }
    Ok((payload, file, files))
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
