use crate::models::NetworkDevice;
use dashmap::DashMap;
use std::sync::Arc;

pub fn restore_selected_devices(devices: &Arc<DashMap<String, NetworkDevice>>) {
    for mut item in devices.iter_mut() {
        let device = item.value_mut();
        if device.selected {
            device.is_killed = false;
        }
    }
}
