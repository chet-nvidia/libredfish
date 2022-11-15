#[macro_use]
extern crate serde_derive;

pub mod bios;
pub mod common;
pub mod manager;
pub mod power;
pub mod storage;
pub mod thermal;
pub mod system;

use std::collections::HashMap;
use reqwest::{header::HeaderValue, header::ACCEPT, header::CONTENT_TYPE, blocking::Client};
use serde::de::DeserializeOwned;
use serde_json::json;

const REDFISH_ENDPOINT: &str = "redfish/v1/";

pub struct Config {
    pub user: Option<String>,
    pub endpoint: String,
    pub password: Option<String>,
    pub port: Option<u16>,
    pub system: String,
}

pub struct Redfish {
    pub client: Client,
    pub config: Config,
}

impl Redfish {

    pub fn new(client: Client, config: Config) -> Self {
        Redfish { client, config }
    }

    pub fn get<T>(&self, api: &str) -> Result<T, reqwest::Error>
    where
        T: DeserializeOwned + ::std::fmt::Debug,
    {
        let url = match self.config.port {
            Some(p) => format!("https://{}:{}/{}/{}", self.config.endpoint, p, REDFISH_ENDPOINT, api),
            None => format!("https://{}/{}/{}", self.config.endpoint, REDFISH_ENDPOINT, api),
        };

        let res: T = match &self.config.user {
            Some(user) => self
                .client
                .get(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .basic_auth(&user, self.config.password.as_ref())
                .send()?
                .error_for_status()?
                .json()?,
            None => self
                .client
                .get(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .send()?
                .error_for_status()?
                .json()?,
        };
        Ok(res)
    }

    pub fn post(&self, api: &str, data: HashMap<&str, String>) -> Result<(), reqwest::Error>
    {
        let url = match self.config.port {
            Some(p) => format!("https://{}:{}/{}/{}", self.config.endpoint, p, REDFISH_ENDPOINT, api),
            None => format!("https://{}/{}/{}", self.config.endpoint, REDFISH_ENDPOINT, api),
        };

        match &self.config.user {
            Some(user) => self
                .client
                .post(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .basic_auth(&user, self.config.password.as_ref())
                .json(&data)
                .send()?
                .error_for_status()?,
            None => self
                .client
                .post(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .json(&data)
                .send()?
                .error_for_status()?,
        };
        Ok(())
    }

    pub fn patch(&self, api: &str, data: serde_json::Value) -> Result<(), reqwest::Error>
    {
        let url = match self.config.port {
            Some(p) => format!("https://{}:{}/{}/{}", self.config.endpoint, p, REDFISH_ENDPOINT, api),
            None => format!("https://{}/{}/{}", self.config.endpoint, REDFISH_ENDPOINT, api),
        };

        match &self.config.user {
            Some(user) => self
                .client
                .patch(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .basic_auth(&user, self.config.password.as_ref())
                .json(&data)
                .send()?
                .error_for_status()?,
            None => self
                .client
                .patch(&url)
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .json(&data)
                .send()?
                .error_for_status()?,
        };
        Ok(())
    }

    pub fn get_system_id(&mut self) -> Result<String, String> {
        let url = "Systems/";
        match self.get(url) {
            Ok(x) => {
                let systems: system::Systems = x;
                if systems.members.is_empty() {
                    return Err(String::from("Invalid response"));
                }
                self.config.system = systems.members[0].odata_id.clone();
                Ok(systems.members[0].odata_id.clone())
            }
            Err(e) => {
                Err(e.to_string())
            }
        }
    }

    pub fn get_system(&self) -> Result<system::ComputerSystem, reqwest::Error> {
        let url = format!("Systems/{}/", self.config.system);
        let host: system::ComputerSystem = self.get(&url)?;
        Ok(host)
    }

    pub fn set_system_power(&self, action: system::SystemPowerControl) -> Result<(), reqwest::Error> {
        let url = format!("Systems/{}/Actions/ComputerSystem.Reset", self.config.system);
        let mut arg = HashMap::new();
        arg.insert("ResetType", action.to_string());
        self.post(&url, arg)
    }

    pub fn get_bios_data(&self) -> Result<bios::OemDellBios, reqwest::Error> {
        let url = format!("Systems/{}/Bios", self.config.system);
        let bios: bios::OemDellBios = self.get(&url)?;
        Ok(bios)
    }

    pub fn set_bios_attribute(&self, attribute: String, value: String) -> Result<(), reqwest::Error> {
        let url = format!("Systems/{}/Bios/Settings/", self.config.system);
        let attr = json!({
            "@Redfish.SettingsApplyTime": {
                "ApplyTime": "OnReset"
            },
            "Attributes": {
                attribute: value
            }
        });
        self.patch(&url, attr)
    }

    pub fn enable_bios_lockdown(&self) -> Result<(), reqwest::Error> {
        self.set_bios_attribute("InBandManageabilityInterface".to_string(), "Disabled".to_string())?;
        self.set_bios_attribute("UefiVariableAccess".to_string(), "Controlled".to_string())
    }

    pub fn get_array_controller(
        &self,
        controller_id: u64,
    ) -> Result<storage::ArrayController, reqwest::Error> {
        let url = format!("Systems/{}/SmartStorage/ArrayControllers/{}/", self.config.system, controller_id);
        let s: storage::ArrayController = self.get(&url)?;
        Ok(s)
    }
    pub fn get_array_controllers(&self) -> Result<storage::ArrayControllers, reqwest::Error> {
        let url = format!("Systems/{}/SmartStorage/ArrayControllers/", self.config.system);
        let s: storage::ArrayControllers = self.get(&url)?;
        Ok(s)
    }

    /// Query the manager status from the server
    pub fn get_manager_status(&self) -> Result<manager::Manager, reqwest::Error> {
        let url = "Managers/";
        let m: manager::Manager = self.get(url)?;
        Ok(m)
    }

    /// Query the power status from the server
    pub fn get_power_status(&self) -> Result<power::Power, reqwest::Error> {
        let url = format!("Chassis/{}/Power/", self.config.system);
        let p: power::Power = self.get(&url)?;
        Ok(p)
    }

    /// Query the thermal status from the server
    pub fn get_thermal_status(&self) -> Result<thermal::Thermal, reqwest::Error> {
        let url = format!("Chassis/{}/Thermal/", self.config.system);
        let t: thermal::Thermal = self.get(&url)?;
        Ok(t)
    }

    /// Query the smart array status from the server
    pub fn get_smart_array_status(
        &self,
        controller_id: u64,
    ) -> Result<storage::SmartArray, reqwest::Error> {
        let url = format!("Systems/{}/SmartStorage/ArrayControllers/{}/", self.config.system, controller_id);
        let s: storage::SmartArray = self.get(&url)?;
        Ok(s)
    }

    pub fn get_logical_drives(
        &self,
        controller_id: u64,
    ) -> Result<storage::LogicalDrives, reqwest::Error> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/LogicalDrives/",
            self.config.system,
            controller_id
        );
        let s: storage::LogicalDrives = self.get(&url)?;
        Ok(s)
    }

    pub fn get_physical_drive(
        &self,
        drive_id: u64,
        controller_id: u64,
    ) -> Result<storage::DiskDrive, reqwest::Error> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/DiskDrives/{}/",
            self.config.system,
            controller_id, drive_id,
        );
        let d: storage::DiskDrive = self.get(&url)?;
        Ok(d)
    }

    pub fn get_physical_drives(
        &self,
        controller_id: u64,
    ) -> Result<storage::DiskDrives, reqwest::Error> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/DiskDrives/",
            self.config.system,
            controller_id
        );
        let d: storage::DiskDrives = self.get(&url)?;
        Ok(d)
    }

    pub fn get_storage_enclosures(
        &self,
        controller_id: u64,
    ) -> Result<storage::StorageEnclosures, reqwest::Error> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/StorageEnclosures/",
            self.config.system,
            controller_id
        );
        let s: storage::StorageEnclosures = self.get(&url)?;
        Ok(s)
    }
    pub fn get_storage_enclosure(
        &self,
        controller_id: u64,
        enclosure_id: u64,
    ) -> Result<storage::StorageEnclosure, reqwest::Error> {
        let url = format!(
            "Systems/{}/SmartStorage/ArrayControllers/{}/StorageEnclosures/{}/",
            self.config.system,
            controller_id, enclosure_id,
        );
        let s: storage::StorageEnclosure = self.get(&url)?;
        Ok(s)
    }
}
