use serde::{Deserialize, Serialize};

use crate::EnabledDisabled;

use super::ODataLinks;

/// http://redfish.dmtf.org/schemas/v1/SecureBoot.v1_0_7.json
/// The SecureBoot schema contains UEFI Secure Boot information and represents properties
/// for managing the UEFI Secure Boot functionality of a system.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SecureBoot {
    #[serde(flatten)]
    pub odata: ODataLinks,
    pub id: String,
    pub name: String,
    pub secure_boot_current_boot: EnabledDisabled,
    pub secure_boot_enable: bool,
    pub secure_boot_mode: SecureBootMode,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum SecureBootMode {
    SetupMode,
    UserMode,
    AuditMode,
    DeployedMode,
}
