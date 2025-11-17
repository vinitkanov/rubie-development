use crate::models::{DeviceStatus, NetworkDevice};
use std::sync::{Arc, Mutex};

/// This module contains dummy functions for "blocking" devices.
/// It only changes the local status in the app.
/// It does NOT perform any real network operations.

pub fn disconnect_selected_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        if device.selected {
            device.status = DeviceStatus::Blocked;
            println!(
                "Set {} to 'Blocked' (local status only).",
                device.ip_address
            );
        }
    }
}

pub fn disconnect_all_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        device.status = DeviceStatus::Blocked;
    }
    println!("Set all devices to 'Blocked' (local status only).");
}
