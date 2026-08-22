mod types;
mod validation;

pub(crate) mod can;

pub(crate) use can::{can, PermissionScope};
pub(crate) use types::{
    PermissionMap, PermissionRule, ADMIN_ROLE_NAME, DEFAULT_ROLE_COLOR,
};
pub(crate) use validation::validate_permissions;
