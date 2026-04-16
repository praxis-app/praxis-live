use axum::http::StatusCode;
use entity::{
    server_role_members, server_role_permissions, server_roles, users,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    Set, SqlErr,
};
use std::collections::BTreeMap;
use uuid::Uuid as NativeUuid;

use super::types::{RoleRequest, ServerRoleResponse};
use crate::{
    common::{
        roles::{
            validate_permissions, PermissionMap, PermissionRule,
            ADMIN_ROLE_NAME, DEFAULT_ROLE_COLOR,
        },
        ApiError, AppResult,
    },
    servers::{self, types::UserResponse},
};

const SERVER_SUBJECTS: &[&str] = &[
    "ServerConfig",
    "Channel",
    "Invite",
    "Message",
    "ServerRole",
    "all",
];

pub(crate) async fn get_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
) -> AppResult<ServerRoleResponse> {
    let role = load_server_role(database, server_id, role_id).await?;
    shape_server_role(database, role).await
}

pub(crate) async fn get_server_roles(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<ServerRoleResponse>> {
    servers::service::ensure_server(database, server_id).await?;
    let roles = server_roles::Entity::find()
        .filter(server_roles::Column::ServerId.eq(server_id))
        .order_by_asc(server_roles::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut responses = Vec::with_capacity(roles.len());
    for role in roles {
        responses.push(shape_server_role(database, role).await?);
    }
    Ok(responses)
}

pub(crate) async fn get_permissions_by_user(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<PermissionMap> {
    let memberships = server_role_members::Entity::find()
        .filter(server_role_members::Column::UserId.eq(user_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let role_ids: Vec<Uuid> =
        memberships.iter().map(|item| item.server_role_id).collect();
    if role_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let roles = server_roles::Entity::find()
        .filter(server_roles::Column::Id.is_in(role_ids))
        .all(database)
        .await
        .map_err(internal_error)?;
    let role_ids: Vec<Uuid> = roles.iter().map(|role| role.id).collect();
    let permissions = server_role_permissions::Entity::find()
        .filter(server_role_permissions::Column::ServerRoleId.is_in(role_ids))
        .all(database)
        .await
        .map_err(internal_error)?;

    let role_server_ids: BTreeMap<Uuid, Uuid> = roles
        .into_iter()
        .map(|role| (role.id, role.server_id))
        .collect();
    let mut raw: BTreeMap<String, Vec<server_role_permissions::Model>> =
        BTreeMap::new();
    for permission in permissions {
        if let Some(server_id) = role_server_ids.get(&permission.server_role_id)
        {
            raw.entry(server_id.to_string())
                .or_default()
                .push(permission);
        }
    }

    Ok(raw
        .into_iter()
        .map(|(server_id, permissions)| {
            (server_id, group_permissions(permissions))
        })
        .collect())
}

pub(crate) async fn get_users_eligible_for_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
) -> AppResult<Vec<UserResponse>> {
    load_server_role(database, server_id, role_id).await?;
    let memberships = server_role_members::Entity::find()
        .filter(server_role_members::Column::ServerRoleId.eq(role_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let member_ids: Vec<Uuid> =
        memberships.iter().map(|item| item.user_id).collect();

    let mut query = users::Entity::find();
    if !member_ids.is_empty() {
        query = query.filter(users::Column::Id.is_not_in(member_ids));
    }

    let users = query.all(database).await.map_err(internal_error)?;
    Ok(users.into_iter().map(shape_user).collect())
}

pub(crate) async fn create_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    request: RoleRequest,
) -> AppResult<ServerRoleResponse> {
    servers::service::ensure_server(database, server_id).await?;
    let (name, color) = validate_role_request(request)?;
    let role = server_roles::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        name: Set(name),
        color: Set(color),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(map_write_error)?;

    shape_server_role(database, role).await
}

pub(crate) async fn create_admin_server_role<C>(
    database: &C,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let role = server_roles::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        name: Set(ADMIN_ROLE_NAME.to_owned()),
        color: Set(DEFAULT_ROLE_COLOR.to_owned()),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(map_write_error)?;

    set_permissions(
        database,
        role.id,
        &[
            PermissionRule {
                subject: "ServerConfig".to_owned(),
                action: vec!["manage".to_owned()],
            },
            PermissionRule {
                subject: "Channel".to_owned(),
                action: vec!["manage".to_owned()],
            },
            PermissionRule {
                subject: "Invite".to_owned(),

                // TODO: Remove redundant `create` once the frontend treats `manage`
                // as satisfying narrower invite permission checks
                action: vec!["create".to_owned(), "manage".to_owned()],
            },
            PermissionRule {
                subject: "ServerRole".to_owned(),
                action: vec!["manage".to_owned()],
            },
        ],
    )
    .await?;
    add_member(database, role.id, user_id).await
}

pub(crate) async fn update_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
    request: RoleRequest,
) -> AppResult<()> {
    let (name, color) = validate_role_request(request)?;
    let role = load_server_role(database, server_id, role_id).await?;
    let mut active = role.into_active_model();
    active.name = Set(name);
    active.color = Set(color);
    active.update(database).await.map_err(map_write_error)?;
    Ok(())
}

pub(crate) async fn update_server_role_permissions(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
    permissions: Vec<PermissionRule>,
) -> AppResult<()> {
    validate_permissions(&permissions, SERVER_SUBJECTS).await?;
    load_server_role(database, server_id, role_id).await?;
    set_permissions(database, role_id, &permissions).await
}

pub(crate) async fn add_server_role_members(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
    user_ids: &[Uuid],
) -> AppResult<()> {
    load_server_role(database, server_id, role_id).await?;
    for user_id in user_ids {
        if users::Entity::find_by_id(*user_id)
            .one(database)
            .await
            .map_err(internal_error)?
            .is_some()
        {
            add_member(database, role_id, *user_id).await?;
        }
    }
    Ok(())
}

pub(crate) async fn remove_server_role_member(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    load_server_role(database, server_id, role_id).await?;
    server_role_members::Entity::delete_many()
        .filter(server_role_members::Column::ServerRoleId.eq(role_id))
        .filter(server_role_members::Column::UserId.eq(user_id))
        .exec(database)
        .await
        .map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn delete_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
) -> AppResult<()> {
    let role = load_server_role(database, server_id, role_id).await?;
    server_roles::Entity::delete_by_id(role.id)
        .exec(database)
        .await
        .map_err(internal_error)?;
    Ok(())
}

async fn shape_server_role(
    database: &DatabaseConnection,
    role: server_roles::Model,
) -> AppResult<ServerRoleResponse> {
    let permissions = server_role_permissions::Entity::find()
        .filter(server_role_permissions::Column::ServerRoleId.eq(role.id))
        .order_by_asc(server_role_permissions::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let members = get_role_members(database, role.id).await?;
    let member_count = members.len();
    Ok(ServerRoleResponse {
        id: role.id.to_string(),
        name: role.name,
        color: role.color,
        permissions: group_permissions(permissions),
        member_count,
        members,
    })
}

async fn get_role_members(
    database: &DatabaseConnection,
    role_id: Uuid,
) -> AppResult<Vec<UserResponse>> {
    let memberships = server_role_members::Entity::find()
        .filter(server_role_members::Column::ServerRoleId.eq(role_id))
        .order_by_asc(server_role_members::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let user_ids: Vec<Uuid> =
        memberships.iter().map(|item| item.user_id).collect();
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids))
        .all(database)
        .await
        .map_err(internal_error)?;
    Ok(users.into_iter().map(shape_user).collect())
}

fn group_permissions(
    permissions: Vec<server_role_permissions::Model>,
) -> Vec<PermissionRule> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for permission in permissions {
        let actions = grouped.entry(permission.subject).or_default();
        if !actions.contains(&permission.action) {
            actions.push(permission.action);
        }
    }
    grouped
        .into_iter()
        .map(|(subject, action)| PermissionRule { subject, action })
        .collect()
}

async fn set_permissions<C>(
    database: &C,
    role_id: Uuid,
    permissions: &[PermissionRule],
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    server_role_permissions::Entity::delete_many()
        .filter(server_role_permissions::Column::ServerRoleId.eq(role_id))
        .exec(database)
        .await
        .map_err(internal_error)?;

    for permission in permissions {
        for action in &permission.action {
            server_role_permissions::ActiveModel {
                id: Set(NativeUuid::new_v4()),
                server_role_id: Set(role_id),
                subject: Set(permission.subject.clone()),
                action: Set(action.clone()),
                ..Default::default()
            }
            .insert(database)
            .await
            .map_err(map_write_error)?;
        }
    }
    Ok(())
}

async fn add_member<C>(
    database: &C,
    role_id: Uuid,
    user_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let exists = server_role_members::Entity::find()
        .filter(server_role_members::Column::ServerRoleId.eq(role_id))
        .filter(server_role_members::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some();
    if exists {
        return Ok(());
    }

    server_role_members::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_role_id: Set(role_id),
        user_id: Set(user_id),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;
    Ok(())
}

async fn load_server_role(
    database: &DatabaseConnection,
    server_id: Uuid,
    role_id: Uuid,
) -> AppResult<server_roles::Model> {
    server_roles::Entity::find_by_id(role_id)
        .filter(server_roles::Column::ServerId.eq(server_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Server role not found.")
        })
}

fn validate_role_request(request: RoleRequest) -> AppResult<(String, String)> {
    let name = request.name.trim().to_owned();
    let color = request.color.trim().to_owned();
    if !(2..=30).contains(&name.chars().count()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Role name must be between 2 and 30 characters.",
        ));
    }
    if color.is_empty() || color.chars().count() > 32 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Role color is invalid.",
        ));
    }
    Ok((name, color))
}

fn shape_user(user: users::Model) -> UserResponse {
    UserResponse {
        id: user.id.to_string(),
        name: user.name,
        display_name: None,
        profile_picture: None,
    }
}

fn map_write_error(error: sea_orm::DbErr) -> ApiError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return ApiError::new(StatusCode::CONFLICT, "Role already exists.");
    }
    internal_error(error)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("server role request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
