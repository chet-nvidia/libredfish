use std::{collections::HashMap, time::Duration};

use reqwest::Method;
use tracing::debug;

use crate::{
    model::{oem::lenovo, BootOption},
    network::{NetworkConfig, REDFISH_ENDPOINT},
    standard::RedfishStandard,
    Boot, EnabledDisabled, PowerState, Redfish, RedfishError, SystemPowerControl,
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

    fn lockdown(&self, target: EnabledDisabled) -> Result<(), RedfishError> {
        use EnabledDisabled::*;
        match target {
            Enabled => self.enable_lockdown(),
            Disabled => self.disable_lockdown(),
        }
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        let mut body = HashMap::new();
        body.insert(
            "Attributes",
            HashMap::from([
                (
                    "DevicesandIOPorts_COMPort1",
                    EnabledDisabled::Enabled.to_string(),
                ),
                (
                    "DevicesandIOPorts_ConsoleRedirection",
                    EnabledDisabled::Enabled.to_string(),
                ),
                (
                    "DevicesandIOPorts_SPRedirection",
                    EnabledDisabled::Enabled.to_string(),
                ),
                (
                    "DevicesandIOPorts_SerialPortSharing",
                    EnabledDisabled::Enabled.to_string(),
                ),
                (
                    "DevicesandIOPorts_COMPortActiveAfterBoot",
                    EnabledDisabled::Enabled.to_string(),
                ),
                (
                    "DevicesandIOPorts_SerialPortAccessMode",
                    "Shared".to_string(),
                ),
            ]),
        );
        let url = format!("Systems/{}/Bios/Pending", self.s.system_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn boot_once(&self, target: Boot) -> Result<(), RedfishError> {
        match target {
            Boot::Pxe => self.set_boot_override(lenovo::BootSource::Pxe),
            Boot::HardDisk => self.set_boot_override(lenovo::BootSource::Hdd),
        }
    }

    fn boot_first(&self, target: Boot) -> Result<(), RedfishError> {
        match target {
            Boot::Pxe => self.set_boot_first(lenovo::BootOptionName::Network),
            Boot::HardDisk => self.set_boot_first(lenovo::BootOptionName::HardDisk),
        }
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        let mut body = HashMap::new();
        body.insert(
            "Attributes",
            HashMap::from([("TrustedComputingGroup_DeviceOperation", "Clear")]),
        );
        let url = format!("Systems/{}/Bios/Pending", self.s.system_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        let url = format!("Systems/{}/Bios/Pending", self.s.system_id());
        self.s.pending(&url)
    }
}

impl Bmc {
    /// Lock a Lenovo server to make it ready for tenants
    fn enable_lockdown(&self) -> Result<(), RedfishError> {
        self.set_kcs_lenovo(false).map_err(|e| {
            debug!("Failed disabling 'IPMI over KCS Access'");
            e
        })?;
        self.set_firmware_rollback_lenovo(EnabledDisabled::Disabled)
            .map_err(|e| {
                debug!("Failed changing 'Prevent System Firmware Down-Level'");
                e
            })?;
        self.set_ethernet_over_usb(false).map_err(|e| {
            debug!("Failed disabling Ethernet over USB");
            e
        })?;
        self.set_front_panel_usb_lenovo(lenovo::FrontPanelUSBMode::Server)
            .map_err(|e| {
                debug!("Failed locking front panel USB to host-only.");
                e
            })?;
        Ok(())
    }

    /// Unlock a Lenovo server, restoring defaults
    pub fn disable_lockdown(&self) -> Result<(), RedfishError> {
        self.set_kcs_lenovo(true).map_err(|e| {
            debug!("Failed enabling 'IPMI over KCS Access'");
            e
        })?;
        self.set_firmware_rollback_lenovo(EnabledDisabled::Enabled)
            .map_err(|e| {
                debug!("Failed changing 'Prevent System Firmware Down-Level'");
                e
            })?;
        self.set_ethernet_over_usb(true).map_err(|e| {
            debug!("Failed disabling Ethernet over USB");
            e
        })?;
        self.set_front_panel_usb_lenovo(lenovo::FrontPanelUSBMode::Shared)
            .map_err(|e| {
                debug!("Failed unlocking front panel USB to shared mode.");
                e
            })?;
        Ok(())
    }

    fn set_kcs_lenovo(&self, is_allowed: bool) -> Result<(), RedfishError> {
        let body = HashMap::from([(
            "Oem",
            HashMap::from([("Lenovo", HashMap::from([("KCSEnabled", is_allowed)]))]),
        )]);
        let url = format!("Managers/{}", self.s.manager_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn set_firmware_rollback_lenovo(&self, set: EnabledDisabled) -> Result<(), RedfishError> {
        let body = HashMap::from([(
            "Configurator",
            HashMap::from([("FWRollback", set.to_string())]),
        )]);
        let url = format!("Managers/{}/Oem/Lenovo/Security", self.s.manager_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn set_front_panel_usb_lenovo(
        &self,
        mode: lenovo::FrontPanelUSBMode,
    ) -> Result<(), RedfishError> {
        let mut body = HashMap::new();
        body.insert(
            "Oem",
            HashMap::from([(
                "Lenovo",
                HashMap::from([(
                    "FrontPanelUSB",
                    HashMap::from([("FPMode", mode.to_string())]),
                )]),
            )]),
        );
        let url = format!("Systems/{}", self.s.system_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn set_ethernet_over_usb(&self, is_allowed: bool) -> Result<(), RedfishError> {
        let body = HashMap::from([("InterfaceEnabled", is_allowed)]);
        let url = format!("Managers/{}/EthernetInterfaces/ToHost", self.s.manager_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    fn set_boot_override(&self, target: lenovo::BootSource) -> Result<(), RedfishError> {
        let target_str = &target.to_string();
        let body = HashMap::from([(
            "Boot",
            HashMap::from([
                ("BootSourceOverrideEnabled", "Once"),
                ("BootSourceOverrideTarget", target_str),
            ]),
        )]);
        let url = format!("Systems/{}", self.s.system_id());
        self.s.net.patch(&url, body).map(|_status_code| ())
    }

    // name: The name of the device you want to make the first boot choice.
    //
    // Note that _within_ the type you choose you could also give the order. e.g for "Network"
    // see Systems/1/Oem/Lenovo/BootSettings/BootOrder.NetworkBootOrder
    // and for "HardDisk" see Systems/1/Oem/Lenovo/BootSettings/BootOrder.HardDiskBootOrder
    fn set_boot_first(&self, name: lenovo::BootOptionName) -> Result<(), RedfishError> {
        let boot_array = match self.get_boot_options_ids_with_first(name)? {
            None => {
                return Err(RedfishError::MissingBootOption(name.to_string()));
            }
            Some(b) => b,
        };

        let body = HashMap::from([("Boot", HashMap::from([("BootOrder", boot_array)]))]);
        let url = format!("Systems/{}/Pending", self.s.system_id());
        // BMC takes longer to respond to this one, so override timeout
        let timeout = Duration::from_secs(10);
        let (_status_code, _resp_body): (_, Option<HashMap<String, serde_json::Value>>) = self
            .s
            .net
            .req(Method::PATCH, &url, Some(body), Some(timeout))?;
        Ok(())
    }

    // A Vec of string boot option names, with the one you want first.
    //
    // Example: get_boot_options_ids_with_first(lenovo::BootOptionName::Network) might return
    // ["Boot0003", "Boot0002", "Boot0001", "Boot0004"] where Boot0003 is Network. It has been
    // moved to the front ready for sending as an update.
    // The order of the other boot options does not change.
    //
    // If the boot option you want is not found returns Ok(None)
    fn get_boot_options_ids_with_first(
        &self,
        with_name: lenovo::BootOptionName,
    ) -> Result<Option<Vec<String>>, RedfishError> {
        let with_name_str = with_name.to_string();
        let mut with_name_match = None; // the ID of the option matching with_name
        let mut ordered = Vec::new(); // the final boot options
        let boot_options = self.s.get_boot_options()?;
        for member in boot_options.members {
            let url = member
                .odata_id
                .replace(&format!("/{REDFISH_ENDPOINT}/"), "");
            let b: BootOption = self.s.net.get(&url)?.1;
            if b.name == with_name_str {
                with_name_match = Some(b.id);
            } else {
                ordered.push(b.id);
            }
        }
        match with_name_match {
            None => Ok(None),
            Some(with_name_id) => {
                ordered.insert(0, with_name_id);
                Ok(Some(ordered))
            }
        }
    }
}
