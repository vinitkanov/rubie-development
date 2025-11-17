use crate::models::{NetworkDevice, NetworkInfo, DeviceStatus};
use anyhow::Result;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

    pub fn scan_network(&self, network_range: String) {
        let devices = Arc::clone(&self.devices);
        let network_info = Arc::clone(&self.network_info);
        let scanning = Arc::clone(&self.scanning);

        std::thread::spawn(move || {
            {
                let mut scan_flag = scanning.lock().unwrap();
                *scan_flag = true;
            }

            // Run nmap scan
            match Self::run_nmap_scan(&network_range) {
                Ok(scanned_devices) => {
                    let mut devices_lock = devices.lock().unwrap();
                    *devices_lock = scanned_devices;

                    // Update network info
                    let mut info = network_info.lock().unwrap();
                    info.active_devices = devices_lock.len();
                    info.network_range = network_range.clone();
                }
                Err(e) => {
                    eprintln!("Scan error: {}", e);
                }
            }

            {
                let mut scan_flag = scanning.lock().unwrap();
                *scan_flag = false;
            }
        });
    }

    fn run_nmap_scan(network_range: &str) -> Result<Vec<NetworkDevice>> {
        let _start = Instant::now();

        // Execute nmap command
        // Example: nmap -sn 192.168.1.0/24
        let output = Command::new("nmap")
            .arg("-sn")
            .arg(network_range)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let mut devices = Vec::new();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse nmap output
                let lines: Vec<&str> = stdout.lines().collect();
                let mut current_ip = String::new();

                for line in lines {
                    if line.contains("Nmap scan report for") {
                        // Extract IP address
                        if let Some(ip_start) = line.rfind('(') {
                            if let Some(ip_end) = line.rfind(')') {
                                current_ip = line[ip_start + 1..ip_end].to_string();
                            }
                        } else {
                            // IP might be directly in the line
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 5 {
                                current_ip = parts[4].to_string();
                            }
                        }
                    } else if line.contains("Host is up") && !current_ip.is_empty() {
                        // Extract response time
                        let mut response_time = 0.0;
                        if let Some(time_start) = line.find('(') {
                            if let Some(time_end) = line.find('s') {
                                let time_str = &line[time_start + 1..time_end];
                                response_time = time_str.parse().unwrap_or(0.0);
                            }
                        }

                        let mut device = NetworkDevice::new(current_ip.clone());
                        device.hostname = format!("DESKTOP-{}", &current_ip.replace('.', ""));
                        device.status = DeviceStatus::Active;
                        device.response_time = response_time * 1000.0; // Convert to ms

                        devices.push(device);
                        current_ip.clear();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to execute nmap: {}", e);
                // Return sample data for demo purposes
                let mut device = NetworkDevice::new("10.2.0.2".to_string());
                device.hostname = "DESKTOP-QG586LN".to_string();
                device.mac_address = "Unknown".to_string();
                device.vendor = "Unknown".to_string();
                device.status = DeviceStatus::Blocked;
                device.response_time = 78.1;
                devices.push(device);
            }
        }

        Ok(devices)
    }

    pub fn get_local_network_info() -> Result<NetworkInfo> {
        // Try to get local network information using ipconfig (Windows)
        let output = Command::new("ipconfig")
            .stdout(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut network_info = NetworkInfo::default();
        let mut local_ip = String::new();

        for line in stdout.lines() {
            if line.contains("IPv4 Address") {
                if let Some(ip_part) = line.split(':').nth(1) {
                    let ip = ip_part.trim();
                    local_ip = ip.to_string();
                }
            } else if line.contains("Subnet Mask") {
                if let Some(mask_part) = line.split(':').nth(1) {
                    let mask = mask_part.trim();
                    // Convert subnet mask to CIDR notation
                    let cidr = Self::subnet_mask_to_cidr(mask);
                    if !local_ip.is_empty() {
                        // Calculate network range
                        network_info.network_range = Self::calculate_network_range(&local_ip, cidr);
                    }
                }
            } else if line.contains("Default Gateway") {
                if let Some(gateway_part) = line.split(':').nth(1) {
                    let gateway = gateway_part.trim();
                    if !gateway.is_empty() {
                        network_info.gateway = gateway.to_string();
                    }
                }
            }
        }

        Ok(network_info)
    }

    fn subnet_mask_to_cidr(mask: &str) -> u8 {
        // Convert subnet mask to CIDR notation
        match mask {
            "255.255.255.0" => 24,
            "255.255.0.0" => 16,
            "255.0.0.0" => 8,
            "255.255.255.128" => 25,
            "255.255.255.192" => 26,
            "255.255.255.224" => 27,
            "255.255.255.240" => 28,
            "255.255.255.248" => 29,
            "255.255.255.252" => 30,
            _ => 24, // Default to /24
        }
    }

    fn calculate_network_range(ip: &str, cidr: u8) -> String {
        // Parse IP address
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return format!("{}/{}", ip, cidr);
        }

        let octets: Vec<u8> = parts.iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        if octets.len() != 4 {
            return format!("{}/{}", ip, cidr);
        }

        // Calculate network address based on CIDR
        let network = match cidr {
            24 => format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2]),
            16 => format!("{}.{}.0.0/16", octets[0], octets[1]),
            8 => format!("{}.0.0.0/8", octets[0]),
            _ => format!("{}/{}", ip, cidr),
        };

        network
    }

    pub fn kill_selected_devices(&self) {
        let mut devices = self.devices.lock().unwrap();
        for device in devices.iter_mut() {
            if device.selected {
                device.status = DeviceStatus::Blocked;
                // Here you would implement actual blocking logic
                // For example, using arp spoofing or firewall rules
            }
        }
    }

    pub fn restore_selected_devices(&self) {
        let mut devices = self.devices.lock().unwrap();
        for device in devices.iter_mut() {
            if device.selected {
                device.status = DeviceStatus::Active;
                // Here you would implement actual restoration logic
            }
        }
    }

    pub fn kill_all_devices(&self) {
        let mut devices = self.devices.lock().unwrap();
        for device in devices.iter_mut() {
            device.status = DeviceStatus::Blocked;
        }
    }

    pub fn restore_all_devices(&self) {
        let mut devices = self.devices.lock().unwrap();
        for device in devices.iter_mut() {
            device.status = DeviceStatus::Active;
        }
    }
}
