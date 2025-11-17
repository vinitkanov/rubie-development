use crate::models::{NetworkDevice, NetworkInfo};
use crate::scanner::NetworkScanner as PnetScanner;
use std::sync::{Arc, Mutex};

pub struct NetworkScanner {
    pub devices: Arc<Mutex<Vec<NetworkDevice>>>,
    pub network_info: Arc<Mutex<NetworkInfo>>,
    pub scanning: Arc<Mutex<bool>>,
}

impl NetworkScanner {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(Vec::new())),
            network_info: Arc::new(Mutex::new(NetworkInfo::default())),
            scanning: Arc::new(Mutex::new(false)),
        }
    }

    pub fn scan_network(&self) {
        let devices = Arc::clone(&self.devices);
        let network_info = Arc::clone(&self.network_info);
        let scanning = Arc::clone(&self.scanning);

        tokio::spawn(async move {
            {
                let mut scan_flag = scanning.lock().unwrap();
                *scan_flag = true;
            }

            // Clear previous results
            {
                let mut devices_lock = devices.lock().unwrap();
                devices_lock.clear();
            }

            match PnetScanner::run_arp_scan().await {
                Ok(scanned_devices) => {
                    let mut unique_devices = Vec::new();
                    let mut mac_addresses = std::collections::HashSet::new();

                    for device in scanned_devices {
                        if mac_addresses.insert(device.mac_address.clone()) {
                            unique_devices.push(device);
                        }
                    }

                    let mut devices_lock = devices.lock().unwrap();
                    *devices_lock = unique_devices;

                    let mut info = network_info.lock().unwrap();
                    info.active_devices = devices_lock.len();
                }
                Err(e) => {
                    eprintln!("Scan error: {}", e);
                    let mut devices_lock = devices.lock().unwrap();
                    devices_lock.clear();
                }
            }

            {
                let mut scan_flag = scanning.lock().unwrap();
                *scan_flag = false;
            }
        });
    }

    pub fn get_local_network_info(&self) -> anyhow::Result<NetworkInfo> {
        PnetScanner::get_local_network_info()
    }

    pub fn kill_selected_devices(&self) {
        let devices = Arc::clone(&self.devices);
        crate::disconnect::kill_selected_devices(&devices);
    }

    pub fn restore_selected_devices(&self) {
        let devices = Arc::clone(&self.devices);
        crate::restore::restore_selected_devices(&devices);
    }

    pub fn kill_all_devices(&self) {
        let devices = Arc::clone(&self.devices);
        crate::disconnect::kill_all_devices(&devices);
    }

    pub fn restore_all_devices(&self) {
        let devices = Arc::clone(&self.devices);
        crate::restore::restore_all_devices(&devices);
    }
}
