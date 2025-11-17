use crate::models::{DeviceStatus, NetworkDevice, NetworkInfo};
use anyhow::Result;
use ipnetwork::IpNetwork;
use pnet::datalink::{self, Channel, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::Packet;
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Clone, Default)]
pub struct NetworkScanner;

impl NetworkScanner {


    pub async fn run_arp_scan() -> Result<Vec<NetworkDevice>> {
        let interface = Self::get_default_interface()?;
        let source_ip = interface
            .ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .map(|ip| match ip.ip() {
                IpAddr::V4(ip) => ip,
                _ => unreachable!(),
            })
            .ok_or_else(|| anyhow::anyhow!("No IPv4 address found"))?;

        let network = interface
            .ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .ok_or_else(|| anyhow::anyhow!("No IPv4 network found"))?;

        let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(anyhow::anyhow!("Unsupported channel type")),
            Err(e) => return Err(anyhow::anyhow!("Failed to create channel: {}", e)),
        };

        let (device_sender, mut device_receiver) = mpsc::unbounded_channel();

        // ARP listener task
        tokio::spawn(async move {
            loop {
                match rx.next() {
                    Ok(packet) => {
                        if let Some(ethernet_packet) = EthernetPacket::new(packet) {
                            if ethernet_packet.get_ethertype() == EtherTypes::Arp {
                                if let Some(arp_packet) = ArpPacket::new(ethernet_packet.payload())
                                {
                                    if arp_packet.get_operation() == ArpOperations::Reply {
                                        let device = NetworkDevice {
                                            ip_address: arp_packet
                                                .get_sender_proto_addr()
                                                .to_string(),
                                            mac_address: arp_packet
                                                .get_sender_hw_addr()
                                                .to_string(),
                                            hostname: "".to_string(), // ARP doesn't provide hostname
                                            vendor: "".to_string(), // Vendor lookup can be added later
                                            status: DeviceStatus::Active,
                                            response_time: 0.0, // RTT is not measured in ARP scan
                                            selected: false,
                                        };
                                        if device_sender.send(device).is_err() {
                                            // Receiver dropped
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving packet: {}", e);
                        break;
                    }
                }
            }
        });

        // ARP sender task
        let network_iter = match network {
            IpNetwork::V4(net) => net.iter(),
            _ => return Err(anyhow::anyhow!("Only IPv4 networks are supported")),
        };
        tokio::spawn(async move {
            for ip in network_iter {
                if ip == source_ip {
                    continue;
                }
                Self::send_arp_request(&mut *tx, &interface, source_ip, ip);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        let mut devices = Vec::new();
        let scan_duration = Duration::from_secs(5);
        let start_time = Instant::now();

        while start_time.elapsed() < scan_duration {
            if let Ok(Some(device)) =
                tokio::time::timeout(Duration::from_millis(100), device_receiver.recv()).await
            {
                if !devices
                    .iter()
                    .any(|dev: &NetworkDevice| dev.ip_address == device.ip_address)
                {
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    fn send_arp_request(
        tx: &mut dyn datalink::DataLinkSender,
        interface: &NetworkInterface,
        source_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) {
        let source_mac = interface.mac.unwrap();

        let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(source_mac);
        ethernet_packet.set_ethertype(EtherTypes::Arp);

        let mut arp_buffer = [0u8; 28];
        let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

        arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp_packet.set_protocol_type(EtherTypes::Ipv4);
        arp_packet.set_hw_addr_len(6);
        arp_packet.set_proto_addr_len(4);
        arp_packet.set_operation(ArpOperations::Request);
        arp_packet.set_sender_hw_addr(source_mac);
        arp_packet.set_sender_proto_addr(source_ip);
        arp_packet.set_target_hw_addr(MacAddr::zero());
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

    pub fn get_local_network_info() -> Result<NetworkInfo> {
        let default_gateway = default_net::get_default_gateway()
            .map_err(|e| anyhow::anyhow!("Failed to get default gateway: {}", e))?;
        let default_interface = Self::get_default_interface()?;

        let network = default_interface
            .ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .ok_or_else(|| anyhow::anyhow!("No IPv4 network found"))?;

        let mut network_info = NetworkInfo::default();
        network_info.network_range = network.to_string();
        network_info.gateway = default_gateway.ip_addr.to_string();

        Ok(network_info)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_local_network_info() {
        let result = NetworkScanner::get_local_network_info();
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.network_range.is_empty());
        assert!(!info.gateway.is_empty());
    }
}
