use serde::{Deserialize, Serialize};

use super::ODataId;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SoftwareImage {
    pub software_images: Vec<ODataId>,
    pub active_software_image: ODataId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiosActions {
    #[serde(rename = "#Bios.ChangePassword")]
    pub change_password: BiosAction,
    #[serde(rename = "#Bios.ResetBios")]
    pub reset_bios: BiosAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiosAction {
    pub title: Option<String>, // Lenovo yes, Dell no
    pub target: String,        // URL path of the action
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BiosCommon {
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub attribute_registry: String,
    pub links: SoftwareImage,
    pub actions: BiosActions,
}
