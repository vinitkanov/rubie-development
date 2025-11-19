use serde::Deserialize;

// Enum to represent the status of a device
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum DeviceStatus {
    Active,
    Inactive,
    Blocked,
    Unknown,
}

use std::time::Instant;

// Struct to hold information about a network device
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkDevice {
    pub ip_address: String,
    pub hostname: String,
    pub mac_address: String,
    pub vendor: String,
    pub status: DeviceStatus,
    #[serde(skip)]
    pub last_arp_time: Option<Instant>,
    #[serde(skip)]
    pub selected: bool,
    #[serde(skip)]
    pub is_killed: bool,
}
