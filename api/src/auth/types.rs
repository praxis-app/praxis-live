use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::users::PublicUser;

pub(super) type AppResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize)]
pub(super) struct SignupRequest {
    pub(super) email: String,
    pub(super) name: String,
    pub(super) password: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct LoginRequest {
    pub(super) email: String,
    pub(super) password: String,
}

#[derive(Debug)]
pub(super) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub(super) fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SessionResponse {
    pub(super) user: Option<PublicUser>,
    #[serde(rename = "access_token")]
    pub(super) access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Claims {
    pub(super) sub: String,
    pub(super) exp: u64,
}
