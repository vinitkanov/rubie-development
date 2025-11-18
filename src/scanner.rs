use crate::models::{DeviceStatus, NetworkDevice};
use anyhow::Result;
use ipnetwork::IpNetwork;
use pnet::datalink::{self, Channel, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::Packet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

pub struct NetworkScanner {
    interface: NetworkInterface,
    devices: Arc<Mutex<Vec<NetworkDevice>>>,
    sender: mpsc::UnboundedSender<NetworkDevice>,
}

impl NetworkScanner {
    pub fn new(
        interface: NetworkInterface,
        sender: mpsc::UnboundedSender<NetworkDevice>,
    ) -> Self {
        Self {
            interface,
            devices: Arc::new(Mutex::new(Vec::new())),
            sender,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let (mut tx, mut rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(anyhow::anyhow!("Unsupported channel type")),
            Err(e) => return Err(anyhow::anyhow!("Failed to create channel: {}", e)),
        };

        let devices = self.devices.clone();
        let sender = self.sender.clone();
        let _interface = self.interface.clone();

        // ARP listener task
        tokio::spawn(async move {
            loop {
                match Self::on_packet_arrival(&mut rx, &devices, &sender).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error receiving packet: {}", e);
                        break;
                    }
                }
            }
        });

        // Background scanning task
        let devices = self.devices.clone();
        tokio::spawn(async move {
            Self::start_background_scan(devices).await;
        });

        // Initial ARP probe
        self.probe_devices(&mut tx).await?;

        Ok(())
    }

    async fn probe_devices(&self, tx: &mut Box<dyn datalink::DataLinkSender>) -> Result<()> {
        let source_ip = self
            .interface
            .ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .map(|ip| match ip.ip() {
                IpAddr::V4(ip) => ip,
                _ => unreachable!(),
            })
            .ok_or_else(|| anyhow::anyhow!("No IPv4 address found"))?;

        let network = self
            .interface
            .ips
            .iter()
            .find(|ip| ip.is_ipv4())
            .ok_or_else(|| anyhow::anyhow!("No IPv4 network found"))?;

        let network_iter = match network {
            IpNetwork::V4(net) => net.iter(),
            _ => return Err(anyhow::anyhow!("Only IPv4 networks are supported")),
        };

        for ip in network_iter.step_by(256) {
            for i in 1..255 {
                let target_ip = Ipv4Addr::new(ip.octets()[0], ip.octets()[1], ip.octets()[2], i);
                if target_ip == source_ip {
                    continue;
                }

                Self::send_arp_request(&mut **tx, &self.interface, source_ip, target_ip);
            }
        }

        Ok(())
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

    async fn on_packet_arrival(
        rx: &mut Box<dyn datalink::DataLinkReceiver>,
        devices: &Arc<Mutex<Vec<NetworkDevice>>>,
        sender: &mpsc::UnboundedSender<NetworkDevice>,
    ) -> Result<()> {
        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet_packet) = EthernetPacket::new(packet) {
                    if ethernet_packet.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp_packet) = ArpPacket::new(ethernet_packet.payload()) {
                            if arp_packet.get_operation() == ArpOperations::Reply {
                                let mut devices = devices.lock().unwrap();
                                let ip_address = arp_packet.get_sender_proto_addr().to_string();
                                let mac_address = arp_packet.get_sender_hw_addr().to_string();

                                if let Some(device) =
                                    devices.iter_mut().find(|d| d.ip_address == ip_address)
                                {
                                    device.last_arp_time = Some(Instant::now());
                                    device.status = DeviceStatus::Active;
                                } else {
                                    let device = NetworkDevice {
                                        ip_address,
                                        mac_address,
                                        hostname: "".to_string(),
                                        vendor: "".to_string(),
                                        status: DeviceStatus::Active,
                                        last_arp_time: Some(Instant::now()),
                                        selected: false,
                                    };
                                    devices.push(device.clone());
                                    sender.send(device)?;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error receiving packet: {}", e));
            }
        }
        Ok(())
    }

    async fn start_background_scan(devices: Arc<Mutex<Vec<NetworkDevice>>>) {
        let mut interval = time::interval(Duration::from_secs(10));
        let mut is_alive_interval = time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Refresh ARP cache
                }
                _ = is_alive_interval.tick() => {
                    let mut devices = devices.lock().unwrap();
                    for device in devices.iter_mut() {
                        if let Some(last_arp_time) = device.last_arp_time {
                            if last_arp_time.elapsed() > Duration::from_secs(60) {
                                device.status = DeviceStatus::Inactive;
                            }
                        }
                    }
                }
            }
        }
    }
}
