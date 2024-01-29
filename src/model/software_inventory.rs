use serde::{Deserialize, Serialize};

use super::ODataLinks;

/// http://redfish.dmtf.org/schemas/v1/SoftwareInventory.v1_9_0.json#/definitions/SoftwareInventory
/// The SoftwareInventory schema contains an inventory of software components.
/// This can include software components such as BIOS, BMC firmware, firmware for other devices, system drivers, or provider software.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SoftwareInventory {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub description: Option<String>,
    pub id: String,
    pub version: Option<String>,
    pub release_date: Option<String>,
}
