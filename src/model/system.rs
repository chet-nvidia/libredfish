use std::fmt;

use serde::{Deserialize, Serialize};

use super::{oem::SystemExtensions, ODataId, ODataLinks};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SystemPowerControl {
    On,
    GracefulShutdown,
    ForceOff,
    GracefulRestart,
    ForceRestart,
    // Dell also has: PushPowerButton, PowerCycle, and Nmi
    // Lenovo also has: ForceOn and Nmi
}

impl fmt::Display for SystemPowerControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum PowerState {
    Off,
    On,
    PoweringOff,
    PoweringOn,
}

impl fmt::Display for PowerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct StatusState {
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Systems {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub description: String,
    pub members: Vec<ODataId>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SystemStatus {
    pub health: String,
    pub health_rollup: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SystemProcessors {
    pub count: i64,
    pub logical_processor_count: i64,
    pub model: String,
    pub status: SystemStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct TrustedModule {
    pub firmware_version: String,
    pub interface_type: String,
    pub status: StatusState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ComputerSystem {
    pub asset_tag: String,
    pub bios_version: String,
    pub manufacturer: String,
    pub model: String,
    pub oem: SystemExtensions,
    // Dell: String. Lenovo: always null
    //pub part_number: String,
    pub power_state: PowerState,
    pub processor_summary: SystemProcessors,
    #[serde(rename = "SKU")]
    pub sku: String,
    pub serial_number: String,
    pub status: SystemStatus,
    pub trusted_modules: Vec<TrustedModule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BootOptions {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub description: String,
    pub members: Vec<ODataId>,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BootOption {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub description: String,
    pub boot_option_enabled: Option<String>,
    pub boot_option_reference: String,
    pub display_name: String,
    pub id: String,
    pub name: String,
    pub uefi_device_path: String,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_systems_parser() {
        let data = include_str!("testdata/systems.json");
        let result: super::Systems = serde_json::from_str(data).unwrap();
        assert_eq!(result.members.len(), 1);
        assert_eq!(result.odata.odata_id, "/redfish/v1/Systems");
    }

    #[test]
    fn test_system_dell() {
        let data = include_str!("testdata/system_dell.json");
        let result: super::ComputerSystem = serde_json::from_str(data).unwrap();
        assert_eq!(result.power_state, crate::PowerState::On);
        assert_eq!(result.processor_summary.count, 2);
    }

    #[test]
    fn test_system_lenovo() {
        let data = include_str!("testdata/system_lenovo.json");
        let result: super::ComputerSystem = serde_json::from_str(data).unwrap();
        assert_eq!(result.oem.lenovo.unwrap().total_power_on_hours, 3816);
        assert_eq!(result.processor_summary.count, 2);
    }

    #[test]
    fn test_boot_options() {
        let data = include_str!("testdata/boot_options.json");
        let result: super::BootOptions = serde_json::from_str(data).unwrap();
        assert_eq!(result.members.len(), 5);
    }

    #[test]
    fn test_boot_option() {
        let data = include_str!("testdata/boot_option.json");
        let result: super::BootOption = serde_json::from_str(data).unwrap();
        assert_eq!(result.name, "Network");
    }
}
