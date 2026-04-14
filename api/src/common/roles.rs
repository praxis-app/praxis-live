use axum::http::StatusCode;
use casbin::{CoreApi, DefaultModel, Enforcer, MemoryAdapter, MgmtApi};
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

const RBAC_MODEL: &str = r#"
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && (p.obj == r.obj || p.obj == "all") && (p.act == r.act || p.act == "manage")
"#;

pub(crate) async fn validate_permissions(
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

    let model = DefaultModel::from_str(RBAC_MODEL)
        .await
        .map_err(internal_error)?;
    let mut enforcer = Enforcer::new(model, MemoryAdapter::default())
        .await
        .map_err(internal_error)?;

    for permission in permissions {
        for action in &permission.action {
            enforcer
                .add_policy(vec![
                    "role".to_owned(),
                    permission.subject.clone(),
                    action.clone(),
                ])
                .await
                .map_err(internal_error)?;

            let allowed = enforcer
                .enforce(("role", permission.subject.as_str(), action.as_str()))
                .map_err(internal_error)?;
            if !allowed {
                return Err(ApiError::new(
                    StatusCode::BAD_REQUEST,
                    "Permission policy is invalid.",
                ));
            }
        }
    }

    Ok(())
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!("authorization request failed: {error}");
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}
