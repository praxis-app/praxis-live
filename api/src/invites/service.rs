use axum::http::StatusCode;
use chrono::Utc;
use entity::{invites, users};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid as NativeUuid;

use super::types::{InviteRequest, InviteResponse, InviteUserResponse};
use crate::{
    common::{roles::PermissionRule, ApiError, AppResult},
    servers::{self, server_roles},
    users as users_service,
};

const INVITES_PAGE_SIZE: usize = 20;

// TODO: This also gates listing invites, so `read` on `Invite` does not grant
// it. Decide whether `get_invites` should check `read` instead.
pub(super) async fn can_create_invites(
    database: &DatabaseConnection,
    user_id: Uuid,
    server_id: Uuid,
) -> AppResult<()> {
    let permissions =
        server_roles::service::get_permissions_by_user(database, user_id)
            .await?;
    let can_create =
        permissions
            .get(&server_id.to_string())
            .is_some_and(|rules| {
                has_invite_permission(rules, &["create", "manage"])
            });

    if can_create {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}

pub(super) async fn can_manage_invites(
    database: &DatabaseConnection,
    user_id: Uuid,
    server_id: Uuid,
) -> AppResult<()> {
    let permissions =
        server_roles::service::get_permissions_by_user(database, user_id)
            .await?;
    let can_manage = permissions
        .get(&server_id.to_string())
        .is_some_and(|rules| has_invite_permission(rules, &["manage"]));

    if can_manage {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}

fn has_invite_permission(
    permissions: &[PermissionRule],
    actions: &[&str],
) -> bool {
    permissions.iter().any(|permission| {
        (permission.subject == "Invite" || permission.subject == "all")
            && permission
                .action
                .iter()
                .any(|action| actions.contains(&action.as_str()))
    })
}

pub(super) async fn is_valid_invite(
    database: &DatabaseConnection,
    token: &str,
) -> AppResult<bool> {
    let invite = invites::Entity::find()
        .filter(invites::Column::Token.eq(token))
        .one(database)
        .await
        .map_err(internal_error)?;

    Ok(invite.as_ref().is_some_and(validate_invite))
}

pub(crate) async fn is_valid_invite_for_server(
    database: &DatabaseConnection,
    token: &str,
    server_id: Uuid,
) -> AppResult<bool> {
    let invite = invites::Entity::find()
        .filter(invites::Column::Token.eq(token))
        .filter(invites::Column::ServerId.eq(server_id))
        .one(database)
        .await
        .map_err(internal_error)?;

    Ok(invite.as_ref().is_some_and(validate_invite))
}

pub(crate) async fn valid_invite_server_id(
    database: &DatabaseConnection,
    token: &str,
) -> AppResult<Option<Uuid>> {
    let invite = invites::Entity::find()
        .filter(invites::Column::Token.eq(token))
        .one(database)
        .await
        .map_err(internal_error)?;

    Ok(invite
        .filter(validate_invite)
        .map(|invite| invite.server_id))
}

pub(crate) async fn get_invite_by_token(
    database: &DatabaseConnection,
    token: &str,
) -> AppResult<invites::Model> {
    let invite = invites::Entity::find()
        .filter(invites::Column::Token.eq(token))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::BAD_REQUEST, "Invalid invite token.")
        })?;

    if validate_invite(&invite) {
        Ok(invite)
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Invalid invite token.",
        ))
    }
}

pub(super) async fn get_valid_invites(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<InviteResponse>> {
    servers::ensure_server(database, server_id).await?;

    let invites = invites::Entity::find()
        .filter(invites::Column::ServerId.eq(server_id))
        .order_by_desc(invites::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut responses = Vec::new();
    for invite in invites {
        if !validate_invite(&invite) {
            continue;
        }
        responses.push(shape_invite(database, invite).await?);
        if responses.len() == INVITES_PAGE_SIZE {
            break;
        }
    }

    Ok(responses)
}

pub(super) async fn create_invite(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
    request: InviteRequest,
) -> AppResult<InviteResponse> {
    servers::ensure_server(database, server_id).await?;
    ensure_user(database, user_id).await?;

    let token = generate_token();
    let invite = invites::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        token: Set(token),
        uses: Set(0),
        max_uses: Set(request.max_uses.filter(|max_uses| *max_uses > 0)),
        user_id: Set(user_id),
        server_id: Set(server_id),
        expires_at: Set(request.expires_at),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    shape_invite(database, invite).await
}

pub(crate) async fn redeem_invite(
    database: &DatabaseConnection,
    token: &str,
) -> AppResult<invites::Model> {
    let invite = get_invite_by_token(database, token).await?;
    let mut active = invite.clone().into_active_model();
    active.uses = Set(invite.uses + 1);
    active.update(database).await.map_err(internal_error)
}

pub(super) async fn delete_invite(
    database: &DatabaseConnection,
    server_id: Uuid,
    invite_id: Uuid,
) -> AppResult<()> {
    invites::Entity::delete_many()
        .filter(invites::Column::Id.eq(invite_id))
        .filter(invites::Column::ServerId.eq(server_id))
        .exec(database)
        .await
        .map_err(internal_error)?;
    Ok(())
}

fn validate_invite(invite: &invites::Model) -> bool {
    let is_expired = invite
        .expires_at
        .is_some_and(|expires_at| Utc::now().fixed_offset() >= expires_at);
    let max_uses_reached = invite
        .max_uses
        .is_some_and(|max_uses| invite.uses >= max_uses);

    !is_expired && !max_uses_reached
}

async fn shape_invite(
    database: &DatabaseConnection,
    invite: invites::Model,
) -> AppResult<InviteResponse> {
    let user = users::Entity::find_by_id(invite.user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "User not found.")
        })?;

    let profile_picture =
        users_service::get_user_profile_picture(database, user.id).await?;

    Ok(InviteResponse {
        id: invite.id.to_string(),
        token: invite.token,
        uses: invite.uses,
        max_uses: invite.max_uses,
        user: InviteUserResponse {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture,
        },
        expires_at: invite.expires_at.map(|value| value.to_rfc3339()),
        created_at: invite.created_at.to_rfc3339(),
    })
}

async fn ensure_user(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<()> {
    users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .map(|_| ())
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "User not found."))
}

fn generate_token() -> String {
    NativeUuid::new_v4().simple().to_string()[..8].to_owned()
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("invites request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
