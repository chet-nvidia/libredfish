use std::collections::HashMap;

use tracing::debug;

use crate::model::{power, storage, thermal};
use crate::network::RedfishHttpClient;
use crate::RedfishError;
use crate::{model, Boot, EnabledDisabled, PowerState, Redfish, Status};

/// The calls that use the Redfish standard without any OEM extensions.
pub struct RedfishStandard {
    pub client: RedfishHttpClient,
    pub vendor: Option<String>,
    manager_id: String,
    system_id: String,
}

impl Redfish for RedfishStandard {
    fn get_power_state(&self) -> Result<PowerState, RedfishError> {
        let system = self.get_system()?;
        Ok(system.power_state)
    }

    fn power(&self, action: model::SystemPowerControl) -> Result<(), RedfishError> {
        let url = format!("Systems/{}/Actions/ComputerSystem.Reset", self.system_id);
        let mut arg = HashMap::new();
        arg.insert("ResetType", action.to_string());
        // Lenovo: The expected HTTP response code is 204 No Content
        self.client.post(&url, arg).map(|_status_code| Ok(()))?
    }

    fn bios(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        let url = format!("Systems/{}/Bios", self.system_id());
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        let url = format!("Systems/{}/Bios/Settings", self.system_id());
        self.pending_with_url(&url)
    }

    fn lockdown(&self, _target: EnabledDisabled) -> Result<(), RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn lockdown_status(&self) -> Result<Status, RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn serial_console_status(&self) -> Result<Status, RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn boot_once(&self, _target: Boot) -> Result<(), RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn boot_first(&self, _target: Boot) -> Result<(), RedfishError> {
        unimplemented!("No standard implementation");
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        unimplemented!("No standard implementation");
    }
}

impl RedfishStandard {
    //
    // PUBLIC
    //

    /// Create and setup a connection to BMC.
    /// Issues two HTTP calls to get intial data.
    pub fn new(client: RedfishHttpClient) -> Result<Self, RedfishError> {
        let mut r = Self {
            client,
            manager_id: "".to_string(),
            system_id: "".to_string(),
            vendor: None,
        };
        r.set_vendor()?;
        r.set_system_id()?;
        r.set_manager_id()?;
        Ok(r)
    }

    pub fn system_id(&self) -> &str {
        &self.system_id
    }

    pub fn manager_id(&self) -> &str {
        &self.manager_id
    }

    pub fn get_boot_options(&self) -> Result<model::BootOptions, RedfishError> {
        let url = format!("Systems/{}/BootOptions", self.system_id());
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    // The URL differs for Lenovo, but the rest is the same
    pub fn pending_with_url(
        &self,
        pending_url: &str,
    ) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        let (_sc, body): (reqwest::StatusCode, HashMap<String, serde_json::Value>) =
            self.client.get(pending_url)?;
        let pending_attrs = body.get("Attributes").unwrap().as_object().unwrap();

        let current = self.bios()?;
        let current_attrs = current.get("Attributes").unwrap();

        let diff = pending_attrs
            .iter()
            .filter(|(k, v)| current_attrs.get(k) != Some(v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(diff)
    }

    //
    // PRIVATE
    //

    /// Fetch root URL and record the vendor, if any
    fn set_vendor(&mut self) -> Result<(), RedfishError> {
        let (_, out): (_, HashMap<String, serde_json::Value>) = self.client.get("")?;
        self.vendor = match out.get("Vendor") {
            Some(v) => v.as_str().map(|s| s.to_string()),
            None => None,
        };
        debug!(
            "BMC Vendor: {}",
            self.vendor.as_deref().unwrap_or("Unknown")
        );
        Ok(())
    }

    /// Fetch and set System number. Needed for all `Systems/{system_id}/...` calls
    fn set_system_id(&mut self) -> Result<(), RedfishError> {
        let (_, systems): (_, model::Systems) = self.client.get("Systems/")?;
        if systems.members.is_empty() {
            self.system_id = "1".to_string(); // default to DMTF standard suggested
            return Ok(());
        }
        let v: Vec<&str> = systems.members[0].odata_id.split('/').collect();
        self.system_id = v.last().unwrap().to_string();
        Ok(())
    }

    /// Fetch and set Manager number. Needed for all `Managers/{system_id}/...` calls
    fn set_manager_id(&mut self) -> Result<(), RedfishError> {
        let (_, bmcs): (_, model::Managers) = self.client.get("Managers/")?;
        if bmcs.members.is_empty() {
            self.manager_id = "1".to_string(); // default to dmtf standard suggested
            return Ok(());
        }
        let v: Vec<&str> = bmcs.members[0].odata_id.split('/').collect();
        self.manager_id = v.last().unwrap().to_string();
        Ok(())
    }

    fn get_system(&self) -> Result<model::ComputerSystem, RedfishError> {
        let url = format!("Systems/{}/", self.system_id);
        let host: model::ComputerSystem = self.client.get(&url)?.1;
        Ok(host)
    }

    //
    // NOT CURRENTLY USED
    //

    #[allow(dead_code)]
    pub fn get_manager(&self) -> Result<model::Manager, RedfishError> {
        let (_, manager): (_, model::Manager) = self
            .client
            .get(&format!("Managers/{}", self.manager_id()))?;
        Ok(manager)
    }

    #[allow(dead_code)]
    pub fn get_array_controller(
        &self,
        controller_id: u64,
    ) -> Result<storage::ArrayController, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/",
            self.system_id(),
            controller_id
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_array_controllers(&self) -> Result<storage::ArrayControllers, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/",
            self.system_id()
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    /// Query the power status from the server
    #[allow(dead_code)]
    pub fn get_power_status(&self) -> Result<power::Power, RedfishError> {
        let url = format!("Chassis/{}/Power/", self.system_id());
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    /// Query the thermal status from the server
    #[allow(dead_code)]
    pub fn get_thermal_status(&self) -> Result<thermal::Thermal, RedfishError> {
        let url = format!("Chassis/{}/Thermal/", self.system_id());
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    /// Query the smart array status from the server
    #[allow(dead_code)]
    pub fn get_smart_array_status(
        &self,
        controller_id: u64,
    ) -> Result<storage::SmartArray, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/",
            self.system_id(),
            controller_id
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_logical_drives(
        &self,
        controller_id: u64,
    ) -> Result<storage::LogicalDrives, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/LogicalDrives/",
            self.system_id(),
            controller_id
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_physical_drive(
        &self,
        drive_id: u64,
        controller_id: u64,
    ) -> Result<storage::DiskDrive, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/DiskDrives/{}/",
            self.system_id(),
            controller_id,
            drive_id,
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_physical_drives(
        &self,
        controller_id: u64,
    ) -> Result<storage::DiskDrives, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/DiskDrives/",
            self.system_id(),
            controller_id
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_storage_enclosures(
        &self,
        controller_id: u64,
    ) -> Result<storage::StorageEnclosures, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/StorageEnclosures/",
            self.system_id(),
            controller_id
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }

    #[allow(dead_code)]
    pub fn get_storage_enclosure(
        &self,
        controller_id: u64,
        enclosure_id: u64,
    ) -> Result<storage::StorageEnclosure, RedfishError> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/StorageEnclosures/{}/",
            self.system_id(),
            controller_id,
            enclosure_id,
        );
        let (_status_code, body) = self.client.get(&url)?;
        Ok(body)
    }
}
