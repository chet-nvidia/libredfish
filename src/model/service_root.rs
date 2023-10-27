use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// https://redfish.dmtf.org/schemas/v1/ServiceRoot.v1_16_0.json
/// This type shall contain information about deep operations that the service supports.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceRoot {
    pub product: Option<String>,
    pub redfish_version: String,
    pub vendor: Option<String>,
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
    pub oem: Option<HashMap<String, serde_json::Value>>,
}

impl ServiceRoot {
    /// Vendor provided by Redfish ServiceRoot
    pub fn vendor(&self) -> Option<String> {
        // If there is no "Vendor" key in ServiceRoot, look for an "Oem" entry. It will have a
        // single key which is the vendor name.
        self.vendor.as_ref().cloned().or_else(|| match &self.oem {
            Some(oem) => oem.keys().next().cloned(),
            None => None,
        })
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_supermicro_service_root() {
        let data = include_str!("testdata/supermicro_service_root.json");
        let result: super::ServiceRoot = serde_json::from_str(data).unwrap();
        assert_eq!(result.vendor().unwrap(), "Supermicro");
    }
}
