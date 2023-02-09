use serde::{Deserialize, Serialize};

use crate::model::{Action, ActionsManagerReset, Availableaction, Commandshell, Status, StatusT};
use crate::model::{Firmware, LinkType, ODataId, ODataLinks, StatusVec};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct HpManager {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub actions: Action,
    pub available_actions: Vec<Availableaction>,
    pub command_shell: Commandshell,
    pub description: String,
    pub ethernet_interfaces: ODataId,
    pub firmware: Firmware,
    pub firmware_version: String,
    pub graphical_console: Commandshell,
    pub id: String,
    pub log_services: ODataId,
    pub manager_type: String,
    pub name: String,
    pub network_protocol: ODataId,
    pub oem: OemHpWrapper,
    pub serial_console: Commandshell,
    pub status: Status,
    #[serde(rename = "Type")]
    pub root_type: String,
    #[serde(rename = "UUID")]
    pub uuid: String,
    pub virtual_media: ODataId,
}

impl StatusVec for HpManager {
    fn get_vec(&self) -> Vec<Box<dyn StatusT>> {
        let mut v: Vec<Box<dyn StatusT>> = Vec::new();
        for res in &self.oem.hp.i_lo_self_test_results {
            v.push(Box::new(res.clone()))
        }
        v
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OemHpActionshpiloResetToFactoryDefault {
    #[serde(rename = "ResetType@Redfish.AllowableValues")]
    pub reset_type_redfish_allowable_values: Vec<String>,
    pub target: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OemHpAction {
    #[serde(rename = "#HpiLO.ClearRestApiState")]
    pub hpi_lo_clear_rest_api_state: ActionsManagerReset,
    #[serde(rename = "#HpiLO.ResetToFactoryDefaults")]
    pub hpi_lo_reset_to_factory_defaults: OemHpActionshpiloResetToFactoryDefault,
    #[serde(rename = "#HpiLO.iLOFunctionality")]
    pub hpi_lo_i_lo_functionality: ActionsManagerReset,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpAvailableactionsCapability {
    pub allowable_values: Vec<String>,
    pub property_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpAvailableaction {
    pub action: String,
    pub capabilities: Vec<OemHpAvailableactionsCapability>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpFederationconfig {
    #[serde(rename = "IPv6MulticastScope")]
    pub i_pv6_multicast_scope: String,
    pub multicast_announcement_interval: i64,
    pub multicast_discovery: String,
    pub multicast_time_to_live: i64,
    #[serde(rename = "iLOFederationManagement")]
    pub i_lo_federation_management: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpFirmwareCurrent {
    pub date: String,
    pub debug_build: bool,
    pub major_version: i64,
    pub minor_version: i64,
    pub time: String,
    pub version_string: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpFirmware {
    pub current: OemHpFirmwareCurrent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpLicense {
    pub license_key: String,
    pub license_string: String,
    pub license_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpIloselftestresult {
    pub notes: String,
    pub self_test_name: String,
    pub status: String,
}
impl StatusT for OemHpIloselftestresult {
    fn health(&self) -> String {
        self.status.to_owned()
    }

    fn state(&self) -> String {
        String::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHp {
    #[serde(flatten)]
    pub oem_type: HpType,
    pub actions: OemHpAction,
    pub available_actions: Vec<OemHpAvailableaction>,
    pub clear_rest_api_status: String,
    pub federation_config: OemHpFederationconfig,
    pub firmware: OemHpFirmware,
    pub license: OemHpLicense,
    #[serde(rename = "RequiredLoginForiLORBSU")]
    pub required_login_fori_lorbsu: bool,
    #[serde(rename = "SerialCLISpeed")]
    pub serial_cli_speed: i64,
    #[serde(rename = "SerialCLIStatus")]
    pub serial_cli_status: String,
    #[serde(rename = "VSPLogDownloadEnabled")]
    pub vsp_log_download_enabled: bool,
    #[serde(rename = "iLOSelfTestResults")]
    pub i_lo_self_test_results: Vec<OemHpIloselftestresult>,
    #[serde(rename = "links", flatten)]
    pub links: LinkType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct OemHpWrapper {
    pub hp: OemHp,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct HpType {
    #[serde(rename = "@odata.type")]
    pub odata_type: String,
    #[serde(rename = "Type")]
    pub hp_type: String,
}
