use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    channel_members, channels as channel_entities,
    enums::{ChannelType, ForumPostStatus},
    forum_posts, messages, polls, users, votes,
};
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, ConnectionTrait,
    DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect,
    Set, TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid as NativeUuid;

use super::{
    service::{get_forum_post, validate_body, validate_title},
    types::{
        MoveProposalToForumRequest, MoveProposalToForumResponse,
        ProposalForumReferenceResponse,
    },
};
use crate::{
    channels,
    common::{encryption, ApiError, AppResult},
    polls::service as polls_service,
    users as users_service,
};

pub(crate) async fn move_proposal_to_forum(
    database: &DatabaseConnection,
    server_id: Uuid,
    source_channel_id: Uuid,
    poll_id: Uuid,
    user_id: Uuid,
    request: MoveProposalToForumRequest,
) -> AppResult<MoveProposalToForumResponse> {
    let title = validate_title(&request.title)?;
    let body = validate_body(&request.body, "A forum post body is required.")?;
    let source_channel =
        channels::get_channel(database, server_id, source_channel_id).await?;
    let destination_channel = channels::get_channel(
        database,
        server_id,
        request.destination_channel_id,
    )
    .await?;
    ensure_move_channel_type(
        source_channel.channel_type,
        ChannelType::Text,
        "Proposals can only be moved from Text channels.",
    )?;
    ensure_move_channel_type(
        destination_channel.channel_type,
        ChannelType::Forum,
        "Proposals can only be moved to Forum channels.",
    )?;
    channels::ensure_channel_membership(database, source_channel_id, user_id)
        .await?;
    channels::ensure_channel_membership(
        database,
        request.destination_channel_id,
        user_id,
    )
    .await?;
    let existing_poll = polls_service::load_poll(
        database,
        server_id,
        source_channel_id,
        poll_id,
    )
    .await?;
    if existing_poll.user_id != user_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Only the proposal author can move it.",
        ));
    }
    if existing_poll.poll_type != entity::enums::PollType::Proposal {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Only proposals can be moved to a forum.",
        ));
    }
    let source_key = match existing_poll.key_id {
        Some(key_id) => {
            channels::get_unwrapped_channel_key_map(database, vec![key_id])
                .await?
                .remove(&key_id)
                .ok_or_else(|| {
                    internal_consistency_error(
                        "Proposal encryption key not found.",
                    )
                })?
        }
        None => Vec::new(),
    };
    let (destination_key, destination_unwrapped_key) =
        channels::get_unwrapped_channel_key(
            database,
            request.destination_channel_id,
        )
        .await?;
    let encrypted_title =
        encryption::encrypt_text(&title, &destination_unwrapped_key)?;
    let encrypted_body =
        encryption::encrypt_text(&body, &destination_unwrapped_key)?;
    let encrypted_proposal_body = match (
        existing_poll.ciphertext.as_deref(),
        existing_poll.iv.as_deref(),
        existing_poll.tag.as_deref(),
        existing_poll.key_id,
    ) {
        (Some(ciphertext), Some(iv), Some(tag), Some(_)) => {
            let plaintext =
                encryption::decrypt_text(ciphertext, iv, tag, &source_key)?;
            Some(encryption::encrypt_text(
                &plaintext,
                &destination_unwrapped_key,
            )?)
        }
        (None, None, None, None) => None,
        _ => {
            return Err(internal_consistency_error(
                "Proposal encryption data is incomplete.",
            ));
        }
    };
    let destination_channel_id = request.destination_channel_id;
    let now = Utc::now().fixed_offset();
    let post_id = NativeUuid::new_v4();
    let root_message_id = NativeUuid::new_v4();

    let transaction = database.begin().await.map_err(internal_error)?;
    let source_channel = load_channel_for_move(
        &transaction,
        server_id,
        source_channel_id,
        "Source channel not found.",
    )
    .await?;
    let destination_channel = load_channel_for_move(
        &transaction,
        server_id,
        destination_channel_id,
        "Destination channel not found.",
    )
    .await?;
    ensure_move_channel_type(
        source_channel.channel_type,
        ChannelType::Text,
        "Proposals can only be moved from Text channels.",
    )?;
    ensure_move_channel_type(
        destination_channel.channel_type,
        ChannelType::Forum,
        "Proposals can only be moved to Forum channels.",
    )?;
    ensure_membership_for_move(&transaction, source_channel_id, user_id)
        .await?;
    ensure_membership_for_move(&transaction, destination_channel_id, user_id)
        .await?;

    let proposal = polls::Entity::find_by_id(poll_id)
        .filter(polls::Column::ChannelId.eq(source_channel_id))
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Proposal not found.")
        })?;
    if proposal.user_id != user_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Only the proposal author can move it.",
        ));
    }
    if proposal.poll_type != entity::enums::PollType::Proposal {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Only proposals can be moved to a forum.",
        ));
    }
    if proposal.key_id != existing_poll.key_id {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "The proposal changed while it was being moved.",
        ));
    }
    if forum_posts::Entity::find()
        .filter(forum_posts::Column::PollId.eq(poll_id))
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "This proposal is already linked to a forum post.",
        ));
    }
    if votes::Entity::find()
        .filter(votes::Column::PollId.eq(poll_id))
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "Proposals with votes cannot be moved to a forum.",
        ));
    }

    messages::ActiveModel {
        id: Set(root_message_id),
        channel_id: Set(destination_channel_id),
        user_id: Set(user_id),
        key_id: Set(Some(destination_key.id)),
        ciphertext: Set(Some(encrypted_body.ciphertext)),
        iv: Set(Some(encrypted_body.iv)),
        tag: Set(Some(encrypted_body.tag)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;

    forum_posts::ActiveModel {
        id: Set(post_id),
        channel_id: Set(destination_channel_id),
        source_channel_id: Set(Some(source_channel_id)),
        user_id: Set(user_id),
        root_message_id: Set(root_message_id),
        poll_id: Set(Some(poll_id)),
        ciphertext: Set(encrypted_title.ciphertext),
        iv: Set(encrypted_title.iv),
        tag: Set(encrypted_title.tag),
        key_id: Set(destination_key.id),
        status: Set(ForumPostStatus::Open),
        latest_activity_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;

    let proposal_created_at = proposal.created_at;
    let mut proposal = proposal.into_active_model();
    proposal.channel_id = Set(destination_channel_id);
    proposal.key_id =
        Set(encrypted_proposal_body.as_ref().map(|_| destination_key.id));
    proposal.ciphertext = Set(encrypted_proposal_body
        .as_ref()
        .map(|encrypted| encrypted.ciphertext.clone()));
    proposal.iv = Set(encrypted_proposal_body
        .as_ref()
        .map(|encrypted| encrypted.iv.clone()));
    proposal.tag = Set(encrypted_proposal_body
        .as_ref()
        .map(|encrypted| encrypted.tag.clone()));
    proposal.updated_at = Set(now);
    proposal
        .update(&transaction)
        .await
        .map_err(internal_error)?;

    transaction.commit().await.map_err(internal_error)?;

    let post = get_forum_post(
        database,
        destination_channel_id,
        post_id,
        Some(user_id),
    )
    .await?;
    let source_reference = ProposalForumReferenceResponse {
        id: poll_id.to_string(),
        proposal_id: poll_id.to_string(),
        source_channel_id: source_channel_id.to_string(),
        destination_channel_id: destination_channel_id.to_string(),
        destination_channel_name: destination_channel.name.clone(),
        forum_post_id: post_id.to_string(),
        user: post.post.user.clone(),
        created_at: proposal_created_at.to_rfc3339(),
        moved_at: now.to_rfc3339(),
    };
    Ok(MoveProposalToForumResponse {
        post,
        source_reference,
        destination_channel_id,
    })
}

pub(crate) async fn list_proposal_forum_references(
    database: &DatabaseConnection,
    source_channel_id: Uuid,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<ProposalForumReferenceResponse>> {
    let posts = forum_posts::Entity::find()
        .filter(forum_posts::Column::SourceChannelId.eq(source_channel_id))
        .all(database)
        .await
        .map_err(internal_error)?;
    if posts.is_empty() {
        return Ok(vec![]);
    }

    let poll_ids = posts
        .iter()
        .filter_map(|post| post.poll_id)
        .collect::<Vec<_>>();
    let proposals = polls::Entity::find()
        .filter(polls::Column::Id.is_in(poll_ids))
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|proposal| (proposal.id, proposal))
        .collect::<HashMap<_, _>>();

    let user_ids = posts.iter().map(|post| post.user_id).collect::<Vec<_>>();
    let destination_channel_ids =
        posts.iter().map(|post| post.channel_id).collect::<Vec<_>>();
    let destination_channels = channel_entities::Entity::find()
        .filter(
            channel_entities::Column::Id
                .is_in(destination_channel_ids.iter().copied()),
        )
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|channel| (channel.id, channel))
        .collect::<HashMap<_, _>>();

    let users = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.iter().copied()))
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();

    let profile_pictures =
        users_service::get_user_profile_pictures_map(database, &user_ids)
            .await?;

    let mut references = posts
        .into_iter()
        .map(|post| {
            let poll_id = post.poll_id.ok_or_else(|| {
                internal_consistency_error(
                    "Moved forum post has no linked proposal.",
                )
            })?;
            let proposal = proposals.get(&poll_id).ok_or_else(|| {
                internal_consistency_error("Moved proposal not found.")
            })?;
            if post.poll_id != Some(proposal.id)
                || post.channel_id != proposal.channel_id
            {
                return Err(internal_consistency_error(
                    "Proposal move link is inconsistent.",
                ));
            }
            let user = users.get(&post.user_id).ok_or_else(|| {
                internal_consistency_error("Moved proposal author not found.")
            })?;
            let destination_channel = destination_channels
                .get(&post.channel_id)
                .ok_or_else(|| {
                    internal_consistency_error(
                        "Moved proposal destination channel not found.",
                    )
                })?;

            Ok(ProposalForumReferenceResponse {
                id: proposal.id.to_string(),
                proposal_id: proposal.id.to_string(),
                source_channel_id: source_channel_id.to_string(),
                destination_channel_id: post.channel_id.to_string(),
                destination_channel_name: destination_channel.name.clone(),
                forum_post_id: post.id.to_string(),
                user: crate::messages::types::MessageUser {
                    id: user.id.to_string(),
                    name: user.name.clone(),
                    display_name: user.display_name.clone(),
                    profile_picture: profile_pictures.get(&user.id).cloned(),
                },
                created_at: proposal.created_at.to_rfc3339(),
                moved_at: post.created_at.to_rfc3339(),
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    references.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(references
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect())
}

async fn load_channel_for_move<C>(
    database: &C,
    server_id: Uuid,
    channel_id: Uuid,
    not_found_message: &'static str,
) -> AppResult<channel_entities::Model>
where
    C: ConnectionTrait,
{
    channel_entities::Entity::find_by_id(channel_id)
        .filter(channel_entities::Column::ServerId.eq(server_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, not_found_message))
}

async fn ensure_membership_for_move<C>(
    database: &C,
    channel_id: Uuid,
    user_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
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

fn ensure_move_channel_type(
    actual: ChannelType,
    expected: ChannelType,
    message: &'static str,
) -> AppResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, message))
    }
}

fn internal_consistency_error(message: &'static str) -> ApiError {
    tracing::error!("forum data is inconsistent: {message}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("forum proposal move failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
