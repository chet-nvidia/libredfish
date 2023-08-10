use crate::{standard::RedfishStandard, Redfish, RedfishError};

pub struct Bmc {
    s: RedfishStandard,
}

impl Bmc {
    pub fn new(s: RedfishStandard) -> Result<Bmc, RedfishError> {
        Ok(Bmc { s })
    }
}

impl Redfish for Bmc {
    fn change_password(&self, user: &str, new: &str) -> Result<(), RedfishError> {
        self.s.change_password(user, new)
    }

    fn get_firmware(
        &self,
        id: &str,
    ) -> Result<crate::model::software_inventory::SoftwareInventory, RedfishError> {
        self.s.get_firmware(id)
    }

    fn get_software_inventories(
        &self,
    ) -> Result<crate::model::software_inventory::SoftwareInventoryCollection, RedfishError> {
        self.s.get_software_inventories()
    }

    fn get_task(&self, id: &str) -> Result<crate::model::task::Task, RedfishError> {
        self.s.get_task(id)
    }

    fn get_power_state(&self) -> Result<crate::PowerState, RedfishError> {
        self.s.get_power_state()
    }

    fn power(&self, action: crate::SystemPowerControl) -> Result<(), RedfishError> {
        self.s.power(action)
    }

    fn forge_setup(&self) -> Result<(), RedfishError> {
        self.s.forge_setup()
    }

    fn lockdown(&self, target: crate::EnabledDisabled) -> Result<(), RedfishError> {
        self.s.lockdown(target)
    }

    fn lockdown_status(&self) -> Result<crate::Status, RedfishError> {
        self.s.lockdown_status()
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        self.s.setup_serial_console()
    }

    fn serial_console_status(&self) -> Result<crate::Status, RedfishError> {
        self.s.serial_console_status()
    }

    fn get_boot_options(&self) -> Result<crate::BootOptions, RedfishError> {
        self.s.get_boot_options()
    }

    fn boot_once(&self, target: crate::Boot) -> Result<(), RedfishError> {
        self.s.boot_once(target)
    }

    fn boot_first(&self, target: crate::Boot) -> Result<(), RedfishError> {
        self.s.boot_first(target)
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        self.s.clear_tpm()
    }

    fn pcie_devices(&self) -> Result<Vec<crate::PCIeDevice>, RedfishError> {
        self.s.pcie_devices()
    }

    fn update_firmware(
        &self,
        firmware: std::fs::File,
    ) -> Result<crate::model::task::Task, RedfishError> {
        self.s.update_firmware(firmware)
    }

    fn bios(&self) -> Result<std::collections::HashMap<String, serde_json::Value>, RedfishError> {
        self.s.bios()
    }

    fn pending(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, RedfishError> {
        self.s.pending()
    }

    fn clear_pending(&self) -> Result<(), RedfishError> {
        self.s.clear_pending()
    }
}
