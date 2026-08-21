use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::common::{ApiError, AppResult};

pub(crate) const ABILITY_ACTIONS: &[&str] =
    &["delete", "create", "read", "update", "manage"];
pub(crate) const ADMIN_ROLE_NAME: &str = "admin";
pub(crate) const DEFAULT_ROLE_COLOR: &str = "#f44336";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionRule {
    pub(crate) subject: String,
    pub(crate) action: Vec<String>,
}

pub(crate) type PermissionMap = BTreeMap<String, Vec<PermissionRule>>;

/// Whether a permission set grants `action` on `subject`.
///
/// The single definition of what a grant means, mirroring the CASL semantics
/// the frontend evaluates in `use-ability`: the `"all"` subject matches every
/// subject, and `manage` satisfies every action. Both rules are easy to forget
/// when the match is written out by hand, which is why call sites should ask
/// through `authz::can` rather than inspect `PermissionRule` themselves.
pub(crate) fn is_allowed(
    rules: &[PermissionRule],
    subject: &str,
    action: &str,
) -> bool {
    rules.iter().any(|rule| {
        (rule.subject == subject || rule.subject == "all")
            && rule
                .action
                .iter()
                .any(|granted| granted == action || granted == "manage")
    })
}

pub(crate) fn validate_permissions(
    permissions: &[PermissionRule],
    subjects: &[&str],
) -> AppResult<()> {
    for permission in permissions {
        if !subjects.contains(&permission.subject.as_str()) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "Permission subject is invalid.",
            ));
        }

        if permission.action.is_empty()
            || permission
                .action
                .iter()
                .any(|action| !ABILITY_ACTIONS.contains(&action.as_str()))
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "Permission action is invalid.",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
