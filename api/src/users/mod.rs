mod handlers;
mod models;
mod routes;
mod service;

pub(crate) use models::{
    CreateUserError, ImageReference, PublicUser, UserRecord,
};
pub(crate) use routes::router;
pub(crate) use service::{
    authenticate, create_user, get_user_profile_picture,
    get_user_profile_pictures_map, is_first_user,
};
