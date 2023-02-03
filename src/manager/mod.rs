use crate::common::{ODataId, ODataLinks};

mod dell;
pub use dell::*;
mod hp;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionsManagerReset {
    pub target: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Action {
    #[serde(rename = "#Manager.Reset")]
    pub manager_reset: ActionsManagerReset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Availableaction {
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Commandshell {
    pub connect_types_supported: Vec<String>,
    pub enabled: Option<bool>,
    pub max_concurrent_sessions: i64,
    pub service_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Status {
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Managers {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub description: String,
    pub members: Vec<ODataId>,
    pub name: String,
}

#[test]
fn test_manager_parser() {
    let test_data = include_str!("../../tests/manager.json");
    let result: hp::ManagerHp = serde_json::from_str(test_data).unwrap();
    println!("result: {:#?}", result);
    let test_data2 = include_str!("../../tests/manager_dell.json");
    let result2: dell::ManagerDell = serde_json::from_str(test_data2).unwrap();
    println!("result2: {:#?}", result2);
    let test_data3 = include_str!("../../tests/manager_dell_attrs.json");
    let result3: dell::OemDellAttributesResult = serde_json::from_str(test_data3).unwrap();
    println!("result3: {:#?}", result3);
}
