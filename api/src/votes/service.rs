use axum::http::StatusCode;
use entity::{
    poll_option_selections, poll_options, polls, users, votes as vote_entities,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use uuid::Uuid as NativeUuid;

use super::types::{
    CreateVoteResponse, PollOptionVoterResponse, UpdateVoteResponse,
    VoteRequest, VoteResponse,
};
use crate::{
    channels,
    common::{request::parse_uuid, ApiError, AppResult},
    poll_actions,
    polls::service as polls_service,
    users as users_service,
};

pub(crate) async fn create_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<CreateVoteResponse> {
    let poll = validate_vote_request(
        database, server_id, channel_id, poll_id, &request,
    )
    .await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;

    if vote_entities::Entity::find()
        .filter(vote_entities::Column::PollId.eq(poll_id))
        .filter(vote_entities::Column::UserId.eq(user_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "You have already voted on this poll.",
        ));
    }

    let poll_option_ids = parse_poll_option_ids(&request)?;
    validate_poll_option_ids(database, poll_id, &poll_option_ids).await?;
    let vote = vote_entities::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll_id),
        user_id: Set(user_id),
        vote_type: Set(request.vote_type.clone()),
        ..Default::default()
    }
    .insert(database)
    .await
    .map_err(internal_error)?;

    save_poll_option_selections(database, vote.id, &poll_option_ids).await?;
    let is_ratifying_vote =
        synchronize_ratification_after_vote(database, &poll).await?;

    Ok(CreateVoteResponse {
        id: vote.id.to_string(),
        poll_id: vote.poll_id.to_string(),
        user_id: vote.user_id.to_string(),
        vote_type: vote.vote_type,
        poll_option_ids: (!poll_option_ids.is_empty())
            .then(|| poll_option_ids.iter().map(ToString::to_string).collect()),
        is_ratifying_vote,
    })
}

pub(crate) async fn update_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    vote_id: Uuid,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<UpdateVoteResponse> {
    let poll = validate_vote_request(
        database, server_id, channel_id, poll_id, &request,
    )
    .await?;
    let vote = vote_entities::Entity::find_by_id(vote_id)
        .filter(vote_entities::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Vote not found.")
        })?;
    if vote.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }

    let poll_option_ids = parse_poll_option_ids(&request)?;
    validate_poll_option_ids(database, poll_id, &poll_option_ids).await?;
    let mut active = vote.into_active_model();
    active.vote_type = Set(request.vote_type);
    active.update(database).await.map_err(internal_error)?;

    poll_option_selections::Entity::delete_many()
        .filter(poll_option_selections::Column::VoteId.eq(vote_id))
        .exec(database)
        .await
        .map_err(internal_error)?;
    save_poll_option_selections(database, vote_id, &poll_option_ids).await?;

    let is_ratifying_vote =
        synchronize_ratification_after_vote(database, &poll).await?;
    Ok(UpdateVoteResponse { is_ratifying_vote })
}

pub(crate) async fn delete_vote(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    vote_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let poll =
        polls_service::load_poll(database, server_id, channel_id, poll_id)
            .await?;
    if poll.stage != "voting" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll is no longer accepting votes.",
        ));
    }

    let vote = vote_entities::Entity::find_by_id(vote_id)
        .filter(vote_entities::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Vote not found.")
        })?;
    if vote.user_id != user_id {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."));
    }
    vote_entities::Entity::delete_by_id(vote_id)
        .exec(database)
        .await
        .map_err(internal_error)?;
    Ok(())
}

pub(crate) async fn get_voters_by_poll_option(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    poll_option_id: Uuid,
) -> AppResult<Vec<PollOptionVoterResponse>> {
    polls_service::load_poll(database, server_id, channel_id, poll_id).await?;
    let option = poll_options::Entity::find_by_id(poll_option_id)
        .filter(poll_options::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll option not found.")
        })?;

    let selections = poll_option_selections::Entity::find()
        .filter(poll_option_selections::Column::PollOptionId.eq(option.id))
        .all(database)
        .await
        .map_err(internal_error)?;
    let vote_ids: Vec<Uuid> = selections
        .iter()
        .map(|selection| selection.vote_id)
        .collect();
    if vote_ids.is_empty() {
        return Ok(vec![]);
    }

    let votes = vote_entities::Entity::find()
        .filter(vote_entities::Column::Id.is_in(vote_ids))
        .all(database)
        .await
        .map_err(internal_error)?;
    let user_ids: Vec<Uuid> = votes.iter().map(|vote| vote.user_id).collect();
    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.clone()))
        .all(database)
        .await
        .map_err(internal_error)?;
    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;

    Ok(users
        .into_iter()
        .map(|user| PollOptionVoterResponse {
            id: user.id.to_string(),
            name: user.name,
            display_name: user.display_name,
            profile_picture: profile_pictures.get(&user.id).cloned(),
        })
        .collect())
}

pub(crate) fn shape_vote(
    vote: &vote_entities::Model,
    selections: &[poll_option_selections::Model],
) -> VoteResponse {
    let poll_option_ids = selections
        .iter()
        .filter(|selection| selection.vote_id == vote.id)
        .map(|selection| selection.poll_option_id.to_string())
        .collect::<Vec<_>>();
    VoteResponse {
        id: vote.id.to_string(),
        vote_type: vote.vote_type.clone(),
        poll_option_ids: (!poll_option_ids.is_empty())
            .then_some(poll_option_ids),
    }
}

async fn validate_vote_request(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    request: &VoteRequest,
) -> AppResult<polls::Model> {
    let poll =
        polls_service::load_poll(database, server_id, channel_id, poll_id)
            .await?;
    if poll.stage != "voting" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll is no longer accepting votes.",
        ));
    }
    if poll.poll_type == "proposal" {
        validate_vote_type(request.vote_type.as_deref())?;
    } else if request
        .poll_option_ids
        .as_ref()
        .map(Vec::is_empty)
        .unwrap_or(true)
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "At least one poll option must be selected.",
        ));
    }
    Ok(poll)
}

async fn save_poll_option_selections(
    database: &DatabaseConnection,
    vote_id: Uuid,
    poll_option_ids: &[Uuid],
) -> AppResult<()> {
    for poll_option_id in poll_option_ids {
        poll_option_selections::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            vote_id: Set(vote_id),
            poll_option_id: Set(*poll_option_id),
            ..Default::default()
        }
        .insert(database)
        .await
        .map_err(internal_error)?;
    }
    Ok(())
}

async fn synchronize_ratification_after_vote(
    database: &DatabaseConnection,
    poll: &polls::Model,
) -> AppResult<bool> {
    if poll.poll_type != "proposal"
        || !polls_service::is_poll_ratifiable(database, poll.id).await?
    {
        return Ok(false);
    }
    polls_service::ratify_poll(database, poll.id).await?;
    poll_actions::service::implement_poll_action(database, poll.id).await?;
    Ok(true)
}

fn validate_vote_type(vote_type: Option<&str>) -> AppResult<()> {
    if matches!(vote_type, Some("agree" | "disagree" | "abstain" | "block")) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid vote type.",
        ))
    }
}

fn parse_poll_option_ids(request: &VoteRequest) -> AppResult<Vec<Uuid>> {
    request
        .poll_option_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|id| parse_uuid(id, "pollOptionId"))
        .collect()
}

async fn validate_poll_option_ids(
    database: &DatabaseConnection,
    poll_id: Uuid,
    poll_option_ids: &[Uuid],
) -> AppResult<()> {
    if poll_option_ids.is_empty() {
        return Ok(());
    }

    let count = poll_options::Entity::find()
        .filter(poll_options::Column::PollId.eq(poll_id))
        .filter(poll_options::Column::Id.is_in(poll_option_ids.to_vec()))
        .count(database)
        .await
        .map_err(internal_error)?;

    if count as usize == poll_option_ids.len() {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Poll option is invalid.",
        ))
    }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("vote request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
