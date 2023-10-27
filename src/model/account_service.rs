use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ManagerAccount {
    pub id: String,
    #[serde(rename = "UserName")]
    pub username: String,
    pub name: String,
    pub description: String,
    pub role_id: String,
    pub enabled: bool,
    pub locked: bool,
}
