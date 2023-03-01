use std::collections::HashMap;

pub mod model;
pub use model::system::{PowerState, SystemPowerControl, Systems};
pub use model::EnabledDisabled;
use serde::{Deserialize, Serialize};

mod dell;
mod error;
mod lenovo;
mod network;
pub use network::{NetworkConfig, REDFISH_ENDPOINT};
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

    /// Are the BIOS and BMC currently locked down?
    fn lockdown_status(&self) -> Result<LockdownStatus, RedfishError>;

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

// When Carbide drops it's `IpmiCommand.launch_command` background job system, we can
// remove the Serialize and Deserialize here.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Boot {
    Pxe,
    HardDisk,
}

#[derive(Clone, PartialEq, Debug)]
pub struct LockdownStatus {
    pub(crate) status: LockdownStatusInternal,
    pub(crate) message: String,
}

impl std::fmt::Display for LockdownStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum LockdownStatusInternal {
    Enabled,
    Partial,
    Disabled,
}

impl LockdownStatus {
    /// Did enabling lockdown complete successfully?
    pub fn is_fully_locked(&self) -> bool {
        self.status == LockdownStatusInternal::Enabled
    }

    /// Did disabling lockdown complete successfuly, or new machine was never locked?
    pub fn is_fully_unlocked(&self) -> bool {
        self.status == LockdownStatusInternal::Disabled
    }

    /// Did lockdown enable/disable fail part way through, so we are partially locked?
    pub fn is_partially_locked(&self) -> bool {
        self.status == LockdownStatusInternal::Partial
    }

    /// A vendor specific message detailing which things are locked and which things are unlocked.
    /// Format of message will change, do not parse.
    pub fn message(&self) -> &str {
        &self.message
    }
}
