use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// https://redfish.dmtf.org/schemas/v1/ServiceRoot.v1_16_0.json
/// This type shall contain information about deep operations that the service supports.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRoot {
    pub product: Option<String>,
    pub redfish_version: String,
    pub vendor: Option<String>,
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
    pub oem: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RedfishVendor {
    Lenovo,
    Dell,
    Nvidia,
    Supermicro,
    AMI,
    Hpe,
    Unknown,
}

impl fmt::Display for RedfishVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl ServiceRoot {
    /// Vendor provided by Redfish ServiceRoot
    pub fn vendor_string(&self) -> Option<String> {
        // If there is no "Vendor" key in ServiceRoot, look for an "Oem" entry. It will have a
        // single key which is the vendor name.
        self.vendor.as_ref().cloned().or_else(|| match &self.oem {
            Some(oem) => oem.keys().next().cloned(),
            None => None,
        })
    }

    pub fn vendor(&self) -> Option<RedfishVendor> {
        let v = self.vendor_string()?;
        Some(match v.as_str() {
            "AMI" => RedfishVendor::AMI,
            "Dell" => RedfishVendor::Dell,
            "HPE" => RedfishVendor::Hpe,
            "Lenovo" => RedfishVendor::Lenovo,
            "Nvidia" => RedfishVendor::Nvidia,
            "Supermicro" => RedfishVendor::Supermicro,
            _ => RedfishVendor::Unknown,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::model::service_root::RedfishVendor;

    #[test]
    fn test_supermicro_service_root() {
        let data = include_str!("testdata/supermicro_service_root.json");
        let result: super::ServiceRoot = serde_json::from_str(data).unwrap();
        assert_eq!(result.vendor().unwrap(), RedfishVendor::Supermicro);
    }
}
