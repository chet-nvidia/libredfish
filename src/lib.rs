mod model;
use std::collections::HashMap;

pub use model::system::{PowerState, SystemPowerControl, Systems};
pub use model::EnabledDisabled;

mod dell;
mod error;
mod lenovo;
mod network;
pub use network::NetworkConfig;
mod standard;
pub use error::RedfishError;

pub fn new(config: network::NetworkConfig) -> Result<Box<dyn Redfish>, RedfishError> {
    let s = standard::RedfishStandard::new(config)?;
    match s.vendor.as_deref() {
        Some("Dell") => Ok(Box::new(dell::Bmc::new(s)?)),
        Some("Lenovo") => Ok(Box::new(lenovo::Bmc::new(s)?)),
        _ => Ok(Box::new(s)),
    }
}

/// Interface to a BMC Redfish server. All calls will include one or more HTTP network calls.
pub trait Redfish {
    /// Is this thing even on?
    fn get_power_state(&self) -> Result<PowerState, RedfishError>;

    /// Change power state: on, off, reboot, etc
    fn power(&self, action: SystemPowerControl) -> Result<(), RedfishError>;

    /// Lock the BIOS and BMC ready for tenant use. Disabled reverses the changes.
    fn lockdown(&self, target: EnabledDisabled) -> Result<(), RedfishError>;

    /// Enable SSH access to console
    fn setup_serial_console(&self) -> Result<(), RedfishError>;

    /// Boot a single time of the given target. Does not change boot order after that.
    fn boot_once(&self, target: Boot) -> Result<(), RedfishError>;

    /// Change boot order putting this target first
    fn boot_first(&self, target: Boot) -> Result<(), RedfishError>;

    /// Reset and enable the TPM
    fn clear_tpm(&self) -> Result<(), RedfishError>;

    /*
     * Diagnostic calls
     */
    /// All the BIOS attributes for this provider. Very OEM specific.
    fn bios_attributes(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError>;

    /// Pending BIOS attributes. Changes that were requested but not applied yet because
    /// they need a reboot.
    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError>;
}

pub enum Boot {
    Pxe,
    HardDisk,
}
