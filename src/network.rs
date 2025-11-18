use crate::models::{NetworkDevice, NetworkInfo};
use crate::scanner::NetworkScanner as PnetScanner;
use std::sync::{Arc, Mutex};
use pnet::datalink::NetworkInterface;

pub struct NetworkScanner {
    pub devices: Arc<Mutex<Vec<NetworkDevice>>>,
    pub network_info: Arc<Mutex<NetworkInfo>>,
    pub scanning: Arc<Mutex<bool>>,
    pub interface: Arc<Mutex<Option<NetworkInterface>>>,
}

impl NetworkScanner {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(Mutex::new(Vec::new())),
            network_info: Arc::new(Mutex::new(NetworkInfo::default())),
            scanning: Arc::new(Mutex::new(false)),
            interface: Arc::new(Mutex::new(None)),
        }
    }

    pub fn scan_network(&self) {
        let devices = Arc::clone(&self.devices);
        let network_info = Arc::clone(&self.network_info);
        let scanning = Arc::clone(&self.scanning);
        let interface = Arc::clone(&self.interface);

        crate::TOKIO_RUNTIME.spawn(async move {
            {
                let mut scan_flag = scanning.lock().unwrap();
                *scan_flag = true;
            }

            // Clear previous results
            {
                let mut devices_lock = devices.lock().unwrap();
                devices_lock.clear();
            }

            let interface = interface.lock().unwrap().clone();
            if interface.is_none() {
                eprintln!("Scan error: No network interface selected");
                return;
            }

            match PnetScanner::run_arp_scan(interface.unwrap()).await {
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
        let interface = self.interface.lock().unwrap().clone();
        if let Some(interface) = interface {
            PnetScanner::get_local_network_info(interface)
        } else {
            Err(anyhow::anyhow!("No network interface selected"))
        }
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
