use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ManagerAccount {
    // Set on GET. Do not set for POST/PATCH
    pub id: Option<String>,

    #[serde(rename = "UserName")]
    pub username: String,

    // Set this for POST/PATCH. Not populated by GET.
    pub password: Option<String>,

    // A RoleId converted to string
    pub role_id: String,

    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub locked: Option<bool>,
}

impl Ord for ManagerAccount {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for ManagerAccount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ManagerAccount {
    fn eq(&self, other: &ManagerAccount) -> bool {
        self.id == other.id
    }
}
