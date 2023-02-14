use std::collections::HashMap;

use crate::{
    model::{
        oem::dell::{
            DellBiosLockdownAttrs, DellBiosSerialAttrs, DellBiosTpmAttrs, DellBmcLockdown,
            DellBmcRemoteAccess, DellBootDevices, DellIpmiSol, DellSerialRedirection,
            DellServerBoot, DellServerBootAttrs, RedfishSettingsApplyTime, SerialCommSettings,
            SerialPortExtSettings, SerialPortSettings, SerialPortTermSettings,
            SetDellBiosLockdownAttrs, SetDellBiosSerialAttrs, SetDellBiosTpmAttrs,
            SetDellBmcLockdown, SetDellBmcRemoteAccess, SetDellFirstBootDevice,
            SetDellSettingsApplyTime, Tpm2HierarchySettings, UefiVariableAccessSettings,
        },
        OnOff,
    },
    network::NetworkConfig,
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

    fn get_bios_attributes(&self) -> Result<HashMap<String, serde_json::Value>, RedfishError> {
        self.s.get_bios_attributes()
    }

    fn lockdown(&self, target: EnabledDisabled) -> Result<(), RedfishError> {
        use EnabledDisabled::*;
        match target {
            Enabled => {
                self.enable_bios_lockdown()?;
                self.enable_bmc_lockdown(DellBootDevices::PXE, false)
            }
            Disabled => {
                self.disable_bmc_lockdown(DellBootDevices::PXE, false)?;
                self.disable_bios_lockdown()
            }
        }
    }

    fn setup_serial_console(&self) -> Result<(), RedfishError> {
        self.setup_bmc_remote_access()?;

        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let serial_console = DellBiosSerialAttrs {
            serial_comm: SerialCommSettings::OnConRedir,
            serial_port_address: SerialPortSettings::Com1,
            ext_serial_connector: SerialPortExtSettings::Serial1,
            fail_safe_baud: "115200".to_string(),
            con_term_type: SerialPortTermSettings::Vt100Vt220,
            redir_after_boot: EnabledDisabled::Enabled,
        };
        let set_serial_attrs = SetDellBiosSerialAttrs {
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
            Boot::Pxe => self.set_boot_first(DellBootDevices::PXE, true),
            Boot::HardDisk => self.set_boot_first(DellBootDevices::HDD, true),
        }
    }

    fn boot_first(&self, target: Boot) -> Result<(), RedfishError> {
        match target {
            Boot::Pxe => self.set_boot_first(DellBootDevices::PXE, false),
            Boot::HardDisk => self.set_boot_first(DellBootDevices::HDD, false),
        }
    }

    fn clear_tpm(&self) -> Result<(), RedfishError> {
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset,
        };
        let tpm = DellBiosTpmAttrs {
            tpm_security: OnOff::On,
            tpm2_hierarchy: Tpm2HierarchySettings::Clear,
        };
        let set_tpm_clear = SetDellBiosTpmAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: tpm,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s.net.patch(&url, set_tpm_clear).map(|_status_code| ())
    }
}

impl Bmc {
    fn set_boot_first(&self, entry: DellBootDevices, once: bool) -> Result<(), RedfishError> {
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset,
        };
        let boot_entry = DellServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let boot = DellServerBootAttrs {
            server_boot: boot_entry,
        };
        let set_boot = SetDellFirstBootDevice {
            redfish_settings_apply_time: apply_time,
            attributes: boot,
        };
        let url = format!("Managers/{}/Attributes", self.s.manager_id());
        self.s.net.patch(&url, set_boot).map(|_status_code| ())
    }
    fn enable_bios_lockdown(&self) -> Result<(), RedfishError> {
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let lockdown = DellBiosLockdownAttrs {
            in_band_manageability_interface: EnabledDisabled::Disabled,
            uefi_variable_access: UefiVariableAccessSettings::Controlled,
        };
        let set_lockdown_attrs = SetDellBiosLockdownAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_lockdown_attrs)
            .map(|_status_code| ())
    }

    fn enable_bmc_lockdown(&self, entry: DellBootDevices, once: bool) -> Result<(), RedfishError> {
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset,
        };
        let boot_entry = DellServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let lockdown = DellBmcLockdown {
            system_lockdown: EnabledDisabled::Enabled,
            racadm_enable: EnabledDisabled::Disabled,
            server_boot: boot_entry,
        };
        let set_bmc_lockdown = SetDellBmcLockdown {
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
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let lockdown = DellBiosLockdownAttrs {
            in_band_manageability_interface: EnabledDisabled::Enabled,
            uefi_variable_access: UefiVariableAccessSettings::Standard,
        };
        let set_lockdown_attrs = SetDellBiosLockdownAttrs {
            redfish_settings_apply_time: apply_time,
            attributes: lockdown,
        };
        let url = format!("Systems/{}/Bios/Settings/", self.s.system_id());
        self.s
            .net
            .patch(&url, set_lockdown_attrs)
            .map(|_status_code| ())
    }

    fn disable_bmc_lockdown(&self, entry: DellBootDevices, once: bool) -> Result<(), RedfishError> {
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::Immediate, // bmc settings don't require reboot
        };
        let boot_entry = DellServerBoot {
            first_boot_device: entry,
            boot_once: if once {
                EnabledDisabled::Enabled
            } else {
                EnabledDisabled::Disabled
            },
        };
        let lockdown = DellBmcLockdown {
            system_lockdown: EnabledDisabled::Disabled,
            racadm_enable: EnabledDisabled::Enabled,
            server_boot: boot_entry,
        };
        let set_bmc_lockdown = SetDellBmcLockdown {
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
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::Immediate,
        };
        let serial_redirect = DellSerialRedirection {
            enable: EnabledDisabled::Enabled,
        };
        let ipmi_sol_settings = DellIpmiSol {
            enable: EnabledDisabled::Enabled,
            baud_rate: "115200".to_string(),
            min_privilege: "Administrator".to_string(),
        };
        let remote_access = DellBmcRemoteAccess {
            ssh_enable: EnabledDisabled::Enabled,
            serial_redirection: serial_redirect,
            ipmi_lan_enable: EnabledDisabled::Enabled,
            ipmi_sol: ipmi_sol_settings,
        };
        let set_remote_access = SetDellBmcRemoteAccess {
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
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let tpm = DellBiosTpmAttrs {
            tpm_security: OnOff::On,
            tpm2_hierarchy: Tpm2HierarchySettings::Enabled,
        };
        let set_tpm_enabled = SetDellBiosTpmAttrs {
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
        let apply_time = SetDellSettingsApplyTime {
            apply_time: RedfishSettingsApplyTime::OnReset, // requires reboot to apply
        };
        let tpm = DellBiosTpmAttrs {
            tpm_security: OnOff::Off,
            tpm2_hierarchy: Tpm2HierarchySettings::Disabled,
        };
        let set_tpm_disabled = SetDellBiosTpmAttrs {
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
