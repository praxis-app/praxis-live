use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::{
    auth::{AuthenticatedUser, HasJwtSecret},
    common::AppResult,
};

#[derive(Clone, Debug)]
pub(super) struct CapabilitiesState {
    jwt_secret: Arc<str>,
    video_calls_enabled: bool,
}

impl CapabilitiesState {
    pub(super) fn new(jwt_secret: String, video_calls_enabled: bool) -> Self {
        Self {
            jwt_secret: Arc::<str>::from(jwt_secret),
            video_calls_enabled,
        }
    }
}

impl HasJwtSecret for CapabilitiesState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_capabilities(
    State(state): State<CapabilitiesState>,
    AuthenticatedUser(_user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "videoCallsEnabled": state.video_calls_enabled,
    })))
}
