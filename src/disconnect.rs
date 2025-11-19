use crate::models::NetworkDevice;
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;

pub fn kill_selected_devices(devices: &Arc<DashMap<IpAddr, NetworkDevice>>) {
    for mut item in devices.iter_mut() {
        let device = item.value_mut();
        if device.selected {
            device.is_killed = true;
        }
    }
}
