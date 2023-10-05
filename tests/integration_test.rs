/// Test against a mockup of BMC. A mockup is a directory of JSON files mirrored from a real BMC>
/// This makes for very good test for GET (e.g. get_power_state) calls, but is only a basic test
/// for POST/PATCH. For those the mockup server checks the path exists but doesn't check the body
/// values, and always returns '204 No Content'.
///
/// See tests/mockup/README for details.
use std::{
    env,
    path::PathBuf,
    process::{Child, Command},
    sync::Once,
    thread::sleep,
    time::Duration,
};

use anyhow::{anyhow, Context};
use libredfish::Redfish;

const ROOT_DIR: &str = env!("CARGO_MANIFEST_DIR");
const PYTHON_VENV_DIR: &str = "libredfish-python-venv";

// Ports we hope are not in use
const DELL_PORT: &str = "8733";
const LENOVO_PORT: &str = "8734";
const NVIDIA_PORT: &str = "8735";

static SETUP: Once = Once::new();

#[test]
fn test_dell() -> Result<(), anyhow::Error> {
    run_integration_test("dell", DELL_PORT)
}

#[test]
fn test_lenovo() -> Result<(), anyhow::Error> {
    run_integration_test("lenovo", LENOVO_PORT)
}

#[test]
fn test_nvidia_dpu() -> Result<(), anyhow::Error> {
    run_integration_test("nvidia_dpu", NVIDIA_PORT)
}

fn nvidia_dpu_integration_test(redfish: &dyn Redfish) -> Result<(), anyhow::Error> {
    let vendor = redfish.get_service_root()?.vendor;
    assert!(vendor.is_some() && vendor.unwrap() == "Nvidia");
    let sw_inventories = redfish.get_software_inventories()?;
    assert!(redfish.get_firmware(&sw_inventories[0])?.version.is_some());
    let boot = redfish.get_system()?.boot;
    let mut boot_array = boot.boot_order;
    assert!(boot_array.len() > 1);
    boot_array.swap(0, 1);
    redfish.change_boot_order(boot_array)?;

    let eth_intefaces = redfish.get_ethernet_interfaces()?;
    assert!(!eth_intefaces.is_empty());
    assert!(redfish
        .get_ethernet_interface(&eth_intefaces[0])?
        .mac_address
        .is_some());

    let chassis = redfish.get_chassis_all()?;
    assert!(!chassis.is_empty());
    assert!(redfish.get_chassis(&chassis[0])?.name.is_some());

    let ports = redfish.get_ports(&chassis[0])?;
    assert!(!ports.is_empty());
    assert!(redfish
        .get_port(&chassis[0], &ports[0])?
        .current_speed_gbps
        .is_some());

    let netdev_funcs = redfish.get_network_device_functions(&chassis[0])?;
    assert!(!netdev_funcs.is_empty());
    assert!(redfish
        .get_network_device_function(&chassis[0], &netdev_funcs[0])?
        .ethernet
        .and_then(|ethernet| ethernet.mac_address)
        .is_some());

    Ok(())
}

fn run_integration_test(vendor_dir: &'static str, port: &'static str) -> Result<(), anyhow::Error> {
    SETUP.call_once(move || {
        use tracing_subscriber::fmt::Layer;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{filter::LevelFilter, EnvFilter};
        tracing_subscriber::registry()
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy()
                    .add_directive("hyper=warn".parse().unwrap())
                    .add_directive("reqwest=warn".parse().unwrap())
                    .add_directive("rustls=warn".parse().unwrap()),
            )
            .with(
                Layer::default()
                    .compact()
                    .with_file(true)
                    .with_line_number(true)
                    .with_ansi(false),
            )
            .init();

        let pip = create_python_venv().expect("Failed creating python virtual env");
        install_python_requirements(pip).expect("failed installing python requirements");
    });
    let python = env::temp_dir()
        .join(PYTHON_VENV_DIR)
        .join("bin")
        .join("python");
    let mut mockup_server = MockupServer {
        vendor_dir,
        port,
        python,
        process: None,
    };
    mockup_server.start()?; // stops on drop

    let endpoint = libredfish::Endpoint {
        host: format!("127.0.0.1:{port}"),
        ..Default::default()
    };

    let pool = libredfish::RedfishClientPool::builder().build()?;
    let redfish = pool.create_client(endpoint)?;

    if vendor_dir == "nvidia_dpu" {
        return nvidia_dpu_integration_test(redfish.as_ref());
    }

    assert_eq!(redfish.get_power_state()?, libredfish::PowerState::On);
    assert!(redfish.bios()?.len() > 10);

    redfish.power(libredfish::SystemPowerControl::GracefulShutdown)?;
    redfish.power(libredfish::SystemPowerControl::ForceOff)?;
    redfish.power(libredfish::SystemPowerControl::On)?;

    // A real BMC requires a reboot after every change, so pretend for accuracy.
    // Dell will 400 Bad Request if you make two consecutive changes.
    redfish.lockdown(libredfish::EnabledDisabled::Disabled)?;
    redfish.power(libredfish::SystemPowerControl::ForceRestart)?;
    if vendor_dir == "dell" {
        // we're testing against static files, so these don't change
        assert!(redfish.lockdown_status()?.is_fully_disabled());
    }

    redfish.setup_serial_console()?;
    redfish.power(libredfish::SystemPowerControl::ForceRestart)?;
    assert!(redfish.serial_console_status()?.is_fully_enabled());

    redfish.clear_tpm()?;
    // The mockup includes TPM clear pending operation
    assert!(!redfish.pending()?.is_empty());
    redfish.power(libredfish::SystemPowerControl::ForceRestart)?;

    redfish.boot_once(libredfish::Boot::Pxe)?;
    redfish.boot_first(libredfish::Boot::HardDisk)?;
    redfish.power(libredfish::SystemPowerControl::ForceRestart)?;

    redfish.lockdown(libredfish::EnabledDisabled::Enabled)?;
    redfish.power(libredfish::SystemPowerControl::GracefulRestart)?;
    if vendor_dir == "lenovo" {
        assert!(redfish.lockdown_status()?.is_fully_enabled());
    }
    _ = redfish.get_thermal_metrics()?;
    _ = redfish.get_power_metrics()?;
    if vendor_dir != "lenovo" {
        // the lenovo mockup doesn't have this content, but their docs have it
        _ = redfish.get_system_event_log()?;
    }

    Ok(())
}

/// Create a python virtualenv to install our requirements into.
/// Return the path of pip
fn create_python_venv() -> Result<PathBuf, anyhow::Error> {
    let venv_dir = env::temp_dir().join(PYTHON_VENV_DIR);
    let venv_out = Command::new("python3")
        .arg("-m")
        .arg("venv")
        .arg(&venv_dir)
        .output()
        .context("Is 'python3' on your $PATH?")?;
    if !venv_out.status.success() {
        eprintln!("*** Python virtual env creation failed:");
        eprintln!("\tSTDOUT: {}", String::from_utf8_lossy(&venv_out.stdout));
        eprintln!("\tSTDERR: {}", String::from_utf8_lossy(&venv_out.stderr));
        return Err(anyhow!(
            "Failed running 'python3 -m venv {}. Exit code {}",
            venv_dir.display(),
            venv_out.status.code().unwrap_or(-1),
        ));
    }

    Ok(venv_dir.join("bin/pip"))
}

fn install_python_requirements(pip: PathBuf) -> Result<(), anyhow::Error> {
    let req_path = PathBuf::from(ROOT_DIR)
        .join("tests")
        .join("requirements.txt");
    let output = Command::new(&pip)
        .arg("install")
        .arg("-q")
        .arg("--requirement")
        .arg(&req_path)
        .output()?;
    if !output.status.success() {
        eprintln!("*** pip install failed:");
        eprintln!("\tSTDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("\tSTDERR: {}", String::from_utf8_lossy(&output.stderr));
        return Err(anyhow!(
            "Failed running '{} install -q --requirement {}. Exit code {}",
            pip.display(),
            req_path.display(),
            output.status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}

struct MockupServer {
    vendor_dir: &'static str,
    port: &'static str,
    python: PathBuf,

    process: Option<Child>,
}

impl Drop for MockupServer {
    fn drop(&mut self) {
        if self.process.is_none() {
            return;
        }
        self.process.take().unwrap().kill().unwrap();
        sleep(Duration::from_secs(1)); // let it stop
    }
}

impl MockupServer {
    fn start(&mut self) -> std::io::Result<()> {
        // For extra debugging edit redfishMockupServer.py change the log level at the top
        self.process = Some(
            Command::new(&self.python)
                .current_dir(PathBuf::from(ROOT_DIR).join("tests"))
                .arg("redfishMockupServer.py")
                .arg("--port")
                .arg(self.port)
                .arg("--dir")
                .arg(format!("mockups/{}/", self.vendor_dir))
                .arg("--ssl")
                .arg("--cert")
                .arg("cert.pem")
                .arg("--key")
                .arg("key.pem")
                .spawn()?,
        );
        sleep(Duration::from_secs(1)); // let it start
        Ok(())
    }
}
