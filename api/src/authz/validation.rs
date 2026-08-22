use axum::http::StatusCode;

use super::types::PermissionRule;
use crate::common::{ApiError, AppResult};

pub(crate) const ABILITY_ACTIONS: &[&str] =
    &["delete", "create", "read", "update", "manage"];

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
mod tests {
    use super::validate_permissions;
    use crate::authz::types::PermissionRule;

    fn rule(subject: &str, actions: &[&str]) -> PermissionRule {
        PermissionRule {
            subject: subject.to_owned(),
            action: actions.iter().map(|action| (*action).to_owned()).collect(),
        }
    }

    #[test]
    fn rejects_unknown_subjects_and_actions() {
        let subjects = &["Channel", "all"];
        assert!(
            validate_permissions(&[rule("Channel", &["read"])], subjects)
                .is_ok()
        );
        assert!(
            validate_permissions(&[rule("Nope", &["read"])], subjects).is_err()
        );
        assert!(validate_permissions(&[rule("Channel", &["fly"])], subjects)
            .is_err());
        assert!(
            validate_permissions(&[rule("Channel", &[])], subjects).is_err()
        );
    }
}
