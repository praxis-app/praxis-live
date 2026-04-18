use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, Response, StatusCode},
    response::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

use super::{models::UpdateUserProfileRequest, service};
use crate::{
    auth::{AuthenticatedUser, AuthenticatedUserOptional, HasJwtSecret},
    common::{
        request::{multipart_file, parse_uuid},
        ApiError, AppResult,
    },
    servers,
};

#[derive(Clone, Debug)]
pub(super) struct UsersState {
    database: DatabaseConnection,
    jwt_secret: Arc<str>,
    upload_root: Arc<PathBuf>,
}

impl UsersState {
    pub(super) fn new(
        database: DatabaseConnection,
        jwt_secret: String,
    ) -> Self {
        Self {
            database,
            jwt_secret: Arc::<str>::from(jwt_secret),
            upload_root: Arc::new(service::upload_root()),
        }
    }
}

impl HasJwtSecret for UsersState {
    fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

pub(super) async fn get_current_user(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let user = service::get_current_user(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({ "user": user })))
}

pub(super) async fn get_current_user_servers(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> AppResult<Json<serde_json::Value>> {
    let servers =
        servers::service::get_servers_for_user(&state.database, user_id)
            .await?;
    Ok(Json(serde_json::json!({ "servers": servers })))
}

pub(super) async fn is_first_user(
    State(state): State<UsersState>,
) -> AppResult<Json<serde_json::Value>> {
    let is_first_user = service::is_first_user(&state.database).await?;
    Ok(Json(serde_json::json!({ "isFirstUser": is_first_user })))
}

pub(super) async fn get_user_profile(
    State(state): State<UsersState>,
    Path(user_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = parse_uuid(&user_id, "userId")?;
    let user = service::get_user_profile(&state.database, user_id).await?;

    Ok(Json(serde_json::json!({ "user": user })))
}

pub(super) async fn update_user_profile(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UpdateUserProfileRequest>,
) -> AppResult<StatusCode> {
    service::update_user_profile(&state.database, user_id, payload).await?;
    Ok(StatusCode::OK)
}

pub(super) async fn upload_user_profile_picture(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    upload_user_image(state, user_id, "profile-picture", multipart).await
}

pub(super) async fn upload_user_cover_photo(
    State(state): State<UsersState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    upload_user_image(state, user_id, "cover-photo", multipart).await
}

#[derive(Debug, Deserialize)]
pub(super) struct UserImagePath {
    #[serde(rename = "userId")]
    user_id: String,
    #[serde(rename = "imageId")]
    image_id: String,
}

pub(super) async fn get_user_image(
    State(state): State<UsersState>,
    Path(path): Path<UserImagePath>,
    AuthenticatedUserOptional(current_user_id): AuthenticatedUserOptional,
) -> AppResult<Response<Body>> {
    let user_id = parse_uuid(&path.user_id, "userId")?;
    let image_id = parse_uuid(&path.image_id, "imageId")?;
    authorize_user_image_access(
        &state.database,
        current_user_id,
        user_id,
        image_id,
    )
    .await?;
    let (content_type, bytes) = service::get_user_image(
        &state.database,
        &state.upload_root,
        user_id,
        image_id,
    )
    .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            content_type
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        )
        .body(Body::from(bytes))
        .map_err(internal_error)
}

async fn upload_user_image(
    state: UsersState,
    user_id: sea_orm::prelude::Uuid,
    kind: &str,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let file = multipart_file(multipart, "file").await?;
    let image = service::store_user_image(
        &state.database,
        &state.upload_root,
        user_id,
        kind,
        file.as_ref().and_then(|file| file.content_type.clone()),
        file.map(|file| file.bytes).unwrap_or_default(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "image": image })),
    ))
}

async fn authorize_user_image_access(
    database: &DatabaseConnection,
    current_user_id: Option<sea_orm::prelude::Uuid>,
    user_id: sea_orm::prelude::Uuid,
    image_id: sea_orm::prelude::Uuid,
) -> AppResult<()> {
    let profile_picture =
        service::get_user_profile_picture(database, user_id).await?;
    if profile_picture
        .as_ref()
        .is_some_and(|image| image.id == image_id.to_string())
    {
        let allowed = current_user_id == Some(user_id)
            || service::is_default_server_member(database, user_id).await?;
        return if allowed {
            Ok(())
        } else {
            Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
        };
    }

    let cover_photo = service::get_user_cover_photo(database, user_id).await?;
    if cover_photo
        .as_ref()
        .is_some_and(|image| image.id == image_id.to_string())
    {
        let allowed = if current_user_id == Some(user_id) {
            true
        } else if let Some(current_user_id) = current_user_id {
            service::has_shared_channel(database, current_user_id, user_id)
                .await?
        } else {
            false
        };
        return if allowed {
            Ok(())
        } else {
            Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
        };
    }

    Err(ApiError::new(StatusCode::NOT_FOUND, "Image not found."))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("user route failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
