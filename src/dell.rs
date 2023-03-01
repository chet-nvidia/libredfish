use std::collections::HashMap;

use crate::{
    model::{oem::dell, OnOff},
    standard::RedfishStandard,
    Boot, EnabledDisabled, LockdownStatus, LockdownStatusInternal, PowerState, Redfish,
    RedfishError, SystemPowerControl,
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
        self.delete_job_queue()?;

        use EnabledDisabled::*;
        match target {
            Enabled => {
                self.enable_bios_lockdown()?;
                self.enable_bmc_lockdown(dell::BootDevices::PXE, false)
            }
            Disabled => {
                self.disable_bmc_lockdown(dell::BootDevices::PXE, false)?;
                self.disable_bios_lockdown()
            }
        }
    }

    fn lockdown_status(&self) -> Result<LockdownStatus, RedfishError> {
        let mut message = String::new();
        let enabled = EnabledDisabled::Enabled.to_string();
        let disabled = EnabledDisabled::Disabled.to_string();

        // BIOS lockdown

        let url = format!("Systems/{}/Bios", self.s.system_id());
        let (_status_code, bios): (_, dell::Bios) = self.s.net.get(&url)?;

        let in_band = bios.attributes.in_band_manageability_interface;
        let uefi_var = bios.attributes.uefi_variable_access;
        message.push_str(&format!(
            "BIOS: in_band_manageability_interface={in_band}, uefi_variable_access={uefi_var}. "
        ));

        let is_bios_locked = in_band == disabled
            && uefi_var == dell::UefiVariableAccessSettings::Controlled.to_string();
        let is_bios_unlocked = in_band == enabled
            && uefi_var == dell::UefiVariableAccessSettings::Standard.to_string();

        // BMC lockdown

        let manager_id = self.s.manager_id();
        let url = &format!("Managers/{manager_id}/Oem/Dell/DellAttributes/{manager_id}");
        let (_status_code, body): (_, HashMap<String, serde_json::Value>) = self.s.net.get(url)?;
        let key = "Attributes";
        let attrs = body
            .get(key)
            .ok_or_else(|| RedfishError::MissingKey {
                key: key.to_string(),
                url: url.to_string(),
            })?
            .as_object()
            .ok_or_else(|| RedfishError::InvalidKeyType {
                key: key.to_string(),
                expected_type: "Object".to_string(),
                url: url.to_string(),
            })?;

        let key = "Lockdown.1.SystemLockdown";
        let system_lockdown = attrs
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

        let key = "Racadm.1.Enable";
        let racadm = attrs
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

        message.push_str(&format!(
            "BMC: system_lockdown={system_lockdown}, racadm={racadm}."
        ));

        let is_bmc_locked = system_lockdown == enabled && racadm == disabled;
        let is_bmc_unlocked = system_lockdown == disabled && racadm == enabled;

        Ok(LockdownStatus {
            message,
            status: if is_bios_locked && is_bmc_locked {
                LockdownStatusInternal::Enabled
            } else if is_bios_unlocked && is_bmc_unlocked {
                LockdownStatusInternal::Disabled
            } else {
                LockdownStatusInternal::Partial
            },
        })
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        self.delete_job_queue()?;

        self.setup_bmc_remote_access()?;

        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let serial_console = dell::BiosSerialAttrs {
            serial_comm: dell::SerialCommSettings::OnConRedir,
            serial_port_address: dell::SerialPortSettings::Com1,
            ext_serial_connector: dell::SerialPortExtSettings::Serial1,
            fail_safe_baud: "115200".to_string(),
            con_term_type: dell::SerialPortTermSettings::Vt100Vt220,
            redir_after_boot: EnabledDisabled::Enabled,
        };
        let set_serial_attrs = dell::SetBiosSerialAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: serial_console,
        };

        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_serial_attrs)
            .map(|_status_code| ())
    }

    fn boot_once(&self, target: Boot) -> Result<(), RedfishError> {
        match target {
            Boot::Pxe => self.set_boot_first(dell::BootDevices::PXE, true),
            Boot::HardDisk => self.set_boot_first(dell::BootDevices::HDD, true),
        }
    }

    fn boot_first(&self, target: Boot) -> Result<(), RedfishError> {
        match target {
            Boot::Pxe => self.set_boot_first(dell::BootDevices::PXE, false),
            Boot::HardDisk => self.set_boot_first(dell::BootDevices::HDD, false),
        }
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        self.delete_job_queue()?;

        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset,
        };
        let tpm = dell::BiosTpmAttrs {
            tpm_security: OnOff::On,
            tpm2_hierarchy: dell::Tpm2HierarchySettings::Clear,
        };
        let set_tpm_clear = dell::SetBiosTpmAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: tpm,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s.net.patch(&url, set_tpm_clear).map(|_status_code| ())
    }

    fn pending(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        self.s.pending()
    }
}

impl Bmc {
    // No changes can be applied if there are pending jobs
    fn delete_job_queue(&self) -> Result<(), RedfishError> {
        let url = format!(
            "Managers/{}/Oem/Dell/DellJobService/Actions/DellJobService.DeleteJobQueue",
            self.s.manager_id()
        );
        let mut body = HashMap::new();
        body.insert("JobID", "JID_CLEARALL".to_string());
        self.s.net.post(&url, body).map(|_status_code| ())
    }

    fn set_boot_first(&self, entry: dell::BootDevices, once: bool) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset,
        };
        let boot_entry = dell::ServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let boot = dell::ServerBootAttrs {
            server_boot: boot_entry,
        };
        let set_boot = dell::SetFirstBootDevice {
            redfish_settings_apply_time: apply_time,
            attributes: boot,
        };
        let url = format!("Managers/{}/Attributes", self.s.manager_id());
        self.s.net.patch(&url, set_boot).map(|_status_code| ())
    }

    fn enable_bios_lockdown(&self) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let lockdown = dell::BiosLockdownAttrs {
            in_band_manageability_interface: EnabledDisabled::Disabled,
            uefi_variable_access: dell::UefiVariableAccessSettings::Controlled,
        };
        let set_lockdown_attrs = dell::SetBiosLockdownAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_lockdown_attrs)
            .map(|_status_code| ())
    }

    fn enable_bmc_lockdown(
        &self,
        entry: dell::BootDevices,
        once: bool,
    ) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset,
        };
        let boot_entry = dell::ServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let lockdown = dell::BmcLockdown {
            system_lockdown: EnabledDisabled::Enabled,
            racadm_enable: EnabledDisabled::Disabled,
            server_boot: boot_entry,
        };
        let set_bmc_lockdown = dell::SetBmcLockdown {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Managers/{}/Attributes", self.s.manager_id());
        self.s
            .net
            .patch(&url, set_bmc_lockdown)
            .map(|_status_code| ())
    }

    fn disable_bios_lockdown(&self) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let lockdown = dell::BiosLockdownAttrs {
            in_band_manageability_interface: EnabledDisabled::Enabled,
            uefi_variable_access: dell::UefiVariableAccessSettings::Standard,
        };
        let set_lockdown_attrs = dell::SetBiosLockdownAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_lockdown_attrs)
            .map(|_status_code| ())
    }

    fn disable_bmc_lockdown(
        &self,
        entry: dell::BootDevices,
        once: bool,
    ) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::Immediate, // bmc settings don't require reboot
        };
        let boot_entry = dell::ServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let lockdown = dell::BmcLockdown {
            system_lockdown: EnabledDisabled::Disabled,
            racadm_enable: EnabledDisabled::Enabled,
            server_boot: boot_entry,
        };
        let set_bmc_lockdown = dell::SetBmcLockdown {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Managers/{}/Attributes", self.s.manager_id());
        self.s
            .net
            .patch(&url, set_bmc_lockdown)
            .map(|_status_code| ())
    }

    fn setup_bmc_remote_access(&self) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::Immediate,
        };
        let serial_redirect = dell::SerialRedirection {
            enable: EnabledDisabled::Enabled,
        };
        let ipmi_sol_settings = dell::IpmiSol {
            enable: EnabledDisabled::Enabled,
            baud_rate: "115200".to_string(),
            min_privilege: "Administrator".to_string(),
        };
        let remote_access = dell::BmcRemoteAccess {
            ssh_enable: EnabledDisabled::Enabled,
            serial_redirection: serial_redirect,
            ipmi_lan_enable: EnabledDisabled::Enabled,
            ipmi_sol: ipmi_sol_settings,
        };
        let set_remote_access = dell::SetBmcRemoteAccess {
            redfish_settings_apply_time: apply_time,
            attributes: remote_access,
        };
        let url = format!("Managers/{}/Attributes", self.s.manager_id());
        self.s
            .net
            .patch(&url, set_remote_access)
            .map(|_status_code| ())
    }

    // TPM is enabled by default so we never call this.
    #[allow(dead_code)]
    fn enable_tpm(&self) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let tpm = dell::BiosTpmAttrs {
            tpm_security: OnOff::On,
            tpm2_hierarchy: dell::Tpm2HierarchySettings::Enabled,
        };
        let set_tpm_enabled = dell::SetBiosTpmAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: tpm,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_tpm_enabled)
            .map(|_status_code| ())
    }

    // Dell supports disabling the TPM. Why would we do this?
    // Lenovo does not support disabling TPM2.0
    #[allow(dead_code)]
    fn disable_tpm(&self) -> Result<(), RedfishError> {
        let apply_time = dell::SetSettingsApplyTime {
            apply_time: dell::RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let tpm = dell::BiosTpmAttrs {
            tpm_security: OnOff::Off,
            tpm2_hierarchy: dell::Tpm2HierarchySettings::Disabled,
        };
        let set_tpm_disabled = dell::SetBiosTpmAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: tpm,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_tpm_disabled)
            .map(|_status_code| ())
    }
}
