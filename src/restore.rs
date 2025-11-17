use crate::models::{DeviceStatus, NetworkDevice};
use std::sync::{Arc, Mutex};

/// This module contains dummy functions for "restoring" devices.
/// It only changes the local status in the app.
/// It does NOT perform any real network operations.

pub fn restore_selected_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        if device.selected {
            device.status = DeviceStatus::Active;
            println!(
                "Set {} to 'Active' (local status only).",
                device.ip_address
            );
        }
    }
}

pub fn restore_all_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        device.status = DeviceStatus::Active;
    }
    println!("Set all devices to 'Active' (local status only).");
}
