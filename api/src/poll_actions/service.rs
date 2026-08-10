//! Owns action-specific validation, persistence, and implementation. Poll
//! authorization, synchronization, and lifecycle transitions remain with polls.

use axum::http::StatusCode;
use chrono::{DateTime, FixedOffset, Utc};
use entity::{
    channels,
    enums::{
        EventAttendeeStatus, PollActionPermissionAbilityAction,
        PollActionPermissionChangeType, PollActionPermissionSubject,
        PollActionRoleMemberChangeType, PollActionType, PollClosedReason,
        ServerAbilitySubject, ServerRoleAbilityAction,
    },
    event_attendees, event_cover_photos, events,
    poll_action_event_cover_photos, poll_action_event_hosts,
    poll_action_events, poll_action_permissions, poll_action_role_members,
    poll_action_roles, poll_action_server_configs, poll_actions, polls,
    server_configs, server_members, server_role_members,
    server_role_permissions, server_roles, users,
};
use sea_orm::{
    prelude::Uuid, sea_query::LockType, ActiveModelTrait, ColumnTrait,
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid as NativeUuid;

use crate::{
    common::{request::parse_uuid, text::sanitize_text, ApiError, AppResult},
    poll_actions::types::{
        CreatePollActionEventRequest, CreatePollActionRequest,
        CreatePollActionServerRoleRequest, PollActionEventCoverPhotoResponse,
        PollActionEventResponse, PollActionPermissionResponse,
        PollActionResponse, PollActionServerConfigResponse,
        PollActionServerRoleMemberResponse, PollActionServerRoleResponse,
        PollActionUserResponse,
    },
    servers, users as users_service,
};

pub(crate) async fn create_poll_action<C: ConnectionTrait>(
    database: &C,
    poll_id: Uuid,
    server_id: Uuid,
    request: CreatePollActionRequest,
    current_server_config: &server_configs::Model,
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

    match request.action_type {
        PollActionType::ChangeSettings => {
            if let Some(server_config) = request.server_config {
                create_poll_action_server_config(
                    database,
                    action.id,
                    server_config,
                    current_server_config,
                )
                .await?;
            }
        }
        PollActionType::ChangeRole | PollActionType::CreateRole => {
            if let Some(server_role) = request.server_role {
                create_poll_action_role(database, action.id, server_role)
                    .await?;
            }
        }
        PollActionType::PlanEvent => {
            if let Some(event) = request.event {
                create_poll_action_event(database, action.id, server_id, event)
                    .await?;
            }
        }
        _ => {}
    }

    Ok(action)
}

pub(crate) fn validate_plan_event_request(
    request: &CreatePollActionEventRequest,
) -> AppResult<()> {
    let name = sanitize_text(&request.name);
    if name.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event name is required.",
        ));
    }
    if name.chars().count() > 255 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event names must be 255 characters or less.",
        ));
    }
    if sanitize_text(&request.description).is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event description is required.",
        ));
    }
    if request.starts_at <= Utc::now().fixed_offset() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event start time must be in the future.",
        ));
    }
    if request
        .ends_at
        .is_some_and(|ends_at| ends_at <= request.starts_at)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event end time must be after its start time.",
        ));
    }
    if !request.online
        && request
            .location
            .as_deref()
            .map(sanitize_text)
            .map(|location| location.is_empty())
            .unwrap_or(true)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "In-person events require a location.",
        ));
    }
    if request
        .location
        .as_deref()
        .is_some_and(|location| sanitize_text(location).chars().count() > 255)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event locations must be 255 characters or less.",
        ));
    }
    if let Some(link) = request
        .external_link
        .as_deref()
        .map(str::trim)
        .filter(|link| !link.is_empty())
    {
        validate_external_link(link)?;
    }
    parse_event_host_ids(&request.host_ids)?;
    Ok(())
}

async fn create_poll_action_event<C: ConnectionTrait>(
    database: &C,
    poll_action_id: Uuid,
    server_id: Uuid,
    request: CreatePollActionEventRequest,
) -> AppResult<()> {
    validate_plan_event_request(&request)?;
    let host_ids = parse_event_host_ids(&request.host_ids)?;
    let memberships = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .filter(server_members::Column::UserId.is_in(host_ids.iter().copied()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let member_ids: HashSet<Uuid> = memberships
        .into_iter()
        .map(|membership| membership.user_id)
        .collect();
    if host_ids.iter().any(|host_id| !member_ids.contains(host_id)) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event hosts must be members of the proposal's server.",
        ));
    }

    let proposed_event = poll_action_events::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_action_id: Set(poll_action_id),
        name: Set(sanitize_text(&request.name)),
        description: Set(sanitize_text(&request.description)),
        starts_at: Set(request.starts_at),
        ends_at: Set(request.ends_at),
        online: Set(request.online),
        location: Set(request
            .location
            .map(|location| sanitize_text(&location))
            .filter(|location| !location.is_empty())),
        external_link: Set(request
            .external_link
            .map(|link| link.trim().to_owned())
            .filter(|link| !link.is_empty())),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    for host_id in host_ids {
        poll_action_event_hosts::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            poll_action_event_id: Set(proposed_event.id),
            user_id: Set(host_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    if request.cover_photo {
        poll_action_event_cover_photos::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            poll_action_event_id: Set(proposed_event.id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

pub(crate) async fn attach_event_cover_photo<C: ConnectionTrait>(
    database: &C,
    upload_root: &Path,
    poll_id: Uuid,
    bytes: Vec<u8>,
) -> AppResult<PathBuf> {
    let content_type =
        crate::common::images::validate_raster(&bytes, "Event cover photo")?
            .content_type;
    let action = poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "An event cover photo requires an event proposal.",
            )
        })?;
    let event = poll_action_events::Entity::find()
        .filter(poll_action_events::Column::PollActionId.eq(action.id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "An event cover photo requires an event proposal.",
            )
        })?;
    let image = poll_action_event_cover_photos::Entity::find()
        .filter(
            poll_action_event_cover_photos::Column::PollActionEventId
                .eq(event.id),
        )
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "The event proposal must request a cover photo.",
            )
        })?;

    let storage_key = format!("poll-action-event-cover-photos/{}", image.id);
    let destination = upload_root.join(&storage_key);
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(internal_error)?;
    }
    if let Err(error) = tokio::fs::write(&destination, bytes).await {
        let _ = tokio::fs::remove_file(&destination).await;
        return Err(internal_error(error));
    }

    let mut active = image.into_active_model();
    active.storage_key = Set(Some(storage_key));
    active.content_type = Set(Some(content_type.to_owned()));
    if let Err(error) = active.update(database).await {
        if let Err(cleanup_error) = tokio::fs::remove_file(&destination).await {
            tracing::warn!(
                "failed to clean up event cover photo after database error: {cleanup_error}"
            );
        }
        return Err(internal_error(error));
    }
    Ok(destination)
}

pub(crate) async fn load_event_cover_photo<C: ConnectionTrait>(
    database: &C,
    poll_id: Uuid,
    image_id: Uuid,
) -> AppResult<poll_action_event_cover_photos::Model> {
    let image = poll_action_event_cover_photos::Entity::find_by_id(image_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Image not found.")
        })?;
    let proposed_event =
        poll_action_events::Entity::find_by_id(image.poll_action_event_id)
            .one(database)
            .await
            .map_err(internal_error)?;
    let belongs_to_poll = match proposed_event {
        Some(proposed_event) => {
            poll_actions::Entity::find_by_id(proposed_event.poll_action_id)
                .filter(poll_actions::Column::PollId.eq(poll_id))
                .one(database)
                .await
                .map_err(internal_error)?
                .is_some()
        }
        None => false,
    };
    if belongs_to_poll {
        Ok(image)
    } else {
        Err(ApiError::new(StatusCode::NOT_FOUND, "Image not found."))
    }
}

pub(crate) async fn plan_event_closed_reason<C: ConnectionTrait>(
    database: &C,
    poll_id: Uuid,
    now: DateTime<FixedOffset>,
) -> AppResult<Option<PollClosedReason>> {
    let action = poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?;

    let Some(action) = action else {
        return Ok(None);
    };
    if action.action_type != PollActionType::PlanEvent {
        return Ok(None);
    }

    let proposed_event = poll_action_events::Entity::find()
        .filter(poll_action_events::Column::PollActionId.eq(action.id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Proposed event is required.",
            )
        })?;
    if proposed_event.starts_at <= now {
        return Ok(Some(PollClosedReason::EventStartElapsed));
    }

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
    let host_ids = poll_action_event_hosts::Entity::find()
        .filter(
            poll_action_event_hosts::Column::PollActionEventId
                .eq(proposed_event.id),
        )
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|host| host.user_id)
        .collect::<Vec<_>>();
    let member_count = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(channel.server_id))
        .filter(server_members::Column::UserId.is_in(host_ids.iter().copied()))
        .count(database)
        .await
        .map_err(internal_error)?;

    Ok((member_count < host_ids.len() as u64)
        .then_some(PollClosedReason::EventHostIneligible))
}

fn parse_event_host_ids(values: &[String]) -> AppResult<Vec<Uuid>> {
    if values.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Events require at least one host.",
        ));
    }

    let mut unique = HashSet::with_capacity(values.len());
    let mut host_ids = Vec::with_capacity(values.len());
    for value in values {
        let host_id = value.parse::<Uuid>().map_err(|_| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Event host IDs must be UUIDs.",
            )
        })?;
        if !unique.insert(host_id) {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Event hosts must be distinct.",
            ));
        }
        host_ids.push(host_id);
    }
    Ok(host_ids)
}

fn validate_external_link(value: &str) -> AppResult<()> {
    let uri = value.parse::<axum::http::Uri>().map_err(|_| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event external links must be valid HTTP(S) URLs.",
        )
    })?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.authority().is_none()
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Event external links must be valid HTTP(S) URLs.",
        ));
    }
    Ok(())
}

async fn create_poll_action_server_config<C: ConnectionTrait>(
    database: &C,
    poll_action_id: Uuid,
    request: crate::servers::types::ServerConfigRequest,
    current: &server_configs::Model,
) -> AppResult<()> {
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

async fn create_poll_action_role<C: ConnectionTrait>(
    database: &C,
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

pub(crate) async fn implement_poll_action_in_transaction(
    transaction: &DatabaseTransaction,
    poll_id: Uuid,
) -> AppResult<bool> {
    let action = match poll_actions::Entity::find()
        .filter(poll_actions::Column::PollId.eq(poll_id))
        .lock(LockType::Update)
        .one(transaction)
        .await
        .map_err(internal_error)?
    {
        Some(action) => action,
        None => return Ok(false),
    };

    if action.executed_at.is_some() {
        return Ok(false);
    }

    match action.action_type {
        PollActionType::ChangeRole => {
            implement_change_server_role(transaction, action.id).await?
        }
        PollActionType::CreateRole => {
            implement_create_server_role(transaction, poll_id, action.id)
                .await?
        }
        PollActionType::ChangeSettings => {
            implement_change_server_config(transaction, poll_id, action.id)
                .await?
        }
        PollActionType::PlanEvent => {
            implement_plan_event(transaction, poll_id, action.id).await?
        }
        _ => {}
    }

    let mut active = action.into_active_model();
    active.executed_at = Set(Some(Utc::now().fixed_offset()));
    active.update(transaction).await.map_err(internal_error)?;
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
    let event = shape_poll_action_event(database, action.id).await?;
    Ok(Some(PollActionResponse {
        id: action.id.to_string(),
        action_type: action.action_type,
        server_role,
        server_config,
        event,
    }))
}

async fn implement_plan_event(
    database: &DatabaseTransaction,
    poll_id: Uuid,
    poll_action_id: Uuid,
) -> AppResult<()> {
    let proposed = poll_action_events::Entity::find()
        .filter(poll_action_events::Column::PollActionId.eq(poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Proposed event is required.",
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

    let event = match events::Entity::find()
        .filter(events::Column::SourcePollActionId.eq(poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(event) => event,
        None => events::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            server_id: Set(channel.server_id),
            source_poll_action_id: Set(Some(poll_action_id)),
            name: Set(proposed.name),
            description: Set(proposed.description),
            starts_at: Set(proposed.starts_at),
            ends_at: Set(proposed.ends_at),
            online: Set(proposed.online),
            location: Set(proposed.location),
            external_link: Set(proposed.external_link),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?,
    };

    let hosts = poll_action_event_hosts::Entity::find()
        .filter(
            poll_action_event_hosts::Column::PollActionEventId.eq(proposed.id),
        )
        .all(database)
        .await
        .map_err(internal_error)?;
    for host in hosts {
        let existing = event_attendees::Entity::find()
            .filter(event_attendees::Column::EventId.eq(event.id))
            .filter(event_attendees::Column::UserId.eq(host.user_id))
            .one(database)
            .await
            .map_err(internal_error)?;
        if let Some(attendee) = existing {
            if attendee.status != EventAttendeeStatus::Host {
                let mut active = attendee.into_active_model();
                active.status = Set(EventAttendeeStatus::Host);
                active.updated_at = Set(Utc::now().fixed_offset());
                active.update(database).await.map_err(internal_error)?;
            }
        } else {
            event_attendees::ActiveModel {
                id: Set(NativeUuid::new_v4()),
                event_id: Set(event.id),
                user_id: Set(host.user_id),
                status: Set(EventAttendeeStatus::Host),
                ..Default::default()
            }
            .insert(database)
            .await
            .map_err(internal_error)?;
        }
    }

    sync_event_cover_photo(database, proposed.id).await?;

    Ok(())
}

pub(crate) async fn sync_event_cover_photo<C: ConnectionTrait>(
    database: &C,
    poll_action_event_id: Uuid,
) -> AppResult<()> {
    let proposed = poll_action_events::Entity::find_by_id(poll_action_event_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Proposed event not found.")
        })?;
    let Some(event) = events::Entity::find()
        .filter(events::Column::SourcePollActionId.eq(proposed.poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
    else {
        return Ok(());
    };
    let existing_cover = event_cover_photos::Entity::find()
        .filter(event_cover_photos::Column::EventId.eq(event.id))
        .one(database)
        .await
        .map_err(internal_error)?;
    if existing_cover.is_none() {
        let proposed_cover = poll_action_event_cover_photos::Entity::find()
            .filter(
                poll_action_event_cover_photos::Column::PollActionEventId
                    .eq(poll_action_event_id),
            )
            .one(database)
            .await
            .map_err(internal_error)?;
        if let Some(proposed_cover) = proposed_cover {
            if let Some(source_storage_key) = proposed_cover.storage_key {
                let cover_photo_id = NativeUuid::new_v4();
                let storage_key =
                    format!("event-cover-photos/{cover_photo_id}");
                let upload_root = crate::common::storage::upload_root();
                let destination = upload_root.join(&storage_key);
                if let Some(parent) = destination.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(internal_error)?;
                }
                tokio::fs::copy(
                    upload_root.join(source_storage_key),
                    &destination,
                )
                .await
                .map_err(internal_error)?;
                event_cover_photos::ActiveModel {
                    id: Set(cover_photo_id),
                    event_id: Set(event.id),
                    storage_key: Set(storage_key),
                    content_type: Set(proposed_cover.content_type),
                    ..Default::default()
                }
                .insert(database)
                .await
                .map_err(internal_error)?;
            }
        }
    }

    Ok(())
}

async fn shape_poll_action_event(
    database: &DatabaseConnection,
    poll_action_id: Uuid,
) -> AppResult<Option<PollActionEventResponse>> {
    let proposed = match poll_action_events::Entity::find()
        .filter(poll_action_events::Column::PollActionId.eq(poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        Some(event) => event,
        None => return Ok(None),
    };
    let hosts = poll_action_event_hosts::Entity::find()
        .filter(
            poll_action_event_hosts::Column::PollActionEventId.eq(proposed.id),
        )
        .order_by_asc(poll_action_event_hosts::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let host_ids: Vec<Uuid> = hosts.iter().map(|host| host.user_id).collect();
    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(host_ids.iter().copied()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let users_by_id: std::collections::HashMap<Uuid, users::Model> =
        users.into_iter().map(|user| (user.id, user)).collect();
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &host_ids)
            .await?;
    let created_event_id = events::Entity::find()
        .filter(events::Column::SourcePollActionId.eq(poll_action_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .map(|event| event.id.to_string());
    let cover_photo = poll_action_event_cover_photos::Entity::find()
        .filter(
            poll_action_event_cover_photos::Column::PollActionEventId
                .eq(proposed.id),
        )
        .one(database)
        .await
        .map_err(internal_error)?
        .map(|image| PollActionEventCoverPhotoResponse {
            id: image.id.to_string(),
            is_placeholder: image.storage_key.is_none(),
            created_at: crate::messages::types::serialize_timestamp(
                image.created_at,
            ),
        });

    Ok(Some(PollActionEventResponse {
        id: proposed.id.to_string(),
        name: proposed.name,
        description: proposed.description,
        starts_at: crate::messages::types::serialize_timestamp(
            proposed.starts_at,
        ),
        ends_at: proposed
            .ends_at
            .map(crate::messages::types::serialize_timestamp),
        online: proposed.online,
        location: proposed.location,
        external_link: proposed.external_link,
        hosts: hosts
            .into_iter()
            .filter_map(|host| users_by_id.get(&host.user_id))
            .map(|user| PollActionUserResponse {
                id: user.id.to_string(),
                name: user.name.clone(),
                display_name: user.display_name.clone(),
                profile_picture: profile_pictures.get(&user.id).cloned(),
            })
            .collect(),
        cover_photo,
        created_event_id,
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
        if permission.change_type == PollActionPermissionChangeType::Remove {
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
        } else if permission.change_type == PollActionPermissionChangeType::Add
        {
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
        if permission.change_type == PollActionPermissionChangeType::Add {
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
        if member.change_type == PollActionRoleMemberChangeType::Remove {
            server_role_members::Entity::delete_many()
                .filter(server_role_members::Column::ServerRoleId.eq(role_id))
                .filter(server_role_members::Column::UserId.eq(member.user_id))
                .exec(database)
                .await
                .map_err(internal_error)?;
        } else if member.change_type == PollActionRoleMemberChangeType::Add
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
                subject: permission.subject.to_string(),
                action: permission.action.to_string(),
                change_type: permission.change_type,
            })
            .collect(),
    })
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("poll action request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
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
