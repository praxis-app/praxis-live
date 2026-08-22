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

#[cfg(test)]
mod tests {
    use super::is_allowed;
    use crate::authz::types::PermissionRule;

    fn rule(subject: &str, actions: &[&str]) -> PermissionRule {
        PermissionRule {
            subject: subject.to_owned(),
            action: actions.iter().map(|action| (*action).to_owned()).collect(),
        }
    }

    #[test]
    fn exact_subject_and_action_is_allowed() {
        let rules = vec![rule("Channel", &["create"])];
        assert!(is_allowed(&rules, "Channel", "create"));
    }

    #[test]
    fn other_subjects_and_actions_are_denied() {
        let rules = vec![rule("Channel", &["create"])];
        assert!(!is_allowed(&rules, "Invite", "create"));
        assert!(!is_allowed(&rules, "Channel", "delete"));
    }

    #[test]
    fn empty_permissions_deny_everything() {
        assert!(!is_allowed(&[], "Channel", "read"));
    }

    // `manage` is the widest action: holding it satisfies every narrower one.
    // Checking for a narrower action alone is the mistake this guards against.
    #[test]
    fn manage_satisfies_every_action() {
        let rules = vec![rule("ServerRole", &["manage"])];
        for action in ["delete", "create", "read", "update", "manage"] {
            assert!(is_allowed(&rules, "ServerRole", action), "{action}");
        }
    }

    // The `all` subject is the widest subject, and is just as easy to forget.
    #[test]
    fn the_all_subject_matches_every_subject() {
        let rules = vec![rule("all", &["read"])];
        assert!(is_allowed(&rules, "Channel", "read"));
        assert!(is_allowed(&rules, "InstanceRole", "read"));
        assert!(!is_allowed(&rules, "Channel", "delete"));
    }

    #[test]
    fn all_combined_with_manage_grants_everything() {
        let rules = vec![rule("all", &["manage"])];
        assert!(is_allowed(&rules, "Invite", "delete"));
        assert!(is_allowed(&rules, "Server", "create"));
    }

    #[test]
    fn a_grant_is_found_across_separate_rules() {
        let rules =
            vec![rule("Channel", &["read"]), rule("Invite", &["manage"])];
        assert!(is_allowed(&rules, "Invite", "create"));
        assert!(!is_allowed(&rules, "Channel", "create"));
    }
}
