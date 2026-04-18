use axum::http::StatusCode;
use entity::{channel_members, channels, server_members, servers};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid as NativeUuid;

use super::types::{ChannelRequest, ChannelResponse, ChannelServer};
use crate::common::{ApiError, AppResult};
use crate::servers as servers_service;

pub(crate) async fn get_channels(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<ChannelResponse>> {
    servers_service::ensure_server(database, server_id).await?;

    let server = servers_service::load_server(database, server_id).await?;
    let channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .order_by_asc(channels::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(channels
        .into_iter()
        .map(|channel| shape_channel(channel, &server))
        .collect())
}

pub(crate) async fn get_joined_channels(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<Vec<ChannelResponse>> {
    servers_service::ensure_server(database, server_id).await?;

    let server = servers_service::load_server(database, server_id).await?;
    let memberships = channel_members::Entity::find()
        .filter(channel_members::Column::UserId.eq(user_id))
        .all(database)
        .await
        .map_err(internal_error)?;

    let channel_ids: Vec<Uuid> = memberships
        .into_iter()
        .map(|member| member.channel_id)
        .collect();

    if channel_ids.is_empty() {
        return Ok(vec![]);
    }

    let channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .filter(channels::Column::Id.is_in(channel_ids))
        .order_by_asc(channels::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(channels
        .into_iter()
        .map(|channel| shape_channel(channel, &server))
        .collect())
}

pub(crate) async fn get_channel_with_server(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<ChannelResponse> {
    let server = servers_service::load_server(database, server_id).await?;
    let channel = get_channel(database, server_id, channel_id).await?;
    Ok(shape_channel(channel, &server))
}

pub(crate) async fn create_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    request: ChannelRequest,
) -> AppResult<ChannelResponse> {
    let server = servers_service::load_server(database, server_id).await?;
    let (name, description) = validate_channel_request(request)?;

    let channel = channels::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        name: Set(name),
        description: Set(description),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    let server_members = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;

    for member in server_members {
        let _ = channel_members::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            channel_id: Set(channel.id),
            user_id: Set(member.user_id),
            ..Default::default()
        }
        .insert(database)
        .await;
    }

    Ok(shape_channel(channel, &server))
}

pub(crate) async fn update_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    request: ChannelRequest,
) -> AppResult<()> {
    let (name, description) = validate_channel_request(request)?;
    let channel = get_channel(database, server_id, channel_id).await?;
    let mut active = channel.into_active_model();
    active.name = Set(name);
    active.description = Set(description);
    active.update(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn delete_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<()> {
    let channel = get_channel(database, server_id, channel_id).await?;
    channel.delete(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn get_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
) -> AppResult<channels::Model> {
    channels::Entity::find_by_id(channel_id)
        .filter(channels::Column::ServerId.eq(server_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Channel not found.")
        })
}

pub(crate) async fn ensure_channel_membership(
    database: &DatabaseConnection,
    channel_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let membership = channel_members::Entity::find()
        .filter(channel_members::Column::ChannelId.eq(channel_id))
        .filter(channel_members::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?;

    if membership.is_some() {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}

pub(crate) async fn get_channel_member_user_ids(
    database: &DatabaseConnection,
    channel_id: Uuid,
) -> AppResult<Vec<Uuid>> {
    let members = channel_members::Entity::find()
        .filter(channel_members::Column::ChannelId.eq(channel_id))
        .all(database)
        .await
        .map_err(internal_error)?;

    Ok(members.into_iter().map(|member| member.user_id).collect())
}

pub(crate) async fn add_member_to_all_server_channels<C>(
    database: &C,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let server_channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let channel_ids: Vec<Uuid> =
        server_channels.iter().map(|channel| channel.id).collect();

    if channel_ids.is_empty() {
        return Ok(());
    }

    let existing_memberships = channel_members::Entity::find()
        .filter(channel_members::Column::UserId.eq(user_id))
        .filter(channel_members::Column::ChannelId.is_in(channel_ids.clone()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let existing_channel_ids: std::collections::HashSet<Uuid> =
        existing_memberships
            .into_iter()
            .map(|membership| membership.channel_id)
            .collect();

    for channel in server_channels {
        if existing_channel_ids.contains(&channel.id) {
            continue;
        }

        channel_members::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            channel_id: Set(channel.id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

pub(crate) async fn create_general_channel(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<()> {
    let existing = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .filter(channels::Column::Name.eq("general"))
        .one(database)
        .await
        .map_err(internal_error)?;

    if existing.is_some() {
        return Ok(());
    }

    let channel = channels::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        name: Set("general".to_owned()),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    let members = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    for member in members {
        channel_members::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            channel_id: Set(channel.id),
            user_id: Set(member.user_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

pub(crate) async fn general_channel_id(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Option<Uuid>> {
    let channel = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .filter(channels::Column::Name.eq("general"))
        .one(database)
        .await
        .map_err(internal_error)?;
    Ok(channel.map(|channel| channel.id))
}

fn shape_channel(
    channel: channels::Model,
    server: &servers::Model,
) -> ChannelResponse {
    ChannelResponse {
        id: channel.id.to_string(),
        name: channel.name,
        description: channel.description,
        server: ChannelServer {
            id: server.id.to_string(),
            slug: server.slug.clone(),
        },
    }
}

fn validate_channel_request(
    request: ChannelRequest,
) -> AppResult<(String, Option<String>)> {
    let name = request.name.trim().to_ascii_lowercase();
    if !(2..=30).contains(&name.chars().count()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Channel name must be between 2 and 30 characters.",
        ));
    }

    let description = request
        .description
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    if description
        .as_ref()
        .map(|value| value.chars().count() > 255)
        .unwrap_or(false)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Channel description must be at most 255 characters.",
        ));
    }

    Ok((name, description))
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("channel request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
