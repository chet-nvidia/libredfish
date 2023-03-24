/* Redfish API test/example client
 * Also useful for debugging BMC's in inconsistent state.
 *
 * USAGE: ./client -H 10.153.145.103 -U TheBMCUsername -P TheBMCPassword -c get_power_state
 * -H: IP address of the BMC's Redfish API. Should be HTTPS on port 443.
 * Run with no params for help.
 * Run with `-v` for more output.
 */

use libredfish::{Boot, EnabledDisabled, Endpoint, SystemPowerControl};
use tracing::{error, info};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::prelude::*;

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = getopts::Options::new();

    let pool = libredfish::RedfishClientPool::builder().build()?;
    let mut endpoint = Endpoint::default();

    opts.optflag("h", "help", "Print this help");
    opts.optflag("v", "verbose", "Log at DEBUG level. Default is INFO");
    opts.optopt(
        "H",
        "hostname",
        "Required. Hostname or IP address of BMC Redfish API",
        "HOST",
    );
    opts.optopt("U", "username", "BMC username", "USER");
    opts.optopt("P", "password", "BMC password", "PASS");
    opts.optopt(
        "c",
        "cmd",
        "Command to run:
                off
                on
                reset
                shutdown
                restart
                get_power_state
                tpm_reset
                serial_enable
                serial_status
                lockdown_enable
                lockdown_disable
                lockdown_status
                bios_attrs
                boot_pxe
                boot_hdd
                boot_once_pxe
                boot_once_hdd
                pending",
        "CMD",
    );

    let args_given = opts.parse(&args[1..]).unwrap();
    if args_given.opt_present("h") || !args_given.opt_present("H") {
        eprintln!(
            "{}",
            opts.usage("client -H bmc_ip -U bmc_user -P bmc_pass -c cmd")
        );
        return Ok(());
    }
    if args_given.opt_present("H") {
        endpoint.host = args_given.opt_str("H").unwrap();
    }
    if args_given.opt_present("U") {
        endpoint.user = Some(args_given.opt_str("U").unwrap());
    }
    if args_given.opt_present("P") {
        endpoint.password = Some(args_given.opt_str("P").unwrap());
    }

    let log_level = if args_given.opt_present("v") {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let env_filter = EnvFilter::from_default_env()
        .add_directive(log_level.into())
        .add_directive("hyper=warn".parse().unwrap());
    tracing_subscriber::registry()
        .with(Layer::default().compact())
        .with(env_filter)
        .init();

    let redfish = pool.create_client(endpoint)?;

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
            "lockdown_status" => {
                info!("{}", redfish.lockdown_status()?);
            }
            "serial_enable" => {
                redfish.setup_serial_console()?;
                info!("BIOS settings changes require system restart");
            }
            "serial_status" => {
                info!("{}", redfish.serial_console_status()?);
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
                let bios = redfish.bios()?;
                info!("{:#?}", bios);
            }
            "pending" => {
                let pending = redfish.pending()?;
                info!("{:#?}", pending);
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
