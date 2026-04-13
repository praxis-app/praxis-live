use axum::http::StatusCode;
use entity::{
    channel_members, channels, instance_configs, server_members, servers, users,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, SqlErr,
};
use uuid::Uuid as NativeUuid;

use super::types::{
    serialize_timestamp, ServerRequest, ServerResponse, UserResponse,
};
use crate::channels as channel_api;
use crate::common::{ApiError, AppResult};
use crate::instance;

pub(crate) use super::server_configs::{
    ensure_server_config, get_server_config, is_anonymous_users_enabled,
    update_server_config,
};

const INITIAL_SERVER_NAME: &str = "praxis";
const INITIAL_SERVER_SLUG: &str = "praxis";

pub(crate) async fn default_server_id(
    database: &DatabaseConnection,
) -> AppResult<Uuid> {
    let config = instance::get_config_safely(database).await?;
    Ok(config.default_server_id)
}

pub(crate) async fn get_servers(
    database: &DatabaseConnection,
) -> AppResult<Vec<ServerResponse>> {
    let default_server_id = default_server_id(database).await?;
    let servers = servers::Entity::find()
        .order_by_desc(servers::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut responses = Vec::with_capacity(servers.len());
    for server in servers {
        responses.push(
            shape_server(database, server, default_server_id, false, true)
                .await?,
        );
    }

    Ok(responses)
}

pub(crate) async fn get_servers_for_user(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<Vec<ServerResponse>> {
    let default_server_id = default_server_id(database).await?;
    let memberships = server_members::Entity::find()
        .filter(server_members::Column::UserId.eq(user_id))
        .order_by_desc(server_members::Column::LastActiveAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let server_ids: Vec<Uuid> = memberships
        .iter()
        .map(|membership| membership.server_id)
        .collect();

    if server_ids.is_empty() {
        return Ok(vec![]);
    }

    let servers = servers::Entity::find()
        .filter(servers::Column::Id.is_in(server_ids))
        .order_by_desc(servers::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;

    let mut responses = Vec::with_capacity(servers.len());
    for server in servers {
        responses.push(
            shape_server(database, server, default_server_id, false, true)
                .await?,
        );
    }

    Ok(responses)
}

pub(crate) async fn get_current_server(
    database: &DatabaseConnection,
    user_id: Uuid,
) -> AppResult<Option<ServerResponse>> {
    let default_server_id = default_server_id(database).await?;
    let membership = server_members::Entity::find()
        .filter(server_members::Column::UserId.eq(user_id))
        .order_by_desc(server_members::Column::LastActiveAt)
        .one(database)
        .await
        .map_err(internal_error)?;

    let server_id = membership
        .map(|membership| membership.server_id)
        .unwrap_or(default_server_id);

    let Some(server) = servers::Entity::find_by_id(server_id)
        .one(database)
        .await
        .map_err(internal_error)?
    else {
        return Ok(None);
    };

    shape_server(database, server, default_server_id, true, false)
        .await
        .map(Some)
}

pub(crate) async fn get_server_by_id(
    database: &DatabaseConnection,
    server_id: Uuid,
    include_general_channel: bool,
) -> AppResult<ServerResponse> {
    let default_server_id = default_server_id(database).await?;
    let server = get_server(database, server_id).await?;
    shape_server(
        database,
        server,
        default_server_id,
        include_general_channel,
        false,
    )
    .await
}

pub(crate) async fn get_server_by_slug(
    database: &DatabaseConnection,
    slug: &str,
    user_id: Uuid,
) -> AppResult<ServerResponse> {
    let server = servers::Entity::find()
        .filter(servers::Column::Slug.eq(slug))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Server not found.")
        })?;

    set_member_activity(database, server.id, user_id).await?;
    let default_server_id = default_server_id(database).await?;
    shape_server(database, server, default_server_id, true, false).await
}

pub(crate) async fn get_default_server(
    database: &DatabaseConnection,
) -> AppResult<ServerResponse> {
    let default_server_id = default_server_id(database).await?;
    let server = get_server(database, default_server_id).await?;
    shape_server(database, server, default_server_id, true, true).await
}

pub(crate) async fn create_server(
    database: &DatabaseConnection,
    request: ServerRequest,
    current_user_id: Uuid,
) -> AppResult<ServerResponse> {
    let (name, slug, description) = validate_server_request(&request)?;
    let server_id = NativeUuid::new_v4();

    let server = servers::ActiveModel {
        id: Set(server_id),
        name: Set(name),
        slug: Set(slug),
        description: Set(description),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(map_write_error)?;

    ensure_server_config(database, server.id).await?;
    add_server_members(database, server.id, &[current_user_id]).await?;
    channel_api::create_general_channel(database, server.id).await?;

    if request.is_default_server.unwrap_or(false) {
        set_default_server(database, server.id).await?;
    }

    let default_server_id = default_server_id(database).await?;
    shape_server(database, server, default_server_id, false, false).await
}

pub(crate) async fn update_server(
    database: &DatabaseConnection,
    server_id: Uuid,
    request: ServerRequest,
) -> AppResult<ServerResponse> {
    let (name, slug, description) = validate_server_request(&request)?;
    let server = get_server(database, server_id).await?;
    let mut active = server.into_active_model();
    active.name = Set(name);
    active.slug = Set(slug);
    active.description = Set(description);
    let server = active.update(database).await.map_err(map_write_error)?;

    if request.is_default_server.unwrap_or(false) {
        set_default_server(database, server.id).await?;
    }

    let default_server_id = default_server_id(database).await?;
    shape_server(database, server, default_server_id, false, false).await
}

pub(crate) async fn delete_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<()> {
    let server = get_server(database, server_id).await?;
    let server_count = servers::Entity::find()
        .count(database)
        .await
        .map_err(internal_error)?;
    if server_count <= 1 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "There must be at least one server per instance.",
        ));
    }

    if server.id == default_server_id(database).await? {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "The default server cannot be deleted.",
        ));
    }

    server.delete(database).await.map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn get_server_members(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<UserResponse>> {
    get_server(database, server_id).await?;
    let memberships = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .order_by_asc(server_members::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let user_ids: Vec<Uuid> = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect();

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

pub(crate) async fn get_users_eligible_for_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<Vec<UserResponse>> {
    get_server(database, server_id).await?;
    let memberships = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let member_ids: Vec<Uuid> = memberships
        .iter()
        .map(|membership| membership.user_id)
        .collect();

    let mut query = users::Entity::find();
    if !member_ids.is_empty() {
        query = query.filter(users::Column::Id.is_not_in(member_ids));
    }

    let users = query.all(database).await.map_err(internal_error)?;
    Ok(users.into_iter().map(shape_user).collect())
}

pub(crate) async fn add_server_members(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_ids: &[Uuid],
) -> AppResult<()> {
    get_server(database, server_id).await?;

    for user_id in user_ids {
        if users::Entity::find_by_id(*user_id)
            .one(database)
            .await
            .map_err(internal_error)?
            .is_none()
        {
            continue;
        }

        add_member_to_server(database, server_id, *user_id).await?;
        channel_api::add_member_to_all_server_channels(
            database, server_id, *user_id,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn add_member_to_server<C>(
    database: &C,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let exists = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .filter(server_members::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some();

    if exists {
        return Ok(());
    }

    server_members::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        server_id: Set(server_id),
        user_id: Set(user_id),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    Ok(())
}

pub(crate) async fn remove_server_members(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_ids: &[Uuid],
) -> AppResult<()> {
    get_server(database, server_id).await?;

    server_members::Entity::delete_many()
        .filter(server_members::Column::ServerId.eq(server_id))
        .filter(server_members::Column::UserId.is_in(user_ids.to_vec()))
        .exec(database)
        .await
        .map_err(internal_error)?;

    let server_channels = channels::Entity::find()
        .filter(channels::Column::ServerId.eq(server_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let channel_ids: Vec<Uuid> =
        server_channels.iter().map(|channel| channel.id).collect();

    if !channel_ids.is_empty() {
        channel_members::Entity::delete_many()
            .filter(channel_members::Column::ChannelId.is_in(channel_ids))
            .filter(channel_members::Column::UserId.is_in(user_ids.to_vec()))
            .exec(database)
            .await
            .map_err(internal_error)?;
    }

    Ok(())
}

pub(crate) async fn join_server(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
    _invite_token: &str,
) -> AppResult<()> {
    add_server_members(database, server_id, &[user_id]).await
}

async fn shape_server(
    database: &DatabaseConnection,
    server: servers::Model,
    default_server_id: Uuid,
    include_general_channel: bool,
    include_member_count: bool,
) -> AppResult<ServerResponse> {
    let general_channel_id = if include_general_channel {
        channel_api::general_channel_id(database, server.id)
            .await?
            .map(|id| id.to_string())
    } else {
        None
    };

    let member_count = if include_member_count {
        Some(
            server_members::Entity::find()
                .filter(server_members::Column::ServerId.eq(server.id))
                .count(database)
                .await
                .map_err(internal_error)?,
        )
    } else {
        None
    };

    Ok(ServerResponse {
        id: server.id.to_string(),
        name: server.name,
        slug: server.slug,
        description: server.description,
        is_default_server: Some(server.id == default_server_id),
        general_channel_id,
        member_count,
        created_at: serialize_timestamp(server.created_at),
        updated_at: serialize_timestamp(server.updated_at),
    })
}

fn shape_user(user: users::Model) -> UserResponse {
    UserResponse {
        id: user.id.to_string(),
        name: user.name,
        display_name: None,
        profile_picture: None,
    }
}

pub(crate) async fn load_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<servers::Model> {
    servers::Entity::find_by_id(server_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Server not found.")
        })
}

pub(crate) async fn ensure_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<()> {
    load_server(database, server_id).await.map(|_| ())
}

async fn get_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<servers::Model> {
    load_server(database, server_id).await
}

async fn set_default_server(
    database: &DatabaseConnection,
    server_id: Uuid,
) -> AppResult<()> {
    get_server(database, server_id).await?;

    let config = instance::get_config(database).await?;

    if let Some(config) = config {
        let mut active = config.into_active_model();
        active.default_server_id = Set(server_id);
        active.update(database).await.map_err(internal_error)?;
    } else {
        instance_configs::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            default_server_id: Set(server_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }

    Ok(())
}

pub(crate) async fn create_initial_server(
    database: &DatabaseConnection,
) -> AppResult<servers::Model> {
    if let Some(server) = servers::Entity::find()
        .filter(servers::Column::Slug.eq(INITIAL_SERVER_SLUG))
        .one(database)
        .await
        .map_err(internal_error)?
    {
        ensure_server_config(database, server.id).await?;
        channel_api::create_general_channel(database, server.id).await?;
        return Ok(server);
    }

    let server = servers::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        name: Set(INITIAL_SERVER_NAME.to_owned()),
        slug: Set(INITIAL_SERVER_SLUG.to_owned()),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(map_write_error)?;

    ensure_server_config(database, server.id).await?;
    channel_api::create_general_channel(database, server.id).await?;

    Ok(server)
}

async fn set_member_activity(
    database: &DatabaseConnection,
    server_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let _membership = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(server_id))
        .filter(server_members::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?;

    Ok(())
}

fn validate_server_request(
    request: &ServerRequest,
) -> AppResult<(String, String, Option<String>)> {
    let name = request.name.trim().to_owned();
    let slug = request.slug.trim().to_ascii_lowercase();
    let description = request
        .description
        .as_ref()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    if !(2..=30).contains(&name.chars().count()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server name must be between 2 and 30 characters.",
        ));
    }
    if !(2..=30).contains(&slug.chars().count()) || !valid_slug(&slug) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server slug is invalid.",
        ));
    }
    if description
        .as_ref()
        .map(|value| value.chars().count() > 255)
        .unwrap_or(false)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server description must be at most 255 characters.",
        ));
    }

    Ok((name, slug, description))
}

fn valid_slug(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
        return false;
    }

    bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'
    }) && !value.contains("--")
}

fn map_write_error(error: sea_orm::DbErr) -> ApiError {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return ApiError::new(StatusCode::CONFLICT, "Server already exists.");
    }

    internal_error(error)
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("server request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
