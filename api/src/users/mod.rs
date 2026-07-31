mod handlers;
mod models;
mod routes;
mod service;
pub(crate) mod types;

pub(crate) use models::UserRecord;
pub(crate) use routes::router;
pub(crate) use service::{
    authenticate, create_anon_user, create_user, get_user_by_id,
    get_user_profile_picture, get_user_profile_pictures_map, is_first_user,
    upgrade_anon_user,
};
pub(crate) use types::{CreateUserError, PublicUser, UserImageRef};
