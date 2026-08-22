use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const ADMIN_ROLE_NAME: &str = "admin";
pub(crate) const DEFAULT_ROLE_COLOR: &str = "#f44336";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionRule {
    pub(crate) subject: String,
    pub(crate) action: Vec<String>,
}

pub(crate) type PermissionMap = BTreeMap<String, Vec<PermissionRule>>;
