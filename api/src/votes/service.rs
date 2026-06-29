use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    calls, enums::VoteType, poll_actions as poll_action_entities, poll_configs,
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
    invites, poll_actions,
    polls::service as polls_service,
    users as users_service,
};

#[cfg(test)]
mod tests;

pub(crate) async fn create_vote(
    database: &DatabaseConnection,
    poll: polls::Model,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<CreateVoteResponse> {
    ensure_poll_accepts_vote_mutations(database, &poll).await?;
    validate_vote_request(&poll, &request)?;

    if vote_entities::Entity::find()
        .filter(vote_entities::Column::PollId.eq(poll.id))
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
    validate_poll_option_ids(database, poll.id, &poll_option_ids).await?;
    let vote = vote_entities::ActiveModel {
        id: Set(NativeUuid::new_v4()),
        poll_id: Set(poll.id),
        user_id: Set(user_id),
        vote_type: Set(parse_vote_type_value(request.vote_type.as_deref())?),
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
        vote_type: vote.vote_type.map(|value| value.to_string()),
        poll_option_ids: (!poll_option_ids.is_empty())
            .then(|| poll_option_ids.iter().map(ToString::to_string).collect()),
        is_ratifying_vote,
    })
}

pub(crate) async fn update_vote(
    database: &DatabaseConnection,
    poll: polls::Model,
    vote_id: Uuid,
    user_id: Uuid,
    request: VoteRequest,
) -> AppResult<UpdateVoteResponse> {
    ensure_poll_accepts_vote_mutations(database, &poll).await?;
    validate_vote_request(&poll, &request)?;
    let vote = vote_entities::Entity::find_by_id(vote_id)
        .filter(vote_entities::Column::PollId.eq(poll.id))
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
    validate_poll_option_ids(database, poll.id, &poll_option_ids).await?;
    let mut active = vote.into_active_model();
    active.vote_type =
        Set(parse_vote_type_value(request.vote_type.as_deref())?);
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
    poll: &polls::Model,
    vote_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    ensure_poll_accepts_vote_mutations(database, poll).await?;
    let vote = vote_entities::Entity::find_by_id(vote_id)
        .filter(vote_entities::Column::PollId.eq(poll.id))
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
    _poll_id: Uuid,
    poll_option_id: Uuid,
) -> AppResult<Vec<PollOptionVoterResponse>> {
    let selections = poll_option_selections::Entity::find()
        .filter(poll_option_selections::Column::PollOptionId.eq(poll_option_id))
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

pub(crate) async fn ensure_can_read_poll_option(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    poll_id: Uuid,
    current_user_id: Option<Uuid>,
    invite_token: Option<&str>,
) -> AppResult<()> {
    if let Some(user_id) = current_user_id {
        return channels::ensure_channel_membership(
            database, channel_id, user_id,
        )
        .await;
    }

    if polls_service::is_public_channel_poll(
        database, server_id, channel_id, poll_id,
    )
    .await?
    {
        return Ok(());
    }

    if let Some(invite_token) = invite_token {
        if invites::service::is_valid_invite_for_server(
            database,
            invite_token,
            server_id,
        )
        .await?
        {
            return Ok(());
        }
    }

    Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
}

pub(crate) async fn ensure_poll_option_exists(
    database: &DatabaseConnection,
    poll_id: Uuid,
    poll_option_id: Uuid,
) -> AppResult<()> {
    poll_options::Entity::find_by_id(poll_option_id)
        .filter(poll_options::Column::PollId.eq(poll_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll option not found.")
        })?;

    Ok(())
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
        vote_type: vote.vote_type.map(|value| value.to_string()),
        poll_option_ids: (!poll_option_ids.is_empty())
            .then_some(poll_option_ids),
    }
}

fn validate_vote_request(
    poll: &polls::Model,
    request: &VoteRequest,
) -> AppResult<()> {
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
    Ok(())
}

pub(crate) async fn ensure_anonymous_can_vote_on_poll(
    database: &DatabaseConnection,
    user_id: Uuid,
    poll: &polls::Model,
) -> AppResult<()> {
    if poll.poll_type != "proposal" {
        return Ok(());
    }

    let user = users::Entity::find_by_id(user_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required.")
        })?;
    if !user.anonymous {
        return Ok(());
    }

    let action = poll_action_entities::Entity::find()
        .filter(poll_action_entities::Column::PollId.eq(poll.id))
        .one(database)
        .await
        .map_err(internal_error)?;
    if action.as_ref().map(|action| action.action_type.as_str()) == Some("test")
    {
        return Ok(());
    }

    Err(ApiError::new(
        StatusCode::FORBIDDEN,
        "Only registered users can vote on non-test proposals.",
    ))
}

pub(crate) async fn ensure_poll_accepts_vote_mutations(
    database: &DatabaseConnection,
    poll: &polls::Model,
) -> AppResult<()> {
    let config = poll_configs::Entity::find()
        .filter(poll_configs::Column::PollId.eq(poll.id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Poll config not found.")
        })?;
    ensure_before_voting_deadline(
        config.closing_at,
        Utc::now().fixed_offset(),
    )?;

    let Some(call_id) = poll.call_id else {
        return Ok(());
    };

    let call = calls::Entity::find_by_id(call_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Call not found.")
        })?;

    if matches!(call.status.as_str(), "starting" | "active") {
        return Ok(());
    }

    Err(ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Call has ended. Votes can no longer be changed.",
    ))
}

fn ensure_before_voting_deadline(
    closing_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> AppResult<()> {
    // The deadline is exclusive: vote mutations are rejected when now >= closing_at.
    if closing_at.is_some_and(|closing_at| now >= closing_at) {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "Voting deadline has passed.",
        ));
    }

    Ok(())
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
    parse_vote_type_value(vote_type).and_then(|value| {
        value.map(|_| ()).ok_or_else(|| {
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid vote type.",
            )
        })
    })
}

fn parse_vote_type_value(
    vote_type: Option<&str>,
) -> AppResult<Option<VoteType>> {
    vote_type
        .map(|value| {
            value.parse().map_err(|_| {
                ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Invalid vote type.",
                )
            })
        })
        .transpose()
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
