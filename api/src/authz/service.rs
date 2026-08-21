//! The one place a permission question is answered.
//!
//! Ports the `can(action, subject, scope)` middleware the legacy Express app
//! used (`common/roles/can.middleware.ts`): resolve the caller's permissions
//! for a scope, then ask whether they grant the action on the subject. Call it
//! from an extractor so a route declares its permission next to its handler
//! and cannot ship without one, and so the `"all"` and `manage` semantics have
//! exactly one implementation instead of one per domain.

use axum::http::StatusCode;
use sea_orm::{prelude::Uuid, DatabaseConnection};

use crate::common::{roles::is_allowed, ApiError, AppResult};

/// Which permission set answers the question. Instance-level authority is
/// separate from, and never implied by, authority within a single server.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PermissionScope {
    Instance,
    Server(Uuid),
}

/// `Ok(())` when the caller may take `action` on `subject` in `scope`,
/// otherwise `403`. The error carries no detail about what was missing.
pub(crate) async fn can(
    database: &DatabaseConnection,
    user_id: Uuid,
    action: &str,
    subject: &str,
    scope: PermissionScope,
) -> AppResult<()> {
    let allowed = match scope {
        PermissionScope::Instance => {
            let rules =
                crate::instance::instance_roles::service::get_permissions_by_user(
                    database, user_id,
                )
                .await?;
            is_allowed(&rules, subject, action)
        }
        PermissionScope::Server(server_id) => {
            let permissions =
                crate::servers::server_roles::service::get_permissions_by_user(
                    database, user_id,
                )
                .await?;
            permissions
                .get(&server_id.to_string())
                .is_some_and(|rules| is_allowed(rules, subject, action))
        }
    };

    if allowed {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::FORBIDDEN, "Forbidden."))
    }
}
