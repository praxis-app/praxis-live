use axum::http::StatusCode;
use sea_orm::{prelude::Uuid, DatabaseConnection};

use super::types::PermissionRule;
use crate::common::{ApiError, AppResult};

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

/// If multiple `actions` are given, all of them must be granted
/// (not just one). An empty list always denies.
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

/// Whether a permission set grants `action` on `subject`. The `"all"` subject
/// matches any subject, and `manage` satisfies any action.
fn is_allowed(rules: &[PermissionRule], subject: &str, action: &str) -> bool {
    rules.iter().any(|rule| {
        (rule.subject == subject || rule.subject == "all")
            && rule
                .action
                .iter()
                .any(|granted| granted == action || granted == "manage")
    })
}
