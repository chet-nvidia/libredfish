use std::collections::HashMap;

use crate::{
    model::{
        boot,
        chassis::Chassis,
        network_device_function::NetworkDeviceFunction,
        oem::nvidia::{HostPrivilegeLevel, InternalCPUModel},
        oem::supermicro,
        power::Power,
        secure_boot::SecureBoot,
        sel::LogEntry,
        service_root::ServiceRoot,
        software_inventory::SoftwareInventory,
        task::Task,
        thermal::Thermal,
        BootOption, Commandshell, ComputerSystem, InvalidValueError, Manager, ODataId,
    },
    standard::RedfishStandard,
    Boot, BootOptions, EnabledDisabled, PCIeDevice, PowerState, Redfish, RedfishError, RoleId,
    Status, StatusInternal, SystemPowerControl,
};

pub struct Bmc {
    s: RedfishStandard,
}

impl Bmc {
    pub fn new(s: RedfishStandard) -> Result<Bmc, RedfishError> {
        Ok(Bmc { s })
    }
}

impl Redfish for Bmc {
    fn create_user(
        &self,
        username: &str,
        password: &str,
        role_id: RoleId,
    ) -> Result<(), RedfishError> {
        self.s.create_user(username, password, role_id)
    }

    fn change_password(&self, username: &str, new_password: &str) -> Result<(), RedfishError> {
        let account_ids = self.get_members("AccountService/Accounts")?;
        let mut maybe_user_id = None;
        for id in account_ids {
            let account = self.s.get_account(&id)?;
            if account.username == username {
                maybe_user_id = Some(id);
                break;
            }
        }
        match maybe_user_id {
            Some(user_id) => self.s.change_password(&user_id, new_password),
            None => Err(RedfishError::UserNotFound(username.to_string())),
        }
    }

    fn get_power_state(&self) -> Result<PowerState, RedfishError> {
        self.s.get_power_state()
    }

    fn get_power_metrics(&self) -> Result<Power, RedfishError> {
        self.s.get_power_metrics()
    }

    fn power(&self, action: SystemPowerControl) -> Result<(), RedfishError> {
        self.s.power(action)
    }

    fn bmc_reset(&self) -> Result<(), RedfishError> {
        self.s.bmc_reset()
    }

    fn get_thermal_metrics(&self) -> Result<Thermal, RedfishError> {
        self.s.get_thermal_metrics()
    }

    fn get_system_event_log(&self) -> Result<Vec<LogEntry>, RedfishError> {
        self.s.get_system_event_log()
    }

    fn bios(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        self.s.bios()
    }

    fn forge_setup(&self) -> Result<(), RedfishError> {
        self.set_tpms("TPM 2.0")?;
        self.setup_serial_console()?;
        // always do system lockdown last.
        self.lockdown(EnabledDisabled::Enabled)
    }

    fn lockdown(&self, target: EnabledDisabled) -> Result<(), RedfishError> {
        use EnabledDisabled::*;
        match target {
            Enabled => {
                self.set_host_interfaces(Disabled)?;
                self.set_kcs_privilege(supermicro::Privilege::Callback)?;
                self.set_syslockdown(Enabled)?; // Lock last
            }
            Disabled => {
                self.set_syslockdown(Disabled)?; // Unlock first
                self.set_kcs_privilege(supermicro::Privilege::Administrator)?;
                self.set_host_interfaces(Enabled)?;
            }
        }
        Ok(())
    }

    fn lockdown_status(&self) -> Result<Status, RedfishError> {
        let is_hi_on = self.is_host_interface_enabled()?;
        let kcs_privilege = self.get_kcs_privilege()?;
        let is_syslockdown = self.get_syslockdown()?;
        let message = format!("SysLockdownEnabled={is_syslockdown}, kcs_privilege={kcs_privilege}, host_interface_enabled={is_hi_on}");
        let is_locked =
            is_syslockdown && kcs_privilege == supermicro::Privilege::Callback && !is_hi_on;
        let is_unlocked =
            !is_syslockdown && kcs_privilege == supermicro::Privilege::Administrator && is_hi_on;
        Ok(Status {
            message,
            status: if is_locked {
                StatusInternal::Enabled
            } else if is_unlocked {
                StatusInternal::Disabled
            } else {
                StatusInternal::Partial
            },
        })
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        let url = format!("Managers/{}", self.s.manager_id());
        let body = Commandshell {
            service_enabled: true,
            max_concurrent_sessions: 1,
            connect_types_supported: vec!["SSH".to_string(), "IPMI".to_string()],
            enabled: None, // Supermicro doens't have this
        };
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    fn serial_console_status(&self) -> Result<Status, RedfishError> {
        let s_interface = self.s.get_serial_interface()?;
        let manager = self.s.get_manager()?;
        let sr = &manager.serial_console;
        let is_enabled = sr.service_enabled
            && sr.max_concurrent_sessions != 0
            && s_interface.is_supermicro_default();
        let status = if is_enabled {
            StatusInternal::Enabled
        } else {
            StatusInternal::Disabled
        };
        Ok(Status {
            message: String::new(),
            status,
        })
    }

    fn get_boot_options(&self) -> Result<BootOptions, RedfishError> {
        self.s.get_boot_options()
    }

    fn get_boot_option(&self, option_id: &str) -> Result<BootOption, RedfishError> {
        self.s.get_boot_option(option_id)
    }

    // Boot from this device once then go back to the normal boot order
    fn boot_once(&self, target: Boot) -> Result<(), RedfishError> {
        self.set_boot(target, true)
    }

    /// Set which device we should boot from first.
    ///
    /// Instead of changing the normal boot order we set a continuous boot override.
    /// Setting the boot order is complicated and requires string matching on
    /// the DisplayName of every BootOptions. This is more reliable.
    fn boot_first(&self, target: Boot) -> Result<(), RedfishError> {
        self.set_boot(target, false)
    }

    /// Supermicro BMC does not appear to have this.
    fn clear_tpm(&self) -> Result<(), RedfishError> {
        Err(RedfishError::NotSupported("clear_tpm".to_string()))
    }

    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        let url = format!("Systems/{}/Bios/SD", self.s.system_id());
        // Supermicro doesn't include the Attributes key if there are no pending changes
        self.s
            .pending_attributes(&url)
            .map(|m| {
                m.into_iter()
                    .collect::<HashMap<String, serde_json::Value>>()
            })
            .or_else(|err| match err {
                RedfishError::MissingKey { .. } => Ok(HashMap::new()),
                err => Err(err),
            })
    }

    // TODO: This resets the pending Bios changes to their default values,
    // but DOES NOT CLEAR THEM. We don't know how to do that, or if Supermicro supports it at all.
    fn clear_pending(&self) -> Result<(), RedfishError> {
        let url = format!("Systems/{}/Bios/SD", self.s.system_id());
        self.s.clear_pending_with_url(&url)
    }

    fn pcie_devices(&self) -> Result<Vec<PCIeDevice>, RedfishError> {
        let Some(chassis_id) = self.get_chassis_all()?.into_iter().next().take() else {
            return Err(RedfishError::NoContent);
        };
        let url = format!("Chassis/{chassis_id}/PCIeDevices");
        let device_ids = self.get_members(&url)?;
        let mut out = Vec::with_capacity(device_ids.len());
        for device_id in device_ids {
            out.push(self.get_pcie_device(&chassis_id, &device_id)?);
        }
        Ok(out)
    }

    fn update_firmware(
        &self,
        firmware: std::fs::File,
    ) -> Result<crate::model::task::Task, RedfishError> {
        self.s.update_firmware(firmware)
    }

    fn get_tasks(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_tasks()
    }

    fn get_task(&self, id: &str) -> Result<crate::model::task::Task, RedfishError> {
        self.s.get_task(id)
    }

    fn get_firmware(&self, id: &str) -> Result<SoftwareInventory, RedfishError> {
        self.s.get_firmware(id)
    }

    fn get_software_inventories(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_software_inventories()
    }

    fn get_system(&self) -> Result<ComputerSystem, RedfishError> {
        self.s.get_system()
    }

    fn add_secure_boot_certificate(&self, pem_cert: &str) -> Result<Task, RedfishError> {
        self.s.add_secure_boot_certificate(pem_cert)
    }

    fn get_secure_boot(&self) -> Result<SecureBoot, RedfishError> {
        self.s.get_secure_boot()
    }

    fn enable_secure_boot(&self) -> Result<(), RedfishError> {
        self.s.enable_secure_boot()
    }

    fn disable_secure_boot(&self) -> Result<(), RedfishError> {
        self.s.disable_secure_boot()
    }

    fn get_network_device_function(
        &self,
        chassis_id: &str,
        id: &str,
    ) -> Result<NetworkDeviceFunction, RedfishError> {
        self.s.get_network_device_function(chassis_id, id)
    }

    fn get_network_device_functions(&self, chassis_id: &str) -> Result<Vec<String>, RedfishError> {
        self.s.get_network_device_functions(chassis_id)
    }

    fn get_chassis_all(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_chassis_all()
    }

    fn get_chassis(&self, id: &str) -> Result<Chassis, RedfishError> {
        self.s.get_chassis(id)
    }

    fn get_ports(&self, chassis_id: &str) -> Result<Vec<String>, RedfishError> {
        self.s.get_ports(chassis_id)
    }

    fn get_port(&self, chassis_id: &str, id: &str) -> Result<crate::NetworkPort, RedfishError> {
        self.s.get_port(chassis_id, id)
    }

    fn get_ethernet_interfaces(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_ethernet_interfaces()
    }

    fn get_ethernet_interface(&self, id: &str) -> Result<crate::EthernetInterface, RedfishError> {
        self.s.get_ethernet_interface(id)
    }

    fn change_uefi_password(
        &self,
        _current_uefi_password: &str,
        _new_uefi_password: &str,
    ) -> Result<(), RedfishError> {
        Err(RedfishError::NotSupported(
            "change_uefi_password".to_string(),
        ))
    }

    fn change_boot_order(&self, boot_array: Vec<String>) -> Result<(), RedfishError> {
        self.s.change_boot_order(boot_array)
    }
    fn set_internal_cpu_model(&self, model: InternalCPUModel) -> Result<(), RedfishError> {
        self.s.set_internal_cpu_model(model)
    }

    fn set_host_privilege_level(&self, level: HostPrivilegeLevel) -> Result<(), RedfishError> {
        self.s.set_host_privilege_level(level)
    }

    fn get_service_root(&self) -> Result<ServiceRoot, RedfishError> {
        self.s.get_service_root()
    }

    fn get_systems(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_systems()
    }

    fn get_managers(&self) -> Result<Vec<String>, RedfishError> {
        self.s.get_managers()
    }

    fn get_manager(&self) -> Result<Manager, RedfishError> {
        self.s.get_manager()
    }

    fn bmc_reset_to_defaults(&self) -> Result<(), RedfishError> {
        self.s.bmc_reset_to_defaults()
    }
}

impl Bmc {
    // TODO: move this to standard.rs and replace the model/ files that just have `members`
    // with this. Probably make a model `Collection` type.
    fn get_members(&self, url: &str) -> Result<Vec<String>, RedfishError> {
        let (_, mut body): (_, HashMap<String, serde_json::Value>) = self.s.client.get(url)?;
        let key = "Members";
        let members_json = body.remove(key).ok_or_else(|| RedfishError::MissingKey {
            key: key.to_string(),
            url: url.to_string(),
        })?;
        let Ok(members) = serde_json::from_value::<Vec<ODataId>>(members_json) else {
            return Err(RedfishError::InvalidKeyType {
                key: key.to_string(),
                expected_type: "Vec<ODataId>".to_string(),
                url: url.to_string(),
            });
        };
        let member_ids: Vec<String> = members
            .into_iter()
            .map(|d| d.odata_id.split('/').last().unwrap().to_string())
            .collect();
        Ok(member_ids)
    }

    fn get_kcs_privilege(&self) -> Result<supermicro::Privilege, RedfishError> {
        let url = format!(
            "Managers/{}/Oem/Supermicro/KCSInterface",
            self.s.manager_id()
        );
        let (_, body): (_, HashMap<String, serde_json::Value>) = self.s.client.get(&url)?;
        let key = "Privilege";
        let p_str = body
            .get(key)
            .ok_or_else(|| RedfishError::MissingKey {
                key: key.to_string(),
                url: url.to_string(),
            })?
            .as_str()
            .ok_or_else(|| RedfishError::InvalidKeyType {
                key: key.to_string(),
                expected_type: "&str".to_string(),
                url: url.to_string(),
            })?;
        p_str.parse().map_err(|_| RedfishError::InvalidKeyType {
            key: key.to_string(),
            expected_type: "oem::supermicro::Privilege".to_string(),
            url: url.to_string(),
        })
    }

    fn set_kcs_privilege(&self, privilege: supermicro::Privilege) -> Result<(), RedfishError> {
        let url = format!(
            "Managers/{}/Oem/Supermicro/KCSInterface",
            self.s.manager_id()
        );
        let body = HashMap::from([("Privilege", privilege.to_string())]);
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    fn is_host_interface_enabled(&self) -> Result<bool, RedfishError> {
        let url = format!("Managers/{}/HostInterfaces", self.s.manager_id());
        let host_interface_ids = self.get_members(&url)?;
        let num_interfaces = host_interface_ids.len();
        if num_interfaces != 1 {
            return Err(RedfishError::InvalidValue {
                url,
                field: "Members".to_string(),
                err: InvalidValueError(format!(
                    "Expected a single host interface, found {num_interfaces}"
                )),
            });
        }

        let url = format!(
            "Managers/{}/HostInterfaces/{}",
            self.s.manager_id(),
            host_interface_ids[0]
        );
        let (_, body): (_, HashMap<String, serde_json::Value>) = self.s.client.get(&url)?;
        let key = "InterfaceEnabled";
        body.get(key)
            .ok_or_else(|| RedfishError::MissingKey {
                key: key.to_string(),
                url: url.to_string(),
            })?
            .as_bool()
            .ok_or_else(|| RedfishError::InvalidKeyType {
                key: key.to_string(),
                expected_type: "bool".to_string(),
                url: url.to_string(),
            })
    }

    // The HostInterface allows remote BMC access
    fn set_host_interfaces(&self, target: EnabledDisabled) -> Result<(), RedfishError> {
        let url = format!("Managers/{}/HostInterfaces", self.s.manager_id());
        // I have only seen exactly one, but you can't be too careful
        let host_iface_ids = self.get_members(&url)?;
        for iface_id in host_iface_ids {
            self.set_host_interface(&iface_id, target)?;
        }
        Ok(())
    }

    fn set_host_interface(
        &self,
        iface_id: &str,
        target: EnabledDisabled,
    ) -> Result<(), RedfishError> {
        let url = format!("Managers/{}/HostInterfaces/{iface_id}", self.s.manager_id());
        let body = HashMap::from([("InterfaceEnabled", target == EnabledDisabled::Enabled)]);
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    fn get_syslockdown(&self) -> Result<bool, RedfishError> {
        let url = format!(
            "Managers/{}/Oem/Supermicro/SysLockdown",
            self.s.manager_id()
        );
        let (_, body): (_, HashMap<String, serde_json::Value>) = self.s.client.get(&url)?;
        let key = "SysLockdownEnabled";
        body.get(key)
            .ok_or_else(|| RedfishError::MissingKey {
                key: key.to_string(),
                url: url.to_string(),
            })?
            .as_bool()
            .ok_or_else(|| RedfishError::InvalidKeyType {
                key: key.to_string(),
                expected_type: "bool".to_string(),
                url: url.to_string(),
            })
    }

    fn set_syslockdown(&self, target: EnabledDisabled) -> Result<(), RedfishError> {
        let url = format!(
            "Managers/{}/Oem/Supermicro/SysLockdown",
            self.s.manager_id()
        );
        let body = HashMap::from([("SysLockdownEnabled", target.is_enabled())]);
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    fn set_boot(&self, target: Boot, once: bool) -> Result<(), RedfishError> {
        let url = format!("Systems/{}", self.s.system_id());
        let boot = boot::Boot {
            boot_source_override_target: Some(match target {
                Boot::Pxe => boot::BootSourceOverrideTarget::Pxe,
                Boot::HardDisk => boot::BootSourceOverrideTarget::Hdd,
                Boot::UefiHttp => {
                    return Err(RedfishError::NotSupported(
                        "No Supermicro UefiHttp implementation".to_string(),
                    ))
                }
            }),
            boot_source_override_enabled: Some(if once {
                boot::BootSourceOverrideEnabled::Once
            } else {
                boot::BootSourceOverrideEnabled::Continuous
            }),
            boot_source_override_mode: Some(boot::BootSourceOverrideMode::UEFI),
            ..Default::default()
        };
        let body = HashMap::from([("Boot", boot)]);
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    // tpm_type: Defined in registries/BiosAttributeRegistry.1.0.0.json/index.json
    fn set_tpms(&self, tpm_type: &str) -> Result<(), RedfishError> {
        let url = format!("Systems/{}/Bios", self.s.system_id());
        let attrs_val = self.s.bios_attributes()?;
        let attrs = attrs_val
            .as_object()
            .ok_or_else(|| RedfishError::InvalidKeyType {
                key: "Attributes".to_string(),
                expected_type: "Object".to_string(),
                url: url.clone(),
            })?;

        let mut new_vals = HashMap::new();
        for (k, v) in attrs {
            // Our test machine has DeviceSelect#0061 and DeviceSelect#0072, presumably one per CPU
            if k.starts_with("DeviceSelect") && v != tpm_type {
                new_vals.insert(k, tpm_type);
            }
        }
        let body = HashMap::from([("Attributes", new_vals)]);
        self.s.client.patch(&url, body).map(|_status_code| ())
    }

    fn get_pcie_device(
        &self,
        chassis_id: &str,
        device_id: &str,
    ) -> Result<PCIeDevice, RedfishError> {
        let url = format!("Chassis/{chassis_id}/PCIeDevices/{device_id}");
        let (_, body): (_, PCIeDevice) = self.s.client.get(&url)?;
        Ok(body)
    }
}
