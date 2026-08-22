mod defaults;
mod types;
mod validation;

pub(crate) mod can;

pub(crate) use can::{can, PermissionScope};
pub(crate) use defaults::{ADMIN_ROLE_NAME, DEFAULT_ROLE_COLOR};
pub(crate) use types::{PermissionMap, PermissionRule};
pub(crate) use validation::validate_permissions;
