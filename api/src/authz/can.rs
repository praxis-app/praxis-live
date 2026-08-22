//! The one place a permission question is answered. Call `can` from an
//! extractor so a route declares its permission next to its handler.

use axum::http::StatusCode;
use sea_orm::{prelude::Uuid, DatabaseConnection};

use crate::common::{roles::is_allowed, ApiError, AppResult};

/// Lets `can` take `"manage"` or `["create", "read"]`.
pub(crate) trait Actions {
    fn as_slice(&self) -> &[&str];
}

impl Actions for &str {
    fn as_slice(&self) -> &[&str] {
        std::slice::from_ref(self)
    }
}

impl<const N: usize> Actions for [&str; N] {
    fn as_slice(&self) -> &[&str] {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PermissionScope {
    Instance,
    Server(Uuid),
}

/// `actions` is conjunctive: every one must be granted, so it is not a way to
/// spell "either". An empty list denies rather than vacuously allowing.
pub(crate) async fn can(
    database: &DatabaseConnection,
    user_id: Uuid,
    actions: impl Actions,
    subject: &str,
    scope: PermissionScope,
) -> AppResult<()> {
    let actions = actions.as_slice();
    let granted = |rules: &[_]| {
        !actions.is_empty()
            && actions
                .iter()
                .all(|action| is_allowed(rules, subject, action))
    };

    let allowed = match scope {
        PermissionScope::Instance => {
            let rules =
                crate::instance::instance_roles::service::get_permissions_by_user(
                    database, user_id,
                )
                .await?;
            granted(&rules)
        }
        PermissionScope::Server(server_id) => {
            let permissions =
                crate::servers::server_roles::service::get_permissions_by_user(
                    database, user_id,
                )
                .await?;
            permissions
                .get(&server_id.to_string())
                .is_some_and(|rules: &Vec<_>| granted(rules))
        }
    };

    if allowed {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}
