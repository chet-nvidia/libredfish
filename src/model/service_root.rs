use std::collections::HashMap;
use std::fmt;

use model::{OData, ODataId};
use serde::{Deserialize, Serialize};

use crate::model;

/// https://redfish.dmtf.org/schemas/v1/ServiceRoot.v1_16_0.json
/// This type shall contain information about deep operations that the service supports.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRoot {
    #[serde(flatten)]
    pub odata: OData,
    pub product: Option<String>,
    pub redfish_version: String,
    pub vendor: Option<String>,
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
    pub oem: Option<HashMap<String, serde_json::Value>>,
    pub update_service: Option<HashMap<String, serde_json::Value>>,
    pub account_service: Option<ODataId>,
    pub certificate_service: Option<ODataId>,
    pub chassis: Option<ODataId>,
    pub event_service: Option<ODataId>,
    pub license_service: Option<ODataId>,
    pub fabrics: Option<ODataId>,
    pub managers: Option<ODataId>,
    pub session_service: Option<ODataId>,
    pub systems: Option<ODataId>,
    pub tasks: Option<ODataId>,
    pub telemetry_service: Option<ODataId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RedfishVendor {
    Lenovo,
    Dell,
    NvidiaDpu,
    Supermicro,
    AMI, // Viking
    Hpe,
    NvidiaGH200,
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
        Some(match v.to_lowercase().as_str() {
            "ami" => RedfishVendor::AMI,
            "dell" => RedfishVendor::Dell,
            "hpe" => RedfishVendor::Hpe,
            "lenovo" => RedfishVendor::Lenovo,
            "nvidia" => match self.product.as_deref() {
                Some("P3809") => RedfishVendor::NvidiaGH200,
                _ => RedfishVendor::NvidiaDpu,
            },
            "supermicro" => RedfishVendor::Supermicro,
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
