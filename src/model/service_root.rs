use serde::{Deserialize, Serialize};

/// https://redfish.dmtf.org/schemas/v1/ServiceRoot.v1_16_0.json
/// This type shall contain information about deep operations that the service supports.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRoot {
    pub product: Option<String>,
    pub redfish_version: String,
    pub vendor: Option<String>,
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
}
