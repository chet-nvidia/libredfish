use serde::{Deserialize, Serialize};

use super::{LinkStatus, ODataId, ODataLinks, ResourceStatus};

/// http://redfish.dmtf.org/schemas/v1/EthernetInterface.v1_6_0.json
/// The EthernetInterface schema contains an inventory of Ethernet interface components.
/// This can include Network Device parameters such as current IP addresses, MAC address, link status, etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct EthernetInterface {
    #[serde(flatten)]
    pub odata: Option<ODataLinks>,
    #[serde(rename = "DHCPv4")]
    pub dhcpv4: Option<DHCPv4>,
    #[serde(rename = "DHCPv6")]
    pub dhcpv6: Option<DHCPv6>,
    pub description: Option<String>,
    #[serde(rename = "FQDN")]
    pub fqdn: Option<String>,
    pub host_name: Option<String>,
    #[serde(default, rename = "IPv4Addresses")]
    pub ipv4_addresses: Vec<IPv4Address>,
    #[serde(rename = "IPv4StaticAddresses", default)]
    pub ipv4_static_addresses: Vec<IPv4Address>,
    #[serde(default, rename = "IPv6AddressPolicyTable")]
    pub ipv6_address_policy_table: Vec<PolicyTable>,
    #[serde(rename = "IPv6Addresses", default)]
    pub ipv6_addresses: Vec<IPv6Address>,
    #[serde(rename = "IPv6DefaultGateway")]
    pub ipv6_default_gateway: Option<String>,
    #[serde(rename = "IPv6StaticAddresses", default)]
    pub ipv6_static_addresses: Vec<IPv6Address>,
    pub id: Option<String>,
    pub interface_enabled: Option<bool>,
    pub link_status: Option<LinkStatus>,
    #[serde(rename = "MACAddress")]
    pub mac_address: Option<String>,
    #[serde(rename = "MTUSize")]
    pub mtu_size: Option<i32>,
    pub name: Option<String>,
    #[serde(default)]
    pub name_servers: Vec<String>,
    pub speed_mbps: Option<i32>,
    #[serde(default)]
    pub static_name_servers: Vec<ODataId>,
    pub status: Option<ResourceStatus>,
    #[serde(rename = "VLANs")]
    pub vlans: Option<ODataId>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum IPv4AddressOrigin {
    Static,
    DHCP,
    BOOTP,
    IPv4LinkLocal,
}

impl std::fmt::Display for IPv4AddressOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub enum IPv6AddressOrigin {
    Static,
    DHCPv6,
    LinkLocal,
    SLAAC,
}

impl std::fmt::Display for IPv6AddressOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// http://redfish.dmtf.org/schemas/v1/IPAddresses.v1_0_10.json
/// The IPAddresses schema contains an inventory of IP Address components.
/// This can include IP Address parameters such as IP address, address origin, etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct IPv4Address {
    #[serde(flatten)]
    pub address: Option<String>,
    pub address_origin: Option<IPv4AddressOrigin>,
    pub gateway: Option<String>,
    pub subnet_mask: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct IPv6Address {
    #[serde(flatten)]
    pub address: Option<String>,
    pub address_origin: Option<IPv6AddressOrigin>,
    pub address_state: Option<String>,
    pub prefix_length: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DHCPv4 {
    #[serde(flatten)]
    #[serde(rename = "DHCPEnabled")]
    pub dhcp_enabled: Option<bool>,
    #[serde(rename = "UseDNSServers")]
    pub use_dns_servers: Option<bool>,
    pub use_domain_name: Option<bool>,
    #[serde(rename = "UseNTPServers")]
    pub use_ntp_servers: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct DHCPv6 {
    #[serde(flatten)]
    pub operating_mode: Option<String>,
    #[serde(rename = "UseDNSServers")]
    pub use_dns_servers: Option<bool>,
    pub use_domain_name: Option<bool>,
    #[serde(rename = "UseNTPServers")]
    pub use_ntp_servers: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PolicyTable {
    prefix: Option<String>,
    precedence: Option<i32>,
    label: Option<i32>,
}
