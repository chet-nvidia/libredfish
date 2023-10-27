use serde::{Deserialize, Serialize};

// A SerialInterface for Serial-Over-LAN. Machines seem to have always exactly one.
// Comment examples are for Supermicro. Lenovo is simliar. Dell has only name, hence all Options.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct SerialInterface {
    pub name: String,
    interface_enabled: Option<bool>,
    signal_type: Option<String>,    // Rs232", Rs485
    bit_rate: Option<String>,       // 1200 through 230400
    parity: Option<String>,         // None, Even, Odd, Mark, Space
    data_bits: Option<String>,      // 5-8 as a string
    stop_bits: Option<String>,      // "1" or "2"
    flow_control: Option<String>,   // None, Software, Hardware
    connector_type: Option<String>, // "RJ45", RJ11, "DB9 Female", "DB9 Male", "DB25 Female", "DB25 Male", USB, mUSB, uUSB
    pin_out: Option<String>,        // Cyclades, Cisco, Digi
}

impl SerialInterface {
    /// Is this serial interface set to the Supermicro defaults, which is also
    /// what we need for Serial Over LAN to work?
    ///
    /// Note that we don't seem to be able to change these via Redfish.
    /// A PATCH to redfish/v1/Managers/1/SerialInterfaces/1 produces a mixture of
    /// - .. is not in the list of valid properties for the resource
    /// - .. is a read only property and cannot be assigned a value
    pub fn is_supermicro_default(&self) -> bool {
        self.interface_enabled == Some(true)
            && self.signal_type.as_deref() == Some("Rs232")
            && self.bit_rate.as_deref() == Some("115200")
            && self.parity.as_deref() == Some("None")
            && self.data_bits.as_deref() == Some("8")
            && self.stop_bits.as_deref() == Some("1")
            && self.flow_control.as_deref() == Some("None")
            && self.connector_type.as_deref() == Some("RJ45")
            && self.pin_out.as_deref() == Some("Cyclades")
    }
}
