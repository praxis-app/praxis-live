use axum::http::StatusCode;
use entity::{
    channels, poll_action_permissions, poll_action_role_members,
    poll_action_roles, poll_actions, polls, server_role_members,
    server_role_permissions, server_roles, users,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, QueryFilter, Set,
};
use uuid::Uuid as NativeUuid;

use crate::{
    common::{request::parse_uuid, ApiError, AppResult},
    poll_actions::types::{
        CreatePollActionRequest, CreatePollActionServerRoleRequest,
        PollActionPermissionResponse, PollActionResponse,
        PollActionServerRoleMemberResponse, PollActionServerRoleResponse,
        PollActionUserResponse,
    },
    users as users_service,
};

pub(crate) async fn create_poll_action(
    database: &DatabaseConnection,
    poll_id: Uuid,
    request: CreatePollActionRequest,
) -> AppResult<poll_actions::Model> {
    let action = poll_actions::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll_id),
        action_type: Set(request.action_type),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    if let Some(server_role) = request.server_role {
        create_poll_action_role(database, action.id, server_role).await?;
    }

    Ok(action)
}

pub(crate) async fn create_poll_action_role(
    database: &DatabaseConnection,
    poll_action_id: Uuid,
    request: CreatePollActionServerRoleRequest,
) -> AppResult<poll_action_roles::Model> {
    let server_role_id = request
        .server_role_to_update_id
        .as_deref()
        .map(|value| parse_uuid(value, "serverRoleToUpdateId"))
        .transpose()?;

    let role_to_update = if let Some(server_role_id) = server_role_id {
        server_roles::Entity::find_by_id(server_role_id)
            .one(database)
            .await
            .map_err(internal_error)?
    } else {
        None
    };

    let name = request.name.map(|value| value.trim().to_owned());
    let color = request.color.map(|value| value.trim().to_owned());
    let role = poll_action_roles::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_action_id: Set(poll_action_id),
        server_role_id: Set(server_role_id),
        prev_name: Set(name.as_ref().and_then(|_| {
            role_to_update.as_ref().map(|role| role.name.clone())
        })),
        prev_color: Set(color.as_ref().and_then(|_| {
            role_to_update.as_ref().map(|role| role.color.clone())
        })),
        name: Set(name),
        color: Set(color),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    if let Some(permissions) = request.permissions {
        for permission in permissions {
            for action in permission.actions {
                poll_action_permissions::ActiveModel {
                    id: Set(NativeUuid::new_v4()),
                    poll_action_role_id: Set(role.id),
                    subject: Set(permission.subject.clone()),
                    action: Set(action.action),
                    change_type: Set(action.change_type),
                    ..Default::default()
                }
                .insert(database)
                .await
                .map_err(internal_error)?;
            }
        }
    }

    if let Some(members) = request.members {
        for member in members {
            poll_action_role_members::ActiveModel {
                id: Set(NativeUuid::new_v4()),
                poll_action_role_id: Set(role.id),
                user_id: Set(parse_uuid(&member.user_id, "userId")?),
                change_type: Set(member.change_type),
                ..Default::default()
            }
            .insert(database)
            .await
            .map_err(internal_error)?;
        }
    }

    Ok(role)
}

pub(crate) async fn implement_poll_action(
    database: &DatabaseConnection,
    poll_id: Uuid,
) -> AppResult<()> {
    let action = match poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(action) => action,
        None => return Ok(()),
    };

    match action.action_type.as_str() {
        "change-role" => {
            implement_change_server_role(database, action.id).await
        }
        "create-role" => {
            implement_create_server_role(database, poll_id, action.id).await
        }
        _ => Ok(()),
    }
}

pub(crate) async fn shape_poll_action(
    database: &DatabaseConnection,
    poll_id: Uuid,
) -> AppResult<Option<PollActionResponse>> {
    let action = match poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(action) => action,
        None => return Ok(None),
    };
    let server_role = poll_action_roles::Entity::find()
        .filter(poll_action_roles::Column::PollActionId.eq(action.id))
        .one(database)
        .await
        .map_err(internal_error)?;
    let server_role = match server_role {
        Some(role) => Some(shape_poll_action_role(database, role).await?),
        None => None,
    };
    Ok(Some(PollActionResponse {
        id: action.id.to_string(),
        action_type: action.action_type,
        server_role,
    }))
}

async fn implement_change_server_role(
    database: &DatabaseConnection,
    poll_action_id: Uuid,
) -> AppResult<()> {
    let action_role = load_action_role(database, poll_action_id).await?;
    let role_id = action_role.server_role_id.ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server role is required.",
        )
    })?;
    let role = server_roles::Entity::find_by_id(role_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Server role not found.")
        })?;

    if action_role.name.is_some() || action_role.color.is_some() {
        let mut active = role.clone().into_active_model();
        if let Some(name) = action_role.name {
            active.name = Set(name);
        }
        if let Some(color) = action_role.color {
            active.color = Set(color);
        }
        active.update(database).await.map_err(internal_error)?;
    }

    apply_permission_changes(database, role_id, action_role.id).await?;
    apply_member_changes(database, role_id, action_role.id).await
}

async fn implement_create_server_role(
    database: &DatabaseConnection,
    poll_id: Uuid,
    poll_action_id: Uuid,
) -> AppResult<()> {
    let action_role = load_action_role(database, poll_action_id).await?;
    let poll = polls::Entity::find_by_id(poll_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll not found.")
        })?;
    let channel = channels::Entity::find_by_id(poll.channel_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Channel not found.")
        })?;
    let name = action_role.name.ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Role name is required.",
        )
    })?;
    let color = action_role.color.ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Role color is required.",
        )
    })?;
    let role = server_roles::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(channel.server_id),
        name: Set(name),
        color: Set(color),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    copy_action_permissions(database, role.id, action_role.id).await?;
    apply_member_changes(database, role.id, action_role.id).await
}

async fn load_action_role(
    database: &DatabaseConnection,
    poll_action_id: Uuid,
) -> AppResult<poll_action_roles::Model> {
    poll_action_roles::Entity::find()
        .filter(poll_action_roles::Column::PollActionId.eq(poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll action role not found.")
        })
}

async fn apply_permission_changes(
    database: &DatabaseConnection,
    role_id: Uuid,
    action_role_id: Uuid,
) -> AppResult<()> {
    let permissions = poll_action_permissions::Entity::find()
        .filter(
            poll_action_permissions::Column::PollActionRoleId
                .eq(action_role_id),
        )
        .all(database)
        .await
        .map_err(internal_error)?;

    for permission in permissions {
        if permission.change_type == "remove" {
            server_role_permissions::Entity::delete_many()
                .filter(
                    server_role_permissions::Column::ServerRoleId.eq(role_id),
                )
                .filter(
                    server_role_permissions::Column::Subject
                        .eq(permission.subject),
                )
                .filter(
                    server_role_permissions::Column::Action
                        .eq(permission.action),
                )
                .exec(database)
                .await
                .map_err(internal_error)?;
        } else if permission.change_type == "add" {
            add_role_permission(
                database,
                role_id,
                permission.subject,
                permission.action,
            )
            .await?;
        }
    }
    Ok(())
}

async fn copy_action_permissions(
    database: &DatabaseConnection,
    role_id: Uuid,
    action_role_id: Uuid,
) -> AppResult<()> {
    let permissions = poll_action_permissions::Entity::find()
        .filter(
            poll_action_permissions::Column::PollActionRoleId
                .eq(action_role_id),
        )
        .all(database)
        .await
        .map_err(internal_error)?;
    for permission in permissions {
        if permission.change_type == "add" {
            add_role_permission(
                database,
                role_id,
                permission.subject,
                permission.action,
            )
            .await?;
        }
    }
    Ok(())
}

async fn add_role_permission(
    database: &DatabaseConnection,
    role_id: Uuid,
    subject: String,
    action: String,
) -> AppResult<()> {
    if server_role_permissions::Entity::find()
        .filter(server_role_permissions::Column::ServerRoleId.eq(role_id))
        .filter(server_role_permissions::Column::Subject.eq(subject.clone()))
        .filter(server_role_permissions::Column::Action.eq(action.clone()))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Ok(());
    }
    server_role_permissions::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_role_id: Set(role_id),
        subject: Set(subject),
        action: Set(action),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;
    Ok(())
}

async fn apply_member_changes(
    database: &DatabaseConnection,
    role_id: Uuid,
    action_role_id: Uuid,
) -> AppResult<()> {
    let members = poll_action_role_members::Entity::find()
        .filter(
            poll_action_role_members::Column::PollActionRoleId
                .eq(action_role_id),
        )
        .all(database)
        .await
        .map_err(internal_error)?;
    for member in members {
        if member.change_type == "remove" {
            server_role_members::Entity::delete_many()
                .filter(server_role_members::Column::ServerRoleId.eq(role_id))
                .filter(server_role_members::Column::UserId.eq(member.user_id))
                .exec(database)
                .await
                .map_err(internal_error)?;
        } else if member.change_type == "add"
            && server_role_members::Entity::find()
                .filter(server_role_members::Column::ServerRoleId.eq(role_id))
                .filter(server_role_members::Column::UserId.eq(member.user_id))
                .one(database)
                .await
                .map_err(internal_error)?
                .is_none()
        {
            server_role_members::ActiveModel {
                id: Set(NativeUuid::new_v4()),
                server_role_id: Set(role_id),
                user_id: Set(member.user_id),
                ..Default::default()
            }
            .insert(database)
            .await
            .map_err(internal_error)?;
        }
    }
    Ok(())
}

async fn shape_poll_action_role(
    database: &DatabaseConnection,
    role: poll_action_roles::Model,
) -> AppResult<PollActionServerRoleResponse> {
    let members = poll_action_role_members::Entity::find()
        .filter(poll_action_role_members::Column::PollActionRoleId.eq(role.id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let user_ids: Vec<Uuid> =
        members.iter().map(|member| member.user_id).collect();
    let users = if user_ids.is_empty() {
        vec![]
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids.clone()))
            .all(database)
            .await
            .map_err(internal_error)?
    };
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;
    let permissions = poll_action_permissions::Entity::find()
        .filter(poll_action_permissions::Column::PollActionRoleId.eq(role.id))
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(PollActionServerRoleResponse {
        id: role.id.to_string(),
        name: role.name,
        color: role.color,
        prev_name: role.prev_name,
        prev_color: role.prev_color,
        server_role_id: role
            .server_role_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        members: members
            .into_iter()
            .filter_map(|member| {
                users.iter().find(|user| user.id == member.user_id).map(
                    |user| PollActionServerRoleMemberResponse {
                        change_type: member.change_type,
                        user: PollActionUserResponse {
                            id: user.id.to_string(),
                            name: user.name.clone(),
                            display_name: user.display_name.clone(),
                            profile_picture: profile_pictures
                                .get(&user.id)
                                .cloned(),
                        },
                    },
                )
            })
            .collect(),
        permissions: permissions
            .into_iter()
            .map(|permission| PollActionPermissionResponse {
                subject: permission.subject,
                action: permission.action,
                change_type: permission.change_type,
            })
            .collect(),
    })
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll action request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
