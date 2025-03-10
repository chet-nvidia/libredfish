use std::str::FromStr;
/*
 * SPDX-FileCopyrightText: Copyright (c) 2025 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: MIT
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */
use std::{collections::HashMap, path::Path, time::Duration};

use reqwest::StatusCode;
use serde::Deserialize;
use tokio::fs::File;

use crate::model::account_service::ManagerAccount;
use crate::model::oem::nvidia_dpu::NicMode;
use crate::model::sensor::GPUSensors;
use crate::model::service_root::RedfishVendor;
use crate::model::task::Task;
use crate::model::update_service::{ComponentType, TransferProtocolType, UpdateService};
use crate::Boot::UefiHttp;
use crate::HostPrivilegeLevel::Restricted;
use crate::InternalCPUModel::Embedded;
use crate::{
    model::{
        boot::{BootSourceOverrideEnabled, BootSourceOverrideTarget},
        chassis::NetworkAdapter,
        oem::nvidia_dpu::{HostPrivilegeLevel, InternalCPUModel},
        sel::{LogEntry, LogEntryCollection},
        service_root::ServiceRoot,
        storage::Drives,
        BootOption, ComputerSystem, Manager,
    },
    standard::RedfishStandard,
    BiosProfileType, Collection, NetworkDeviceFunction, ODataId, Redfish, RedfishError, Resource,
};
use crate::{JobState, MachineSetupDiff, MachineSetupStatus, RoleId};

pub struct Bmc {
    s: RedfishStandard,
}

pub enum BootOptionName {
    Http,
    Pxe,
    Disk,
}
impl BootOptionName {
    fn to_string(&self) -> &str {
        match self {
            BootOptionName::Http => "UEFI HTTPv4",
            BootOptionName::Pxe => "UEFI PXEv4",
            BootOptionName::Disk => "UEFI Non-Block Boot Device",
        }
    }
}

impl Bmc {
    pub fn new(s: RedfishStandard) -> Result<Bmc, RedfishError> {
        Ok(Bmc { s })
    }
}

#[async_trait::async_trait]
impl Redfish for Bmc {
    async fn create_user(
        &self,
        username: &str,
        password: &str,
        role_id: RoleId,
    ) -> Result<(), RedfishError> {
        self.s.create_user(username, password, role_id).await
    }

    async fn change_username(&self, old_name: &str, new_name: &str) -> Result<(), RedfishError> {
        self.s.change_username(old_name, new_name).await
    }

    async fn change_password(&self, user: &str, new: &str) -> Result<(), RedfishError> {
        self.s.change_password(user, new).await
    }

    /// Note that DPU account_ids are not numbers but usernames: "root", "admin", etc
    async fn change_password_by_id(
        &self,
        account_id: &str,
        new_pass: &str,
    ) -> Result<(), RedfishError> {
        self.s.change_password_by_id(account_id, new_pass).await
    }

    async fn get_accounts(&self) -> Result<Vec<ManagerAccount>, RedfishError> {
        self.s.get_accounts().await
    }

    async fn get_firmware(
        &self,
        id: &str,
    ) -> Result<crate::model::software_inventory::SoftwareInventory, RedfishError> {
        self.s.get_firmware(id).await
    }

    async fn get_software_inventories(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_software_inventories().await
    }

    async fn get_tasks(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_tasks().await
    }

    async fn get_task(&self, id: &str) -> Result<crate::model::task::Task, RedfishError> {
        self.s.get_task(id).await
    }

    async fn get_power_state(&self) -> Result<crate::PowerState, RedfishError> {
        self.s.get_power_state().await
    }

    async fn get_power_metrics(&self) -> Result<crate::Power, RedfishError> {
        let (_status_code, body) = self.s.client.get("Chassis/Card1/Power/").await?;
        Ok(body)
    }

    async fn power(&self, action: crate::SystemPowerControl) -> Result<(), RedfishError> {
        self.s.power(action).await
    }

    async fn bmc_reset(&self) -> Result<(), RedfishError> {
        self.s.bmc_reset().await
    }

    async fn chassis_reset(
        &self,
        chassis_id: &str,
        reset_type: crate::SystemPowerControl,
    ) -> Result<(), RedfishError> {
        self.s.chassis_reset(chassis_id, reset_type).await
    }

    async fn get_thermal_metrics(&self) -> Result<crate::Thermal, RedfishError> {
        let (_status_code, body) = self.s.client.get("Chassis/Card1/Thermal/").await?;
        Ok(body)
    }

    async fn get_gpu_sensors(&self) -> Result<Vec<GPUSensors>, RedfishError> {
        self.s.get_gpu_sensors().await
    }

    async fn get_system_event_log(&self) -> Result<Vec<LogEntry>, RedfishError> {
        self.get_system_event_log().await
    }

    async fn get_drives_metrics(&self) -> Result<Vec<Drives>, RedfishError> {
        self.s.get_drives_metrics().await
    }

    async fn machine_setup(
        &self,
        _boot_interface_mac: Option<&str>,
        _bios_profiles: &HashMap<
            RedfishVendor,
            HashMap<String, HashMap<BiosProfileType, HashMap<String, serde_json::Value>>>,
        >,
        _selected_profile: BiosProfileType,
    ) -> Result<(), RedfishError> {
        self.disable_secure_boot().await?;
        self.set_host_privilege_level(Restricted).await?;
        // we have found that only newer BMC fws support this action.
        // Until we re-enable DPU BMC firmware updates in preingestion,
        // ignore an error from trying to disable host rshim against
        // BF3s that have a BMC that is too old.
        self.set_host_rshim(false, true).await?;
        self.set_internal_cpu_model(Embedded).await?;
        self.boot_once(UefiHttp).await
    }

    async fn machine_setup_status(&self) -> Result<MachineSetupStatus, RedfishError> {
        let mut diffs = vec![];

        let sb = self.get_secure_boot().await?;
        if sb.secure_boot_enable.unwrap_or(false) {
            diffs.push(MachineSetupDiff {
                key: "SecureBoot".to_string(),
                expected: "false".to_string(),
                actual: "true".to_string(),
            });
        }

        let bios = self.s.bios_attributes().await?;
        let key = "HostPrivilegeLevel";
        let key_with_spaces = "Host Privilege Level";
        let Some(hpl) = bios.get(key).or_else(|| bios.get(key_with_spaces)) else {
            return Err(RedfishError::MissingKey {
                key: key.to_string(),
                url: "Systems/{}/Bios".to_string(),
            });
        };

        let actual = HostPrivilegeLevel::deserialize(hpl).map_err(|e| {
            RedfishError::JsonDeserializeError {
                url: "Systems/{}/Bios".to_string(),
                body: hpl.to_string(),
                source: e,
            }
        })?;
        let expected = HostPrivilegeLevel::Restricted;
        if actual != expected {
            diffs.push(MachineSetupDiff {
                key: key.to_string(),
                actual: actual.to_string(),
                expected: expected.to_string(),
            });
        }

        let key = "InternalCPUModel";
        let key_with_spaces = "Internal CPU Model";
        let Some(icm) = bios.get(key).or_else(|| bios.get(key_with_spaces)) else {
            return Err(RedfishError::MissingKey {
                key: key.to_string(),
                url: "Systems/{}/Bios".to_string(),
            });
        };

        let actual =
            InternalCPUModel::deserialize(icm).map_err(|e| RedfishError::JsonDeserializeError {
                url: "Systems/{}/Bios".to_string(),
                body: hpl.to_string(),
                source: e,
            })?;
        let expected = InternalCPUModel::Embedded;
        if actual != expected {
            diffs.push(MachineSetupDiff {
                key: key.to_string(),
                actual: actual.to_string(),
                expected: expected.to_string(),
            });
        }

        Ok(MachineSetupStatus {
            is_done: diffs.is_empty(),
            diffs,
        })
    }

    async fn set_machine_password_policy(&self) -> Result<(), RedfishError> {
        /*
        We used to try to PATCH AccountLockoutThreshold and AccountLockoutDuration
        But, I tried this against multiple DPUs, both BF2 and BF3. When I issued the same
        request, the DPU's BMC returns an error indicating that these properties are read only.
        */
        Ok(())
    }

    async fn lockdown(&self, target: crate::EnabledDisabled) -> Result<(), RedfishError> {
        self.s.lockdown(target).await
    }

    async fn lockdown_status(&self) -> Result<crate::Status, RedfishError> {
        self.s.lockdown_status().await
    }

    async fn setup_serial_console(&self) -> Result<(), RedfishError> {
        self.s.setup_serial_console().await
    }

    async fn serial_console_status(&self) -> Result<crate::Status, RedfishError> {
        self.s.serial_console_status().await
    }

    async fn get_boot_options(&self) -> Result<crate::BootOptions, RedfishError> {
        self.s.get_boot_options().await
    }

    async fn get_boot_option(&self, option_id: &str) -> Result<BootOption, RedfishError> {
        self.s.get_boot_option(option_id).await
    }

    async fn boot_once(&self, target: crate::Boot) -> Result<(), RedfishError> {
        match target {
            crate::Boot::Pxe => {
                self.set_boot_override(
                    BootSourceOverrideTarget::Pxe,
                    BootSourceOverrideEnabled::Once,
                )
                .await
            }
            crate::Boot::HardDisk => {
                self.set_boot_override(
                    BootSourceOverrideTarget::Hdd,
                    BootSourceOverrideEnabled::Once,
                )
                .await
            }
            crate::Boot::UefiHttp => {
                self.set_boot_override(
                    BootSourceOverrideTarget::UefiHttp,
                    BootSourceOverrideEnabled::Once,
                )
                .await
            }
        }
    }

    async fn boot_first(&self, target: crate::Boot) -> Result<(), RedfishError> {
        match target {
            crate::Boot::Pxe => self.set_boot_order(&BootOptionName::Pxe).await,
            crate::Boot::HardDisk => self.set_boot_order(&BootOptionName::Disk).await,
            crate::Boot::UefiHttp => self.set_boot_order(&BootOptionName::Http).await,
        }
    }

    async fn clear_tpm(&self) -> Result<(), RedfishError> {
        self.s.clear_tpm().await
    }

    async fn pcie_devices(&self) -> Result<Vec<crate::PCIeDevice>, RedfishError> {
        self.s.pcie_devices().await
    }

    async fn update_firmware(
        &self,
        firmware: tokio::fs::File,
    ) -> Result<crate::model::task::Task, RedfishError> {
        self.s.update_firmware(firmware).await
    }

    async fn get_update_service(&self) -> Result<UpdateService, RedfishError> {
        self.s.get_update_service().await
    }

    async fn update_firmware_multipart(
        &self,
        filename: &Path,
        _reboot: bool,
        timeout: Duration,
        _component_type: ComponentType,
    ) -> Result<String, RedfishError> {
        let firmware = File::open(&filename)
            .await
            .map_err(|e| RedfishError::FileError(format!("Could not open file: {}", e)))?;

        let update_service = self.s.get_update_service().await?;

        if update_service.multipart_http_push_uri.is_empty() {
            return Err(RedfishError::NotSupported(
                "Host BMC does not support HTTP multipart push".to_string(),
            ));
        }

        let parameters = "{}".to_string();

        let (_status_code, _loc, body) = self
            .s
            .client
            .req_update_firmware_multipart(
                filename,
                firmware,
                parameters,
                &update_service.multipart_http_push_uri,
                true,
                timeout,
            )
            .await
            .map_err(|e| match e {
                RedfishError::HTTPErrorCode { status_code, .. }
                    if status_code == StatusCode::NOT_FOUND =>
                {
                    RedfishError::NotSupported(
                        "Host BMC does not support HTTP multipart push".to_string(),
                    )
                }
                e => e,
            })?;

        let task: Task =
            serde_json::from_str(&body).map_err(|e| RedfishError::JsonDeserializeError {
                url: update_service.multipart_http_push_uri,
                body,
                source: e,
            })?;

        Ok(task.id)
    }

    async fn bios(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, RedfishError> {
        self.s.bios().await
    }

    async fn set_bios(
        &self,
        values: HashMap<String, serde_json::Value>,
    ) -> Result<(), RedfishError> {
        self.s.set_bios(values).await
    }

    async fn pending(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, RedfishError> {
        self.s.pending().await
    }

    async fn clear_pending(&self) -> Result<(), RedfishError> {
        self.s.clear_pending().await
    }

    async fn get_system(&self) -> Result<ComputerSystem, RedfishError> {
        self.s.get_system().await
    }

    async fn get_secure_boot(&self) -> Result<crate::model::secure_boot::SecureBoot, RedfishError> {
        self.s.get_secure_boot().await
    }

    async fn enable_secure_boot(&self) -> Result<(), RedfishError> {
        self.s.enable_secure_boot().await
    }

    async fn disable_secure_boot(&self) -> Result<(), RedfishError> {
        self.s.disable_secure_boot().await
    }

    async fn add_secure_boot_certificate(&self, pem_cert: &str) -> Result<Task, RedfishError> {
        self.s.add_secure_boot_certificate(pem_cert).await
    }

    async fn get_chassis_all(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_chassis_all().await
    }

    async fn get_chassis(&self, id: &str) -> Result<crate::Chassis, RedfishError> {
        self.s.get_chassis(id).await
    }

    async fn get_chassis_network_adapters(
        &self,
        chassis_id: &str,
    ) -> Result<Vec<String>, RedfishError> {
        self.s.get_chassis_network_adapters(chassis_id).await
    }

    async fn get_chassis_network_adapter(
        &self,
        chassis_id: &str,
        id: &str,
    ) -> Result<NetworkAdapter, RedfishError> {
        self.s.get_chassis_network_adapter(chassis_id, id).await
    }

    async fn get_base_network_adapters(
        &self,
        system_id: &str,
    ) -> Result<Vec<String>, RedfishError> {
        self.s.get_base_network_adapters(system_id).await
    }

    async fn get_base_network_adapter(
        &self,
        system_id: &str,
        id: &str,
    ) -> Result<NetworkAdapter, RedfishError> {
        self.s.get_base_network_adapter(system_id, id).await
    }

    async fn get_manager_ethernet_interfaces(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_manager_ethernet_interfaces().await
    }

    async fn get_manager_ethernet_interface(
        &self,
        id: &str,
    ) -> Result<crate::EthernetInterface, RedfishError> {
        self.s.get_manager_ethernet_interface(id).await
    }

    async fn get_system_ethernet_interfaces(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_system_ethernet_interfaces().await
    }

    async fn get_system_ethernet_interface(
        &self,
        id: &str,
    ) -> Result<crate::EthernetInterface, RedfishError> {
        self.s.get_system_ethernet_interface(id).await
    }

    async fn get_ports(
        &self,
        chassis_id: &str,
        network_adapter: &str,
    ) -> Result<Vec<String>, RedfishError> {
        // http://redfish.dmtf.org/schemas/v1/NetworkPortCollection.json
        let url = format!(
            "Chassis/{}/NetworkAdapters/{}/Ports",
            chassis_id, network_adapter
        );
        self.s.get_members(&url).await
    }

    async fn get_port(
        &self,
        chassis_id: &str,
        network_adapter: &str,
        id: &str,
    ) -> Result<crate::NetworkPort, RedfishError> {
        let url = format!(
            "Chassis/{}/NetworkAdapters/{}/Ports/{}",
            chassis_id, network_adapter, id
        );
        let (_status_code, body) = self.s.client.get(&url).await?;
        Ok(body)
    }

    async fn get_network_device_function(
        &self,
        chassis_id: &str,
        id: &str,
        _port: Option<&str>,
    ) -> Result<NetworkDeviceFunction, RedfishError> {
        let url = format!(
            "Chassis/{}/NetworkAdapters/NvidiaNetworkAdapter/NetworkDeviceFunctions/{}",
            chassis_id, id
        );
        let (_status_code, body) = self.s.client.get(&url).await?;
        Ok(body)
    }

    /// http://redfish.dmtf.org/schemas/v1/NetworkDeviceFunctionCollection.json
    async fn get_network_device_functions(
        &self,
        chassis_id: &str,
    ) -> Result<Vec<String>, RedfishError> {
        let url = format!(
            "Chassis/{}/NetworkAdapters/NvidiaNetworkAdapter/NetworkDeviceFunctions",
            chassis_id
        );
        self.s.get_members(&url).await
    }

    async fn change_uefi_password(
        &self,
        current_uefi_password: &str,
        new_uefi_password: &str,
    ) -> Result<Option<String>, RedfishError> {
        let mut attributes = HashMap::new();
        let mut data = HashMap::new();
        data.insert("CurrentUefiPassword", current_uefi_password.to_string());
        data.insert("UefiPassword", new_uefi_password.to_string());
        attributes.insert("Attributes", data);
        let url = format!("Systems/{}/Bios/Settings", self.s.system_id());
        let _status_code = self.s.client.patch(&url, attributes).await?;
        Ok(None)
    }

    async fn change_boot_order(&self, boot_array: Vec<String>) -> Result<(), RedfishError> {
        let body = HashMap::from([("Boot", HashMap::from([("BootOrder", boot_array)]))]);
        let url = format!("Systems/{}/Settings", self.s.system_id());
        self.s.client.patch(&url, body).await?;
        Ok(())
    }

    async fn get_service_root(&self) -> Result<ServiceRoot, RedfishError> {
        self.s.get_service_root().await
    }

    async fn get_systems(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_systems().await
    }

    async fn get_managers(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_managers().await
    }

    async fn get_manager(&self) -> Result<Manager, RedfishError> {
        self.s.get_manager().await
    }

    async fn bmc_reset_to_defaults(&self) -> Result<(), RedfishError> {
        let url = format!(
            "Managers/{}/Actions/Manager.ResetToDefaults",
            self.s.manager_id()
        );
        let mut arg = HashMap::new();
        arg.insert("ResetToDefaultsType", "ResetAll".to_string());
        self.s.client.post(&url, arg).await.map(|_resp| Ok(()))?
    }

    async fn get_job_state(&self, job_id: &str) -> Result<JobState, RedfishError> {
        self.s.get_job_state(job_id).await
    }

    async fn get_collection(&self, id: ODataId) -> Result<Collection, RedfishError> {
        self.s.get_collection(id).await
    }

    async fn get_resource(&self, id: ODataId) -> Result<Resource, RedfishError> {
        self.s.get_resource(id).await
    }

    async fn set_boot_order_dpu_first(
        &self,
        _mac_address: Option<&str>,
    ) -> Result<(), RedfishError> {
        Err(RedfishError::NotSupported(
            "set_dpu_first_boot_order".to_string(),
        ))
    }

    async fn clear_uefi_password(
        &self,
        current_uefi_password: &str,
    ) -> Result<Option<String>, RedfishError> {
        self.change_uefi_password(current_uefi_password, "").await
    }

    async fn get_base_mac_address(&self) -> Result<Option<String>, RedfishError> {
        let url = format!("Systems/{}/Oem/Nvidia", self.s.system_id());
        let (_sc, body): (reqwest::StatusCode, HashMap<String, serde_json::Value>) =
            self.s.client.get(url.as_str()).await?;
        Ok(body.get("BaseMAC").map(|v| v.to_string()))
    }

    async fn lockdown_bmc(&self, target: crate::EnabledDisabled) -> Result<(), RedfishError> {
        self.s.lockdown_bmc(target).await
    }

    async fn is_ipmi_over_lan_enabled(&self) -> Result<bool, RedfishError> {
        self.s.is_ipmi_over_lan_enabled().await
    }

    async fn enable_ipmi_over_lan(
        &self,
        target: crate::EnabledDisabled,
    ) -> Result<(), RedfishError> {
        self.s.enable_ipmi_over_lan(target).await
    }

    async fn update_firmware_simple_update(
        &self,
        image_uri: &str,
        targets: Vec<String>,
        transfer_protocol: TransferProtocolType,
    ) -> Result<Task, RedfishError> {
        self.s
            .update_firmware_simple_update(image_uri, targets, transfer_protocol)
            .await
    }

    async fn enable_rshim_bmc(&self) -> Result<(), RedfishError> {
        let data = HashMap::from([("BmcRShim", HashMap::from([("BmcRShimEnabled", true)]))]);

        self.s
            .client
            .patch("Managers/Bluefield_BMC/Oem/Nvidia", data)
            .await
            .map(|_status_code| Ok(()))?
    }

    async fn clear_nvram(&self) -> Result<(), RedfishError> {
        self.s.clear_nvram().await
    }

    async fn get_nic_mode(&self) -> Result<Option<NicMode>, RedfishError> {
        let nic_mode = match self.s.bios().await {
            Ok(bios) => self.get_nic_mode_from_bios(bios).await,
            Err(e) => match e {
                RedfishError::HTTPErrorCode {
                    url,
                    status_code,
                    response_body,
                } if status_code == StatusCode::INTERNAL_SERVER_ERROR => {
                    let current_bmc_firmware_version = match self.get_bmc_firmware_version().await {
                        Ok(version) => version,
                        Err(bmc_fw_err) => {
                            tracing::warn!("could not retrieve BMC firmware version, returning the original error to the caller: {bmc_fw_err}");
                            return Err(RedfishError::HTTPErrorCode {
                                url,
                                status_code,
                                response_body,
                            });
                        }
                    };
                    // If the BMC firmware version is less than 24.07, querying the bios attributes on a DPU in NIC mode will return an internal 500 error.
                    let min_bmc_fw_version_to_query_nic_mode_without_error = "BF-24.07-14";
                    match version_compare::compare(
                        current_bmc_firmware_version,
                        min_bmc_fw_version_to_query_nic_mode_without_error,
                    )
                    .is_ok_and(|c| c == version_compare::Cmp::Lt)
                    {
                        true => {
                            let bios: HashMap<String, serde_json::Value> =
                                serde_json::from_str(&response_body).map_err(|e| {
                                    tracing::warn!("could not parse BIOS from the response body, returning the original error to the caller: {e}");
                                    RedfishError::HTTPErrorCode {
                                        url: url.clone(),
                                        status_code,
                                        response_body: response_body.clone(),
                                    }
                                })?;

                            let nic_mode = self.get_nic_mode_from_bios(bios).await.map_err(|e| {
                                tracing::warn!("could not retrieve NIC mode value, returning the original error to the caller: {e}");
                                RedfishError::HTTPErrorCode {
                                    url: url.clone(),
                                    status_code,
                                    response_body: response_body.clone(),
                                }
                            })?;

                            // we only want to parse the NIC mode from the response body if the DPU is actually in NIC mode
                            // it is unexpected for a DPU to return an error when querying the BIOS attributes if it is in DPU mode
                            match nic_mode {
                                NicMode::Dpu => Err(RedfishError::HTTPErrorCode {
                                    url,
                                    status_code,
                                    response_body,
                                }),
                                NicMode::Nic => Ok(nic_mode),
                            }
                        }
                        false => Err(RedfishError::HTTPErrorCode {
                            url,
                            status_code,
                            response_body,
                        }),
                    }
                }
                _ => Err(e),
            },
        }?;

        Ok(Some(nic_mode))
    }

    async fn is_infinite_boot_enabled(&self) -> Result<Option<bool>, RedfishError> {
        self.s.is_infinite_boot_enabled().await
    }
}

impl Bmc {
    async fn patch_bios_setting(
        &self,
        data: HashMap<&str, HashMap<&str, String>>,
    ) -> Result<(), RedfishError> {
        let url = format!("Systems/{}/Bios/Settings", self.s.system_id());
        self.s
            .client
            .patch(&url, data)
            .await
            .map(|_status_code| Ok(()))?
    }

    async fn set_host_privilege_level(
        &self,
        level: HostPrivilegeLevel,
    ) -> Result<(), RedfishError> {
        // There is a change in the Attribute naming in DPU BMC 24.10, it no longer has spaces
        // Because of this we need to try both cases of the named key
        let key = "HostPrivilegeLevel";
        let data = HashMap::from([("Attributes", HashMap::from([(key, level.to_string())]))]);

        match self.patch_bios_setting(data).await {
            Ok(_) => return Ok(()),
            Err(RedfishError::HTTPErrorCode { response_body, .. })
                if response_body.contains(key) =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }?;

        let key = "Host Privilege Level";
        let data = HashMap::from([("Attributes", HashMap::from([(key, level.to_string())]))]);

        self.patch_bios_setting(data)
            .await
            .map(|_status_code| Ok(()))?
    }

    async fn is_bf2(&self) -> Result<bool, RedfishError> {
        let chassis = self.get_chassis("Card1").await?;
        Ok(chassis
            .model
            .is_none_or(|m| m.as_str().to_lowercase().as_str().contains("bluefield 2")))
    }

    async fn set_host_rshim(
        &self,
        enabled: bool,
        ignore_if_unsupported: bool,
    ) -> Result<(), RedfishError> {
        if self.is_bf2().await? {
            return Ok(());
        }

        let value = if enabled { "Enabled" } else { "Disabled" };

        let data = HashMap::from([("HostRshim", value)]);
        let url = format!(
            "Systems/{}/Oem/Nvidia/Actions/HostRshim.Set",
            self.s.system_id()
        );

        match self.s.client.post(&url, data).await {
            Ok(_) => Ok(()),
            Err(e) => match e {
                RedfishError::HTTPErrorCode { status_code, .. }
                    if status_code == StatusCode::NOT_FOUND && ignore_if_unsupported =>
                {
                    tracing::warn!(
                        "This BF3 does not support setting host rshim; the BMC fw maybe too old"
                    );
                    Ok(())
                }
                e => Err(e),
            },
        }
    }

    async fn set_internal_cpu_model(&self, model: InternalCPUModel) -> Result<(), RedfishError> {
        // There is a change in the Attribute naming in DPU BMC 24.10, it no longer has spaces
        // Because of this we need to try both cases of the named key
        let key = "InternalCPUModel";
        let data = HashMap::from([("Attributes", HashMap::from([(key, model.to_string())]))]);

        match self.patch_bios_setting(data).await {
            Ok(_) => return Ok(()),
            Err(RedfishError::HTTPErrorCode { response_body, .. })
                if response_body.contains(key) =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }?;

        let key = "Internal CPU Model";
        let data = HashMap::from([("Attributes", HashMap::from([(key, model.to_string())]))]);

        self.patch_bios_setting(data)
            .await
            .map(|_status_code| Ok(()))?
    }

    async fn set_boot_override(
        &self,
        override_target: BootSourceOverrideTarget,
        override_enabled: BootSourceOverrideEnabled,
    ) -> Result<(), RedfishError> {
        let mut data: HashMap<String, String> = HashMap::new();
        data.insert("BootSourceOverrideMode".to_string(), "UEFI".to_string());
        data.insert(
            "BootSourceOverrideEnabled".to_string(),
            format!("{}", override_enabled),
        );
        data.insert(
            "BootSourceOverrideTarget".to_string(),
            format!("{}", override_target),
        );
        let url = format!("Systems/{}/Settings ", self.s.system_id());
        self.s
            .client
            .patch(&url, HashMap::from([("Boot", data)]))
            .await?;
        Ok(())
    }

    // name: The name of the device you want to make the first boot choice.
    async fn set_boot_order(&self, name: &BootOptionName) -> Result<(), RedfishError> {
        let boot_array = match self.get_boot_options_ids_with_first(name).await? {
            None => {
                return Err(RedfishError::MissingBootOption(name.to_string().to_owned()));
            }
            Some(b) => b,
        };
        self.change_boot_order(boot_array).await
    }

    // A Vec of string boot option names, with the one you want first.
    //
    // Example: get_boot_options_ids_with_first(lenovo::BootOptionName::Network) might return
    // ["Boot0003", "Boot0002", "Boot0001", "Boot0004"] where Boot0003 is Network. It has been
    // moved to the front ready for sending as an update.
    // The order of the other boot options does not change.
    //
    // If the boot option you want is not found returns Ok(None)
    async fn get_boot_options_ids_with_first(
        &self,
        with_name: &BootOptionName,
    ) -> Result<Option<Vec<String>>, RedfishError> {
        let with_name_str = with_name.to_string();
        let mut ordered = Vec::new(); // the final boot options
        let boot_options = self.s.get_system().await?.boot.boot_order;
        for member in boot_options {
            let b: BootOption = self.s.get_boot_option(member.as_str()).await?;
            if b.display_name.starts_with(with_name_str) {
                ordered.insert(0, b.id);
            } else {
                ordered.push(b.id);
            }
        }
        Ok(Some(ordered))
    }

    // dpu stores the sel as part of the system? there's a LogServices for the bmc too, but no sel
    async fn get_system_event_log(&self) -> Result<Vec<LogEntry>, RedfishError> {
        let url = format!("Systems/{}/LogServices/SEL/Entries", self.s.system_id());
        let (_status_code, log_entry_collection): (_, LogEntryCollection) =
            self.s.client.get(&url).await?;
        let log_entries = log_entry_collection.members;
        Ok(log_entries)
    }

    // get bmc firmware version for the DPU
    async fn get_bmc_firmware_version(&self) -> Result<String, RedfishError> {
        let inventory_list = self.get_software_inventories().await?;
        if let Some(bmc_firmware) = inventory_list.iter().find(|i| i.contains("BMC_Firmware")) {
            if let Some(bmc_firmware_version) =
                self.get_firmware(bmc_firmware.as_str()).await?.version
            {
                Ok(bmc_firmware_version)
            } else {
                Err(RedfishError::MissingKey {
                    key: "BMC_Firmware".to_owned(),
                    url: format!("UpdateService/FirmwareInventory/{bmc_firmware}"),
                })
            }
        } else {
            Err(RedfishError::MissingKey {
                key: "BMC_Firmware".to_owned(),
                url: "UpdateService/FirmwareInventory".to_owned(),
            })
        }
    }

    async fn get_nic_mode_from_bios(
        &self,
        bios: HashMap<String, serde_json::Value>,
    ) -> Result<NicMode, RedfishError> {
        match bios.get("Attributes") {
            Some(bios_attributes) => {
                if let Some(nic_mode) = bios_attributes
                    .get("NicMode")
                    .and_then(|v| v.as_str().and_then(|v| NicMode::from_str(v).ok()))
                {
                    Ok(nic_mode)
                } else {
                    return Err(RedfishError::MissingKey {
                        key: "NicMode".to_owned(),
                        url: format!("Systems/{}/Bios", self.s.system_id()),
                    });
                }
            }
            None => {
                return Err(RedfishError::MissingKey {
                    key: "Attributes".to_owned(),
                    url: format!("Systems/{}/Bios", self.s.system_id()),
                });
            }
        }
    }
}
