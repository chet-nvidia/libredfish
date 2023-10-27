use std::fmt;

use serde::{Deserialize, Serialize};

/// https://redfish.dmtf.org/schemas/v1/ComputerSystem.v1_20_1.json
/// The boot information for this resource.
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Boot {
    pub automatic_retry_attempts: Option<i32>,
    pub automatic_retry_config: Option<AutomaticRetryConfig>,
    pub boot_next: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub boot_order: Vec<String>,
    pub boot_source_override_enabled: Option<BootSourceOverrideEnabled>,
    pub boot_source_override_target: Option<BootSourceOverrideTarget>,
    pub boot_source_override_mode: Option<BootSourceOverrideMode>,
    pub http_boot_uri: Option<String>,
    pub trusted_module_required_to_boot: Option<TrustedModuleRequiredToBoot>,
    pub uefi_target_boot_source_override: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AutomaticRetryConfig {
    Disabled,
    RetryAttempts,
    RetryAlways,
}

impl std::fmt::Display for AutomaticRetryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BootSourceOverrideEnabled {
    Once,
    Continuous,
    Disabled,
}

impl fmt::Display for BootSourceOverrideEnabled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// http://redfish.dmtf.org/schemas/v1/ComputerSystem.json#/definitions/BootSource
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BootSourceOverrideTarget {
    None,
    Pxe,
    Floppy,
    Cd,
    Usb,
    Hdd,
    BiosSetup,
    Utilities,
    Diags,
    UefiShell,
    UefiTarget,
    SDCard,
    UefiHttp,
    RemoteDrive,
    UefiBootNext,
    Recovery,
}

impl fmt::Display for BootSourceOverrideTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::upper_case_acronyms)]
pub enum BootSourceOverrideMode {
    UEFI,
    Legacy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TrustedModuleRequiredToBoot {
    Disabled,
    Required,
}

impl std::fmt::Display for TrustedModuleRequiredToBoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
