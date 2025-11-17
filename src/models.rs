use serde::Deserialize;

// Enum to represent the status of a device
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum DeviceStatus {
    Active,
    Inactive,
    Blocked,
    Unknown,
}

impl DeviceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceStatus::Active => "Active",
            DeviceStatus::Inactive => "Inactive",
            DeviceStatus::Blocked => "Blocked",
            DeviceStatus::Unknown => "Unknown",
        }
    }
}

// Struct to hold information about a network device
#[derive(Debug, Clone, Deserialize)]
pub struct NetworkDevice {
    pub ip_address: String,
    #[serde(default = "default_string")]
    pub hostname: String,
    #[serde(default = "default_string")]
    pub mac_address: String,
    #[serde(default = "default_string")]
    pub vendor: String,
    pub status: DeviceStatus,
    pub response_time: f64, // in milliseconds
    #[serde(skip)]
    pub selected: bool,
}


// Struct to hold information about the local network
#[derive(Debug, Clone, Default)]
pub struct NetworkInfo {
    pub network_range: String,
    pub gateway: String,
    pub active_devices: usize,
}

// Helper for serde defaults
fn default_string() -> String {
    "Unknown".to_string()
}
