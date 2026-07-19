use axum::http::StatusCode;
use chrono::Utc;
use entity::{
    channel_members, channels as channel_entities,
    enums::{ChannelType, ForumPostStatus},
    forum_posts, message_images, messages, polls, users, votes,
};
use sea_orm::{
    prelude::Uuid, sea_query::Expr, ActiveModelTrait, ColumnTrait,
    ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid as NativeUuid;

use super::types::{
    CreateForumPostRequest, CreateForumReplyRequest, ForumPostResponse,
    ForumPostSummaryResponse, MoveProposalToForumRequest,
    MoveProposalToForumResponse, ProposalForumReferenceResponse,
    UpdateForumPostRequest,
};
use crate::{
    channels,
    common::{encryption, text::sanitize_text, ApiError, AppResult},
    messages::{self as messages_service, types::MessageResponse},
    polls::service as polls_service,
    pub_sub::{PubSubService, PubSubTopic},
    users as users_service,
};

const MAX_POST_TITLE_LENGTH: usize = 100;

#[derive(FromQueryResult)]
struct ReplyCount {
    thread_root_id: Option<Uuid>,
    reply_count: i64,
}

pub(crate) async fn list_forum_posts(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    sort: Option<&str>,
    status: Option<&str>,
    offset: u64,
    limit: u64,
) -> AppResult<Vec<ForumPostSummaryResponse>> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;

    let status = parse_status_filter(status)?;
    let sort = parse_sort(sort)?;
    let mut select = forum_posts::Entity::find()
        .filter(forum_posts::Column::ChannelId.eq(channel_id));
    if let Some(status) = status {
        select = select.filter(forum_posts::Column::Status.eq(status));
    }
    select = match sort {
        ForumPostSort::Recent => select
            .order_by_desc(forum_posts::Column::LatestActivityAt)
            .order_by_desc(forum_posts::Column::Id),
        ForumPostSort::Newest => select
            .order_by_desc(forum_posts::Column::CreatedAt)
            .order_by_desc(forum_posts::Column::Id),
    };

    let posts = select
        .offset(offset)
        .limit(limit)
        .all(database)
        .await
        .map_err(internal_error)?;
    shape_post_summaries(database, posts).await
}

pub(crate) async fn create_forum_post(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
    request: CreateForumPostRequest,
) -> AppResult<ForumPostResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    let title = validate_title(&request.title)?;
    let body = validate_body(&request.body, "A forum post body is required.")?;
    let prepared_proposal = match request.proposal {
        Some(proposal) => Some(
            polls_service::prepare_forum_proposal(
                database, server_id, channel_id, user_id, proposal,
            )
            .await?,
        ),
        None => None,
    };
    let (key, unwrapped_key) =
        channels::get_unwrapped_channel_key(database, channel_id).await?;
    let encrypted_title = encryption::encrypt_text(&title, &unwrapped_key)?;
    let encrypted_body = encryption::encrypt_text(&body, &unwrapped_key)?;
    let now = Utc::now().fixed_offset();
    let post_id = NativeUuid::new_v4();
    let root_message_id = NativeUuid::new_v4();

    let transaction = database.begin().await.map_err(internal_error)?;
    let proposal = match prepared_proposal {
        Some(prepared) => Some(
            polls_service::insert_prepared_poll(
                &transaction,
                None,
                user_id,
                prepared,
            )
            .await?,
        ),
        None => None,
    };

    messages::ActiveModel {
        id: Set(root_message_id),
        channel_id: Set(channel_id),
        user_id: Set(user_id),
        key_id: Set(Some(key.id)),
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
        channel_id: Set(channel_id),
        source_channel_id: Set(None),
        user_id: Set(user_id),
        root_message_id: Set(root_message_id),
        poll_id: Set(proposal.as_ref().map(|proposal| proposal.id)),
        ciphertext: Set(encrypted_title.ciphertext),
        iv: Set(encrypted_title.iv),
        tag: Set(encrypted_title.tag),
        key_id: Set(key.id),
        status: Set(ForumPostStatus::Open),
        latest_activity_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    get_forum_post(database, server_id, channel_id, post_id, user_id).await
}

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
        server_id,
        destination_channel_id,
        post_id,
        user_id,
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

pub(crate) async fn get_forum_post(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    shape_forum_post(
        database,
        load_post(database, channel_id, post_id).await?,
        user_id,
    )
    .await
}

pub(crate) async fn update_forum_post(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: UpdateForumPostRequest,
) -> AppResult<ForumPostResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    if request.title.is_none() && request.body.is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "A forum post update is required.",
        ));
    }

    let title = request.title.as_deref().map(validate_title).transpose()?;
    let body = request
        .body
        .as_deref()
        .map(|body| validate_body(body, "A forum post body is required."))
        .transpose()?;
    let encrypted = if title.is_some() || body.is_some() {
        let (key, unwrapped_key) =
            channels::get_unwrapped_channel_key(database, channel_id).await?;
        let title = title
            .as_deref()
            .map(|title| encryption::encrypt_text(title, &unwrapped_key))
            .transpose()?;
        let body = body
            .as_deref()
            .map(|body| encryption::encrypt_text(body, &unwrapped_key))
            .transpose()?;
        Some((key.id, title, body))
    } else {
        None
    };

    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(post.user_id, user_id, "Only the post author can edit it.")?;
    let root_message_id = post.root_message_id;
    let now = Utc::now().fixed_offset();
    let mut active = post.into_active_model();
    if let Some((key_id, Some(encrypted_title), _)) = encrypted.as_ref() {
        active.key_id = Set(*key_id);
        active.ciphertext = Set(encrypted_title.ciphertext.clone());
        active.iv = Set(encrypted_title.iv.clone());
        active.tag = Set(encrypted_title.tag.clone());
    }
    active.updated_at = Set(now);
    active.update(&transaction).await.map_err(internal_error)?;

    if let Some((key_id, _, Some(encrypted_body))) = encrypted {
        let root = messages::Entity::find_by_id(root_message_id)
            .one(&transaction)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| {
                internal_consistency_error("Post root message not found.")
            })?;
        let mut root = root.into_active_model();
        root.key_id = Set(Some(key_id));
        root.ciphertext = Set(Some(encrypted_body.ciphertext));
        root.iv = Set(Some(encrypted_body.iv));
        root.tag = Set(Some(encrypted_body.tag));
        root.updated_at = Set(now);
        root.update(&transaction).await.map_err(internal_error)?;
    }

    transaction.commit().await.map_err(internal_error)?;
    get_forum_post(database, server_id, channel_id, post_id, user_id).await
}

pub(crate) async fn create_forum_post_proposal(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: crate::polls::types::CreatePollRequest,
) -> AppResult<ForumPostResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    let prepared = polls_service::prepare_forum_proposal(
        database, server_id, channel_id, user_id, request,
    )
    .await?;
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(
        post.user_id,
        user_id,
        "Only the post author can create its proposal.",
    )?;
    if post.poll_id.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "This forum post already has a proposal.",
        ));
    }
    let proposal = polls_service::insert_prepared_poll(
        &transaction,
        None,
        user_id,
        prepared,
    )
    .await?;
    let mut active = post.into_active_model();
    active.poll_id = Set(Some(proposal.id));
    active.updated_at = Set(Utc::now().fixed_offset());
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    get_forum_post(database, server_id, channel_id, post_id, user_id).await
}

pub(crate) async fn close_forum_post(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    ensure_owner(post.user_id, user_id, "Only the post author can close it.")?;
    if post.status != ForumPostStatus::Closed {
        let mut active = post.into_active_model();
        active.status = Set(ForumPostStatus::Closed);
        active.updated_at = Set(Utc::now().fixed_offset());
        active.update(&transaction).await.map_err(internal_error)?;
    }
    transaction.commit().await.map_err(internal_error)?;
    get_forum_post(database, server_id, channel_id, post_id, user_id).await
}

pub(crate) async fn create_forum_reply(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    user_id: Uuid,
    request: CreateForumReplyRequest,
) -> AppResult<(MessageResponse, ForumPostSummaryResponse)> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    messages_service::validate_message_content(
        Some(&request.body),
        request.image_count,
    )?;
    let body = sanitize_text(&request.body);
    let body = (!body.is_empty()).then_some(body);
    let encrypted = match body.as_deref() {
        Some(body) => {
            let (key, unwrapped_key) =
                channels::get_unwrapped_channel_key(database, channel_id)
                    .await?;
            Some((key.id, encryption::encrypt_text(body, &unwrapped_key)?))
        }
        None => None,
    };
    let reply_id = NativeUuid::new_v4();
    let now = Utc::now().fixed_offset();

    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    if post.status == ForumPostStatus::Closed {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "Closed forum posts cannot receive replies.",
        ));
    }
    let parent_message_id =
        request.parent_message_id.unwrap_or(post.root_message_id);
    validate_reply_parent(
        &transaction,
        channel_id,
        post.root_message_id,
        parent_message_id,
    )
    .await?;

    let reply = messages::ActiveModel {
        id: Set(reply_id),
        channel_id: Set(channel_id),
        user_id: Set(user_id),
        key_id: Set(encrypted.as_ref().map(|(key_id, _)| *key_id)),
        ciphertext: Set(encrypted
            .as_ref()
            .map(|(_, value)| value.ciphertext.clone())),
        iv: Set(encrypted.as_ref().map(|(_, value)| value.iv.clone())),
        tag: Set(encrypted.as_ref().map(|(_, value)| value.tag.clone())),
        thread_root_id: Set(Some(post.root_message_id)),
        parent_message_id: Set(Some(parent_message_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&transaction)
    .await
    .map_err(internal_error)?;
    for _ in 0..request.image_count {
        message_images::ActiveModel {
            id: Set(NativeUuid::new_v4()),
            message_id: Set(reply.id),
            ..Default::default()
        }
        .insert(&transaction)
        .await
        .map_err(internal_error)?;
    }
    let mut active = post.into_active_model();
    active.latest_activity_at = Set(now);
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    let reply = messages_service::shape_messages(database, vec![reply])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Reply not found."))?;
    let post = load_post(database, channel_id, post_id).await?;
    let summary = shape_post_summaries(database, vec![post])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Post not found."))?;
    Ok((reply, summary))
}

pub(crate) async fn delete_forum_reply(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    post_id: Uuid,
    reply_id: Uuid,
    user_id: Uuid,
) -> AppResult<ForumPostSummaryResponse> {
    ensure_forum_access(database, server_id, channel_id, user_id).await?;
    let transaction = database.begin().await.map_err(internal_error)?;
    let post = load_post_for_update(&transaction, channel_id, post_id).await?;
    let reply = messages::Entity::find_by_id(reply_id)
        .filter(messages::Column::ChannelId.eq(channel_id))
        .filter(messages::Column::ThreadRootId.eq(post.root_message_id))
        .one(&transaction)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Reply not found.")
        })?;
    ensure_owner(
        reply.user_id,
        user_id,
        "Only the reply author can delete it.",
    )?;
    reply.delete(&transaction).await.map_err(internal_error)?;

    let latest_reply = messages::Entity::find()
        .filter(messages::Column::ThreadRootId.eq(post.root_message_id))
        .order_by_desc(messages::Column::CreatedAt)
        .one(&transaction)
        .await
        .map_err(internal_error)?;
    let latest_activity_at = latest_reply
        .map(|reply| reply.created_at)
        .unwrap_or(post.created_at);
    let mut active = post.into_active_model();
    active.latest_activity_at = Set(latest_activity_at);
    active.update(&transaction).await.map_err(internal_error)?;
    transaction.commit().await.map_err(internal_error)?;

    let post = load_post(database, channel_id, post_id).await?;
    shape_post_summaries(database, vec![post])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Post not found."))
}

pub(crate) async fn broadcast_forum_post(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    action: &'static str,
    post: &ForumPostResponse,
) {
    broadcast_event(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        sender_id,
        serde_json::json!({
            "type": "forumPost",
            "action": action,
            "post": post,
        }),
    )
    .await;
}

pub(crate) async fn broadcast_proposal_forum_reference(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    source_channel_id: Uuid,
    sender_id: Uuid,
    reference: &ProposalForumReferenceResponse,
) {
    let members = match channels::get_channel_member_user_ids(
        database,
        source_channel_id,
    )
    .await
    {
        Ok(members) => members,
        Err(error) => {
            tracing::warn!("failed to load proposal move recipients: {error}");
            return;
        }
    };
    let body = serde_json::json!({
        "type": "proposalMoved",
        "reference": reference,
    });
    for member_id in members {
        if member_id == sender_id {
            continue;
        }
        let topic =
            PubSubTopic::new_poll(server_id, source_channel_id, member_id)
                .to_string();
        if let Err(error) = pub_sub_service.publish(&topic, body.clone()).await
        {
            tracing::warn!("failed to broadcast proposal move: {error}");
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn broadcast_forum_reply(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    action: &'static str,
    post_id: Uuid,
    reply: Option<&MessageResponse>,
    reply_id: Option<Uuid>,
    post: &ForumPostSummaryResponse,
) {
    broadcast_event(
        database,
        pub_sub_service,
        server_id,
        channel_id,
        sender_id,
        serde_json::json!({
            "type": "forumReply",
            "action": action,
            "postId": post_id,
            "reply": reply,
            "replyId": reply_id,
            "post": post,
        }),
    )
    .await;
}

async fn shape_forum_post(
    database: &DatabaseConnection,
    post: forum_posts::Model,
    user_id: Uuid,
) -> AppResult<ForumPostResponse> {
    let root = messages::Entity::find_by_id(post.root_message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            internal_consistency_error("Post root message not found.")
        })?;
    let replies = messages::Entity::find()
        .filter(messages::Column::ThreadRootId.eq(post.root_message_id))
        .order_by_asc(messages::Column::CreatedAt)
        .all(database)
        .await
        .map_err(internal_error)?;
    let mut records = Vec::with_capacity(replies.len() + 1);
    records.push(root);
    records.extend(replies);
    let mut shaped_messages =
        messages_service::shape_messages(database, records)
            .await?
            .into_iter();
    let root = shaped_messages.next().ok_or_else(|| {
        internal_consistency_error("Post root message not found.")
    })?;
    let proposal = match post.poll_id {
        Some(poll_id) => Some(
            polls_service::get_poll_response(
                database,
                Uuid::nil(),
                post.channel_id,
                poll_id,
                Some(user_id),
            )
            .await?,
        ),
        None => None,
    };
    let summary = shape_post_summaries(database, vec![post])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| internal_consistency_error("Post not found."))?;

    Ok(ForumPostResponse {
        post: summary,
        body: root.body.unwrap_or_default(),
        replies: shaped_messages.collect(),
        proposal,
    })
}

async fn shape_post_summaries(
    database: &DatabaseConnection,
    posts: Vec<forum_posts::Model>,
) -> AppResult<Vec<ForumPostSummaryResponse>> {
    if posts.is_empty() {
        return Ok(vec![]);
    }

    let user_ids = posts.iter().map(|post| post.user_id).collect::<Vec<_>>();
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
    let key_map = channels::get_unwrapped_channel_key_map(
        database,
        posts.iter().map(|post| post.key_id).collect(),
    )
    .await?;
    let root_ids = posts
        .iter()
        .map(|post| post.root_message_id)
        .collect::<Vec<_>>();
    let reply_counts = messages::Entity::find()
        .select_only()
        .column(messages::Column::ThreadRootId)
        .column_as(Expr::col(messages::Column::Id).count(), "reply_count")
        .filter(messages::Column::ThreadRootId.is_in(root_ids))
        .group_by(messages::Column::ThreadRootId)
        .into_model::<ReplyCount>()
        .all(database)
        .await
        .map_err(internal_error)?
        .into_iter()
        .filter_map(|count| {
            count
                .thread_root_id
                .map(|root_id| (root_id, count.reply_count as usize))
        })
        .collect::<HashMap<_, _>>();

    posts
        .into_iter()
        .map(|post| {
            let user = users.get(&post.user_id).ok_or_else(|| {
                internal_consistency_error("Post author not found.")
            })?;
            let key = key_map.get(&post.key_id).ok_or_else(|| {
                internal_consistency_error("Post encryption key not found.")
            })?;
            let title = encryption::decrypt_text(
                &post.ciphertext,
                &post.iv,
                &post.tag,
                key,
            )?;
            Ok(ForumPostSummaryResponse {
                id: post.id.to_string(),
                title,
                root_message_id: post.root_message_id.to_string(),
                poll_id: post.poll_id.map(|id| id.to_string()),
                status: post.status.to_string(),
                user: crate::messages::types::MessageUser {
                    id: user.id.to_string(),
                    name: user.name.clone(),
                    display_name: user.display_name.clone(),
                    profile_picture: profile_pictures.get(&user.id).cloned(),
                },
                reply_count: reply_counts
                    .get(&post.root_message_id)
                    .copied()
                    .unwrap_or_default(),
                latest_activity_at: post.latest_activity_at.to_rfc3339(),
                created_at: post.created_at.to_rfc3339(),
                updated_at: post.updated_at.to_rfc3339(),
            })
        })
        .collect()
}

async fn ensure_forum_access(
    database: &DatabaseConnection,
    server_id: Uuid,
    channel_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let channel =
        channels::get_channel(database, server_id, channel_id).await?;
    channels::ensure_channel_membership(database, channel_id, user_id).await?;
    if channel.channel_type == ChannelType::Forum {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "Forum channel not found.",
        ))
    }
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

async fn load_post<C>(
    database: &C,
    channel_id: Uuid,
    post_id: Uuid,
) -> AppResult<forum_posts::Model>
where
    C: ConnectionTrait,
{
    forum_posts::Entity::find_by_id(post_id)
        .filter(forum_posts::Column::ChannelId.eq(channel_id))
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Post not found."))
}

async fn load_post_for_update<C>(
    database: &C,
    channel_id: Uuid,
    post_id: Uuid,
) -> AppResult<forum_posts::Model>
where
    C: ConnectionTrait,
{
    forum_posts::Entity::find_by_id(post_id)
        .filter(forum_posts::Column::ChannelId.eq(channel_id))
        .lock_exclusive()
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "Post not found."))
}

async fn validate_reply_parent<C>(
    database: &C,
    channel_id: Uuid,
    root_message_id: Uuid,
    parent_message_id: Uuid,
) -> AppResult<()>
where
    C: ConnectionTrait,
{
    let parent = messages::Entity::find_by_id(parent_message_id)
        .one(database)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "Parent message not found.")
        })?;
    if parent.channel_id != channel_id
        || (parent.id != root_message_id
            && parent.thread_root_id != Some(root_message_id))
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Parent message must belong to the same forum post.",
        ));
    }
    Ok(())
}

async fn broadcast_event(
    database: &DatabaseConnection,
    pub_sub_service: &PubSubService,
    server_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    body: serde_json::Value,
) {
    let members =
        match channels::get_channel_member_user_ids(database, channel_id).await
        {
            Ok(members) => members,
            Err(error) => {
                tracing::warn!(
                    "failed to load forum event recipients: {error}"
                );
                return;
            }
        };
    for member_id in members {
        if member_id == sender_id {
            continue;
        }
        let topic = PubSubTopic::forum_posts(server_id, channel_id, member_id)
            .to_string();
        if let Err(error) = pub_sub_service.publish(&topic, body.clone()).await
        {
            tracing::warn!("failed to broadcast forum event: {error}");
        }
    }
}

fn validate_title(value: &str) -> AppResult<String> {
    let title = sanitize_text(value);
    let length = title.chars().count();
    if (1..=MAX_POST_TITLE_LENGTH).contains(&length) {
        Ok(title)
    } else {
        Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Forum post title must be between 1 and {MAX_POST_TITLE_LENGTH} characters."
            ),
        ))
    }
}

fn validate_body(value: &str, message: &'static str) -> AppResult<String> {
    let body = sanitize_text(value);
    if body.is_empty() {
        Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, message))
    } else {
        Ok(body)
    }
}

fn ensure_owner(
    owner_id: Uuid,
    user_id: Uuid,
    message: &'static str,
) -> AppResult<()> {
    if owner_id == user_id {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, message))
    }
}

#[derive(Clone, Copy)]
enum ForumPostSort {
    Recent,
    Newest,
}

fn parse_sort(value: Option<&str>) -> AppResult<ForumPostSort> {
    match value.unwrap_or("recent") {
        "recent" => Ok(ForumPostSort::Recent),
        "newest" => Ok(ForumPostSort::Newest),
        _ => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Forum post sort must be recent or newest.",
        )),
    }
}

fn parse_status_filter(
    value: Option<&str>,
) -> AppResult<Option<ForumPostStatus>> {
    match value {
        None => Ok(None),
        Some("open") => Ok(Some(ForumPostStatus::Open)),
        Some("closed") => Ok(Some(ForumPostStatus::Closed)),
        Some(_) => Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Forum post status must be open or closed.",
        )),
    }
}

fn internal_consistency_error(message: &'static str) -> ApiError {
    tracing::error!("forum data is inconsistent: {message}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("forum request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
