use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    channels,
    enums::{
        PollActionPermissionAbilityAction, PollActionPermissionChangeType,
        PollActionPermissionSubject, PollActionRoleMemberChangeType,
        PollActionType, ServerAbilitySubject, ServerRoleAbilityAction,
    },
    poll_action_permissions, poll_action_role_members, poll_action_roles,
    poll_action_server_configs, poll_actions, polls, server_configs,
    server_role_members, server_role_permissions, server_roles, users,
};
use sea_orm::{
    prelude::Uuid, sea_query::LockType, ActiveModelTrait, ColumnTrait,
    DatabaseConnection, DatabaseTransaction, EntityTrait, IntoActiveModel,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid as NativeUuid;

use crate::{
    common::{request::parse_uuid, ApiError, AppResult},
    poll_actions::types::{
        CreatePollActionRequest, CreatePollActionServerRoleRequest,
        PollActionPermissionResponse, PollActionResponse,
        PollActionServerConfigResponse, PollActionServerRoleMemberResponse,
        PollActionServerRoleResponse, PollActionUserResponse,
    },
    servers, users as users_service,
};

pub(crate) async fn create_poll_action(
    database: &DatabaseConnection,
    poll_id: Uuid,
    request: CreatePollActionRequest,
) -> AppResult<poll_actions::Model> {
    let action = poll_actions::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll_id),
        action_type: Set(parse_poll_action_type(&request.action_type)?),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    if let Some(server_role) = request.server_role {
        create_poll_action_role(database, action.id, server_role).await?;
    }
    if let Some(server_config) = request.server_config {
        create_poll_action_server_config(
            database,
            poll_id,
            action.id,
            server_config,
        )
        .await?;
    }

    Ok(action)
}

async fn create_poll_action_server_config(
    database: &DatabaseConnection,
    poll_id: Uuid,
    poll_action_id: Uuid,
    request: crate::servers::types::ServerConfigRequest,
) -> AppResult<()> {
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
    let current = servers::server_configs::service::ensure_server_config(
        database,
        channel.server_id,
    )
    .await?;
    poll_action_server_configs::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_action_id: Set(poll_action_id),
        anonymous_users_enabled: Set(request.anonymous_users_enabled),
        prev_anonymous_users_enabled: Set(request
            .anonymous_users_enabled
            .map(|_| current.anonymous_users_enabled)),
        decision_making_model: Set(request.decision_making_model.clone()),
        prev_decision_making_model: Set(request
            .decision_making_model
            .map(|_| current.decision_making_model.to_string())),
        disagreements_limit: Set(request.disagreements_limit),
        prev_disagreements_limit: Set(request
            .disagreements_limit
            .map(|_| current.disagreements_limit)),
        abstains_limit: Set(request.abstains_limit),
        prev_abstains_limit: Set(request
            .abstains_limit
            .map(|_| current.abstains_limit)),
        agreement_threshold: Set(request.agreement_threshold),
        prev_agreement_threshold: Set(request
            .agreement_threshold
            .map(|_| current.agreement_threshold)),
        quorum_enabled: Set(request.quorum_enabled),
        prev_quorum_enabled: Set(request
            .quorum_enabled
            .map(|_| current.quorum_enabled)),
        quorum_threshold: Set(request.quorum_threshold),
        prev_quorum_threshold: Set(request
            .quorum_threshold
            .map(|_| current.quorum_threshold)),
        voting_time_limit: Set(request.voting_time_limit),
        prev_voting_time_limit: Set(request
            .voting_time_limit
            .map(|_| current.voting_time_limit)),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;
    Ok(())
}

pub(crate) fn validate_server_config_change(
    request: &crate::servers::types::ServerConfigRequest,
    current: &server_configs::Model,
) -> AppResult<()> {
    servers::server_configs::service::validate_server_config_request(request)?;
    let has_real_change = request
        .anonymous_users_enabled
        .map(|value| value != current.anonymous_users_enabled)
        .unwrap_or(false)
        || request
            .decision_making_model
            .as_deref()
            .map(|value| value != current.decision_making_model.to_string())
            .unwrap_or(false)
        || request
            .disagreements_limit
            .map(|value| value != current.disagreements_limit)
            .unwrap_or(false)
        || request
            .abstains_limit
            .map(|value| value != current.abstains_limit)
            .unwrap_or(false)
        || request
            .agreement_threshold
            .map(|value| value != current.agreement_threshold)
            .unwrap_or(false)
        || request
            .quorum_enabled
            .map(|value| value != current.quorum_enabled)
            .unwrap_or(false)
        || request
            .quorum_threshold
            .map(|value| value != current.quorum_threshold)
            .unwrap_or(false)
        || request
            .voting_time_limit
            .map(|value| value != current.voting_time_limit)
            .unwrap_or(false);
    if !has_real_change {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server settings proposals must include at least 1 real change.",
        ));
    }
    Ok(())
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
        Some(
            server_roles::Entity::find_by_id(server_role_id)
                .one(database)
                .await
                .map_err(internal_error)?
                .ok_or_else(|| {
                    ApiError::new(
                        StatusCode::NOT_FOUND,
                        "Server role not found.",
                    )
                })?,
        )
    } else {
        None
    };

    let name = request.name.map(|value| value.trim().to_owned());
    let color = request.color.map(|value| value.trim().to_owned());

    let prev_name = name
        .as_deref()
        .filter(|value| !value.is_empty())
        .and_then(|_| role_to_update.as_ref().map(|role| role.name.clone()));

    let prev_color = color
        .as_deref()
        .filter(|value| !value.is_empty())
        .and_then(|_| role_to_update.as_ref().map(|role| role.color.clone()));

    let role = poll_action_roles::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_action_id: Set(poll_action_id),
        server_role_id: Set(server_role_id),
        prev_name: Set(prev_name),
        prev_color: Set(prev_color),
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
                    subject: Set(parse_poll_action_permission_subject(
                        &permission.subject,
                    )?),
                    action: Set(parse_poll_action_permission_action(
                        &action.action,
                    )?),
                    change_type: Set(parse_poll_action_permission_change_type(
                        &action.change_type,
                    )?),
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
                change_type: Set(parse_poll_action_role_member_change_type(
                    &member.change_type,
                )?),
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
) -> AppResult<bool> {
    let transaction = database.begin().await.map_err(internal_error)?;
    let action = match poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .lock(LockType::Update)
        .one(&transaction)
        .await
        .map_err(internal_error)?
    {
        Some(action) => action,
        None => {
            transaction.commit().await.map_err(internal_error)?;
            return Ok(false);
        }
    };

    if action.executed_at.is_some() {
        transaction.commit().await.map_err(internal_error)?;
        return Ok(false);
    }

    match action.action_type.as_str() {
        "change-role" => {
            implement_change_server_role(&transaction, action.id).await?
        }
        "create-role" => {
            implement_create_server_role(&transaction, poll_id, action.id)
                .await?
        }
        "change-settings" => {
            implement_change_server_config(&transaction, poll_id, action.id)
                .await?
        }
        _ => {}
    }

    let mut active = action.into_active_model();
    active.executed_at = Set(Some(Utc::now().fixed_offset()));
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;
    Ok(true)
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
    let server_config = poll_action_server_configs::Entity::find()
        .filter(poll_action_server_configs::Column::PollActionId.eq(action.id))
        .one(database)
        .await
        .map_err(internal_error)?
        .map(|config| PollActionServerConfigResponse {
            anonymous_users_enabled: config.anonymous_users_enabled,
            prev_anonymous_users_enabled: config.prev_anonymous_users_enabled,
            decision_making_model: config.decision_making_model,
            prev_decision_making_model: config.prev_decision_making_model,
            disagreements_limit: config.disagreements_limit,
            prev_disagreements_limit: config.prev_disagreements_limit,
            abstains_limit: config.abstains_limit,
            prev_abstains_limit: config.prev_abstains_limit,
            agreement_threshold: config.agreement_threshold,
            prev_agreement_threshold: config.prev_agreement_threshold,
            quorum_enabled: config.quorum_enabled,
            prev_quorum_enabled: config.prev_quorum_enabled,
            quorum_threshold: config.quorum_threshold,
            prev_quorum_threshold: config.prev_quorum_threshold,
            voting_time_limit: config.voting_time_limit,
            prev_voting_time_limit: config.prev_voting_time_limit,
        });
    Ok(Some(PollActionResponse {
        id: action.id.to_string(),
        action_type: action.action_type.to_string(),
        server_role,
        server_config,
    }))
}

async fn implement_change_server_config(
    database: &DatabaseTransaction,
    poll_id: Uuid,
    poll_action_id: Uuid,
) -> AppResult<()> {
    let change = poll_action_server_configs::Entity::find()
        .filter(
            poll_action_server_configs::Column::PollActionId.eq(poll_action_id),
        )
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Server config changes are required.",
            )
        })?;
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
    let config = server_configs::Entity::find()
        .filter(server_configs::Column::ServerId.eq(channel.server_id))
        .lock(LockType::Update)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Server config not found.")
        })?;
    let request = crate::servers::types::ServerConfigRequest {
        anonymous_users_enabled: change.anonymous_users_enabled,
        decision_making_model: change.decision_making_model,
        disagreements_limit: change.disagreements_limit,
        abstains_limit: change.abstains_limit,
        agreement_threshold: change.agreement_threshold,
        quorum_enabled: change.quorum_enabled,
        quorum_threshold: change.quorum_threshold,
        voting_time_limit: change.voting_time_limit,
    };
    servers::server_configs::service::apply_server_config(
        database, config, &request,
    )
    .await
}

async fn implement_change_server_role(
    database: &DatabaseTransaction,
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
    database: &DatabaseTransaction,
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
    database: &DatabaseTransaction,
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
    database: &DatabaseTransaction,
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
                        .eq(ServerAbilitySubject::from(permission.subject)),
                )
                .filter(
                    server_role_permissions::Column::Action
                        .eq(ServerRoleAbilityAction::from(permission.action)),
                )
                .exec(database)
                .await
                .map_err(internal_error)?;
        } else if permission.change_type == "add" {
            add_role_permission(
                database,
                role_id,
                permission.subject.into(),
                permission.action.into(),
            )
            .await?;
        }
    }
    Ok(())
}

async fn copy_action_permissions(
    database: &DatabaseTransaction,
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
                permission.subject.into(),
                permission.action.into(),
            )
            .await?;
        }
    }
    Ok(())
}

async fn add_role_permission(
    database: &DatabaseTransaction,
    role_id: Uuid,
    subject: ServerAbilitySubject,
    action: ServerRoleAbilityAction,
) -> AppResult<()> {
    if server_role_permissions::Entity::find()
        .filter(server_role_permissions::Column::ServerRoleId.eq(role_id))
        .filter(server_role_permissions::Column::Subject.eq(subject))
        .filter(server_role_permissions::Column::Action.eq(action))
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
    database: &DatabaseTransaction,
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
                        change_type: member.change_type.to_string(),
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
                subject: permission.subject.to_string(),
                action: permission.action.to_string(),
                change_type: permission.change_type.to_string(),
            })
            .collect(),
    })
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll action request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn parse_poll_action_type(value: &str) -> AppResult<PollActionType> {
    value.parse().map_err(|_| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll action type is invalid.",
        )
    })
}

fn parse_poll_action_permission_subject(
    value: &str,
) -> AppResult<PollActionPermissionSubject> {
    value.parse().map_err(|_| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Subject is invalid.")
    })
}

fn parse_poll_action_permission_action(
    value: &str,
) -> AppResult<PollActionPermissionAbilityAction> {
    value.parse().map_err(|_| {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Action is invalid.")
    })
}

fn parse_poll_action_permission_change_type(
    value: &str,
) -> AppResult<PollActionPermissionChangeType> {
    value.parse().map_err(|_| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Change type is invalid.",
        )
    })
}

fn parse_poll_action_role_member_change_type(
    value: &str,
) -> AppResult<PollActionRoleMemberChangeType> {
    value.parse().map_err(|_| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Change type is invalid.",
        )
    })
}
