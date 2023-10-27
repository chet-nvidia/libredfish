use serde::{Deserialize, Serialize};

pub mod dell;
pub mod hp;
pub mod lenovo;
pub mod nvidia;
pub mod supermicro;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ManagerExtensions {
    pub dell: Option<dell::Manager>,
    pub lenovo: Option<lenovo::Manager>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SystemExtensions {
    pub dell: Option<dell::SystemWrapper>,
    pub lenovo: Option<lenovo::System>,
}
