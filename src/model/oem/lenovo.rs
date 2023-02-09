use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    model::{BiosCommon, ODataId},
    EnabledDisabled,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct LenovoManager {
    pub agentless_capabilities: Vec<String>,

    #[serde(rename = "KCSEnabled")]
    pub kcs_enabled: bool,

    pub recipients_settings: LenovoRecipientSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct LenovoRecipientSettings {
    pub retry_count: i64,
    pub retry_interval: f64,
    pub rntry_retry_interval: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct LenovoSystem {
    pub scheduled_power_actions: ODataId,
    #[serde(rename = "FrontPanelUSB")]
    pub front_panel_usb: LenovoFrontPanelUSB,
    pub metrics: ODataId,
    pub system_status: String,
    pub number_of_reboots: i64,
    pub history_sys_perf: ODataId,
    #[serde(rename = "@odata.type")]
    pub odata_type: String,
    pub total_power_on_hours: i64,
    pub sensors: ODataId,
    pub boot_settings: ODataId,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct LenovoFrontPanelUSB {
    inactivity_timeout_mins: i64,
    #[serde(rename = "IDButton")]
    id_button: String,
    port_switching_to: String,
    #[serde(rename = "FPMode")]
    fp_mode: LenovoFrontPanelUSBMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LenovoFrontPanelUSBMode {
    Server, // "Host Only Mode" - the secure option
    Shared, // "Shared Mode: owned by host" - the default
}

impl fmt::Display for LenovoFrontPanelUSBMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LenovoFrontPanelUSBMode::Server => f.write_str("Server"),
            LenovoFrontPanelUSBMode::Shared => f.write_str("Shared"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
// I think this is actually a string (e.g. "ubuntu" is valid), and there are more variants.
// We only use these two, so use typing checking.
pub enum LenovoBootOptionName {
    HardDisk,
    Network,
}

impl fmt::Display for LenovoBootOptionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub enum LenovoBootSource {
    None,
    Pxe,
    Cd,
    Usb,
    Hdd,
    BiosSetup,
    Diags,
    UefiTarget,
}

impl fmt::Display for LenovoBootSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Attributes part of response from Lenovo server for Systems/:id/Bios
/// There are many more attributes, see tests/bios_lenovo.json
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LenovoBiosAttributes {
    #[serde(flatten)]
    pub tpm: LenovoBiosAttributesTPM,
    #[serde(flatten)]
    pub processors: LenovoBiosAttributesProcessors,

    #[serde(rename = "Memory_MirrorMode")]
    pub memory_mirror_mode: EnabledDisabled,

    #[serde(rename = "LegacyBIOS_LegacyBIOS")]
    pub legacy_bios: EnabledDisabled,

    #[serde(rename = "BootModes_SystemBootMode")]
    pub boot_modes_system_boot_mode: LenovoBootMode,

    #[serde(rename = "SecureBootConfiguration_SecureBootStatus")]
    pub secure_boot_configuration_secure_boot_status: EnabledDisabled,
    #[serde(rename = "SecureBootConfiguration_SecureBootSetting")]
    pub secure_boot_configuration_secure_boot_setting: EnabledDisabled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LenovoBootMode {
    UEFIMode,
    LegacyMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LenovoBiosAttributesProcessors {
    #[serde(rename = "Processors_CPUPstateControl")]
    pub cpu_state_control: String,
    #[serde(rename = "Processors_AdjacentCachePrefetch")]
    pub adjacent_cache_prefetch: String,
    #[serde(rename = "Processors_HyperThreading")]
    pub hyper_threading: String,
    #[serde(rename = "Processors_IntelVirtualizationTechnology")]
    pub intel_virtualization_technology: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LenovoBiosAttributesTPM {
    #[serde(rename = "TrustedComputingGroup_DeviceOperation")]
    pub device_operation: LenovoTPMOperation,
    #[serde(rename = "TrustedComputingGroup_SHA_1PCRBank")]
    pub sha1_pcrbank: EnabledDisabled,
    #[serde(rename = "TrustedComputingGroup_DeviceStatus")]
    pub device_status: String, // "TPM2.0 Device present."
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum LenovoTPMOperation {
    None,
    UpdateToTPM2_0FirmwareVersion7_2_2_0,
    Clear, // reset
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct LenovoBios {
    #[serde(flatten)]
    pub common: BiosCommon,
    pub attributes: LenovoBiosAttributes,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_bios_parser_lenovo() {
        let test_data = include_str!("../testdata/bios_lenovo.json");
        let result: super::LenovoBios = serde_json::from_str(test_data).unwrap();
        println!("result: {:#?}", result);
    }
}
