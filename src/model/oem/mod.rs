use serde::{Deserialize, Serialize};

pub mod dell;
pub mod hp;
pub mod lenovo;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ManagerExtensions {
    pub dell: Option<dell::DellManager>,
    pub lenovo: Option<lenovo::LenovoManager>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SystemExtensions {
    pub dell: Option<dell::DellSystemWrapper>,
    pub lenovo: Option<lenovo::LenovoSystem>,
}
