use anyhow::anyhow;
use libredfish::{Boot, EnabledDisabled, SystemPowerControl, Vendor};
use tracing::{error, info};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::*;

fn main() -> Result<(), anyhow::Error> {
    let env_filter = EnvFilter::from_default_env()
        .add_directive(LevelFilter::DEBUG.into())
        .add_directive("hyper=warn".parse().unwrap());

    tracing_subscriber::registry()
        .with(Layer::default().compact())
        .with(env_filter)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();
    let mut conf = libredfish::NetworkConfig {
        user: None,
        endpoint: "".to_string(),
        password: None,
        port: None,
    };

    opts.optopt("H", "hostname", "specify hostname or IP address", "HOST");
    opts.optopt("U", "username", "specify authentication username", "USER");
    opts.optopt("P", "password", "specify authentication password", "PASS");
    opts.optopt("V", "vendor", "[Dell|Lenovo|Hpe|Supermicro]", "Unknown");
    opts.optopt("c", "cmd", "specify the command to run: off/on/reset/shutdown/restart/get_power_state/tpm_reset/serial_enable/lockdown_enable/lockdown_disable/bios_attrs/boot_pxe/boot_hdd/boot_once_pxe/boot_once_hdd", "CMD");

    let args_given = opts.parse(&args[1..]).unwrap();
    if args_given.opt_present("H") {
        conf.endpoint = args_given.opt_str("H").unwrap();
    }
    if args_given.opt_present("U") {
        conf.user = Some(args_given.opt_str("U").unwrap());
    }
    if args_given.opt_present("P") {
        conf.password = Some(args_given.opt_str("P").unwrap());
    }
    let vendor_str = args_given
        .opt_str("V")
        .ok_or(anyhow!("Vendor -V is required"))?;
    let vendor = match vendor_str.as_str() {
        "Dell" => Vendor::Dell,
        "Lenovo" => Vendor::Lenovo,
        "Supermicro" => Vendor::Supermicro,
        "Hpe" => Vendor::Hpe,
        _ => return Err(anyhow!(format!("Unknown vendor '{vendor_str}'"))),
    };

    let redfish = libredfish::new(vendor, conf)?;

    if args_given.opt_present("c") {
        use EnabledDisabled::*;
        match args_given.opt_str("c").unwrap().as_str() {
            "get_power_state" => {
                info!("{}", redfish.get_power_state()?);
            }
            "on" => {
                redfish.power(SystemPowerControl::On)?;
            }
            "shutdown" => {
                redfish.power(SystemPowerControl::GracefulShutdown)?;
            }
            "off" => {
                redfish.power(SystemPowerControl::ForceOff)?;
            }
            "restart" => {
                redfish.power(SystemPowerControl::GracefulRestart)?;
            }
            "reset" => {
                redfish.power(SystemPowerControl::ForceRestart)?;
            }
            "lockdown_enable" => {
                redfish.lockdown(Enabled)?;
                info!("BIOS settings changes require system restart");
            }
            "lockdown_disable" => {
                redfish.lockdown(Disabled)?;
                info!("BIOS settings changes require system restart");
            }
            "serial_enable" => {
                redfish.setup_serial_console()?;
                info!("BIOS settings changes require system restart");
            }
            "tpm_reset" => {
                redfish.clear_tpm()?;
                info!("BIOS settings changes require system restart");
            }
            "boot_pxe" => {
                redfish.boot_first(Boot::Pxe)?;
            }
            "boot_hdd" => {
                redfish.boot_first(Boot::HardDisk)?;
            }
            "boot_once_pxe" => {
                redfish.boot_once(Boot::Pxe)?;
            }
            "boot_once_hdd" => {
                redfish.boot_once(Boot::HardDisk)?;
            }
            "bios_attrs" => {
                let bios = redfish.get_bios_attributes()?;
                info!("{:#?}", bios);
            }
            _ => {
                error!(
                    "Unsupported command specified {}",
                    args_given.opt_str("c").unwrap()
                );
            }
        }
    }

    Ok(())
}
