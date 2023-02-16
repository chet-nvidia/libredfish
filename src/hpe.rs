use std::collections::HashMap;

use crate::{
    network::NetworkConfig, standard::RedfishStandard, Boot, EnabledDisabled, PowerState, Redfish,
    RedfishError, SystemPowerControl,
};

pub struct Bmc {
    s: RedfishStandard,
}

impl Bmc {
    pub fn new(config: NetworkConfig) -> Result<Bmc, RedfishError> {
        Ok(Bmc {
            s: RedfishStandard::new(config)?,
        })
    }
}

impl Redfish for Bmc {
    fn get_power_state(&self) -> Result<PowerState, RedfishError> {
        self.s.get_power_state()
    }

    fn power(&self, action: SystemPowerControl) -> Result<(), RedfishError> {
        self.s.power(action)
    }

    fn bios_attributes(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        self.s.bios_attributes()
    }

    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        unimplemented!("not implement for Hpe yet");
        // copying the lenovo.rs version might work, it _looks_ standard even though Dell doesn't
        // have it
    }

    fn lockdown(&self, _target: EnabledDisabled) -> Result<(), RedfishError> {
        unimplemented!("not implement for Hpe yet");
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        unimplemented!("not implement for Hpe yet");
    }

    fn boot_once(&self, _target: Boot) -> Result<(), RedfishError> {
        unimplemented!("not implement for Hpe yet");
    }

    fn boot_first(&self, _target: Boot) -> Result<(), RedfishError> {
        unimplemented!("not implement for Hpe yet");
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        unimplemented!("not implement for Hpe yet");
    }
}
