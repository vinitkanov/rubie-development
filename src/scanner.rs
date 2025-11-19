use crate::models::{DeviceStatus, NetworkDevice};
use anyhow::Result;
use dashmap::DashMap;
use ipnetwork::IpNetwork;
use pnet::datalink::{self, Channel, MacAddr, NetworkInterface};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{echo_request, IcmpTypes, MutableIcmpPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{self, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::tcp::{self, MutableTcpPacket, TcpFlags};
use pnet::packet::Packet;
use rand::random;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

pub enum ScanCommand {
    Scan,
}

pub struct NetworkScanner {
    interface: NetworkInterface,
    devices: Arc<DashMap<String, NetworkDevice>>,
    sender: mpsc::UnboundedSender<NetworkDevice>,
    command_receiver: mpsc::UnboundedReceiver<ScanCommand>,
    warning_sender: mpsc::UnboundedSender<String>,
}

impl NetworkScanner {
    pub fn new(
        interface: NetworkInterface,
        devices: Arc<DashMap<String, NetworkDevice>>,
        sender: mpsc::UnboundedSender<NetworkDevice>,
        command_receiver: mpsc::UnboundedReceiver<ScanCommand>,
        warning_sender: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            interface,
            devices,
            sender,
            command_receiver,
            warning_sender,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        println!("[Scanner] Starting scanner");
        let (mut tx, mut rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(anyhow::anyhow!("Unsupported channel type")),
            Err(e) => return Err(anyhow::anyhow!("Failed to create channel: {}", e)),
        };

        let devices = self.devices.clone();
        let sender = self.sender.clone();

        // ARP listener task
        tokio::spawn(async move {
            loop {
                Self::on_packet_arrival(&mut rx, &devices, &sender).await;
            }
        });

        // Background scanning task
        let devices = self.devices.clone();
        tokio::spawn(async move {
            Self::start_background_scan(devices).await;
        });

        // Initial ARP probe
        self.probe_devices(&mut tx).await?;

        // Proxy ARP detection
        let mut mac_to_ips: std::collections::HashMap<MacAddr, Vec<Ipv4Addr>> = std::collections::HashMap::new();
        for entry in self.devices.iter() {
            let device = entry.value();
            if let Ok(ip) = device.ip_address.parse::<Ipv4Addr>() {
                if let Ok(mac) = device.mac_address.parse::<MacAddr>() {
                    mac_to_ips.entry(mac).or_default().push(ip);
                }
            }
        }

        if let Some(gateway) = default_net::get_default_gateway().ok() {
            let router_mac_bytes = gateway.mac_addr.octets();
            let router_mac = MacAddr::new(
                router_mac_bytes[0],
                router_mac_bytes[1],
                router_mac_bytes[2],
                router_mac_bytes[3],
                router_mac_bytes[4],
                router_mac_bytes[5],
            );
            if mac_to_ips.contains_key(&router_mac) {
                let _ = self.warning_sender.send(
                    "Proxy ARP detected! Your router is responding for all devices. \
                    For genuine MAC addresses, please disable Proxy ARP on your MikroTik router."
                        .to_string(),
                );
            }
        }

        loop {
            if let Some(command) = self.command_receiver.recv().await {
                match command {
                    ScanCommand::Scan => {
                        self.probe_devices(&mut tx).await?;
                    }
                }
            }
        }
    }

    async fn probe_devices(&self, tx: &mut Box<dyn datalink::DataLinkSender>) -> Result<()> {
        println!("[Scanner] Probing devices");
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

        println!("[Scanner] Iterating through network to send ARP requests");
        for ip in network_iter {
            if ip == source_ip {
                continue;
            }
            Self::send_arp_request(&mut **tx, &self.interface, source_ip, ip)?;
            Self::send_icmp_echo_request(&mut **tx, &self.interface, source_ip, ip)?;
            let common_ports = vec![22, 80, 443, 3389, 8080];
            for port in common_ports {
                Self::send_tcp_syn_packet(&mut **tx, &self.interface, source_ip, ip, port)?;
            }
        }

        Ok(())
    }

    fn create_ipv4_packet(
        source_ip: Ipv4Addr,
        destination_ip: Ipv4Addr,
        protocol: pnet::packet::ip::IpNextHeaderProtocol,
        payload_size: usize,
    ) -> Result<MutableIpv4Packet<'static>> {
        let buffer = vec![0u8; 20 + payload_size];
        let mut ipv4_packet = MutableIpv4Packet::owned(buffer).unwrap();

        ipv4_packet.set_version(4);
        ipv4_packet.set_header_length(5);
        ipv4_packet.set_total_length((20 + payload_size) as u16);
        ipv4_packet.set_ttl(64);
        ipv4_packet.set_next_level_protocol(protocol);
        ipv4_packet.set_source(source_ip);
        ipv4_packet.set_destination(destination_ip);
        let checksum = ipv4::checksum(&ipv4_packet.to_immutable());
        ipv4_packet.set_checksum(checksum);

        Ok(ipv4_packet)
    }

    fn send_icmp_echo_request(
        tx: &mut dyn datalink::DataLinkSender,
        interface: &NetworkInterface,
        source_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) -> Result<()> {
        let source_mac = interface.mac.unwrap();

        let mut icmp_buffer = [0u8; 8];
        let mut icmp_packet = MutableIcmpPacket::new(&mut icmp_buffer).unwrap();
        icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
        icmp_packet.set_icmp_code(echo_request::IcmpCodes::NoCode);
        let checksum = pnet::packet::util::checksum(icmp_packet.packet(), 1);
        icmp_packet.set_checksum(checksum);

        let mut ipv4_packet =
            Self::create_ipv4_packet(source_ip, target_ip, IpNextHeaderProtocols::Icmp, 8)?;
        ipv4_packet.set_payload(icmp_packet.packet());

        let mut ethernet_buffer = [0u8; 14 + 20 + 8];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(source_mac);
        ethernet_packet.set_ethertype(EtherTypes::Ipv4);
        ethernet_packet.set_payload(ipv4_packet.packet());

        match tx.send_to(ethernet_packet.packet(), None) {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(e.into()),
            None => Err(anyhow::anyhow!("Failed to send packet")),
        }
    }

    fn send_tcp_syn_packet(
        tx: &mut dyn datalink::DataLinkSender,
        interface: &NetworkInterface,
        source_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
        target_port: u16,
    ) -> Result<()> {
        let source_mac = interface.mac.unwrap();

        let mut tcp_buffer = [0u8; 20];
        let mut tcp_packet = MutableTcpPacket::new(&mut tcp_buffer).unwrap();

        tcp_packet.set_source(random::<u16>());
        tcp_packet.set_destination(target_port);
        tcp_packet.set_sequence(random::<u32>());
        tcp_packet.set_acknowledgement(0);
        tcp_packet.set_data_offset(5);
        tcp_packet.set_flags(TcpFlags::SYN);
        tcp_packet.set_window(65535);
        let checksum = tcp::ipv4_checksum(&tcp_packet.to_immutable(), &source_ip, &target_ip);
        tcp_packet.set_checksum(checksum);

        let mut ipv4_packet =
            Self::create_ipv4_packet(source_ip, target_ip, IpNextHeaderProtocols::Tcp, 20)?;
        ipv4_packet.set_payload(tcp_packet.packet());

        let mut ethernet_buffer = [0u8; 14 + 20 + 20];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(source_mac);
        ethernet_packet.set_ethertype(EtherTypes::Ipv4);
        ethernet_packet.set_payload(ipv4_packet.packet());

        match tx.send_to(ethernet_packet.packet(), None) {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(e.into()),
            None => Err(anyhow::anyhow!("Failed to send packet")),
        }
    }

    fn send_arp_request(
        tx: &mut dyn datalink::DataLinkSender,
        interface: &NetworkInterface,
        source_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) -> Result<()> {
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

        match tx.send_to(ethernet_packet.packet(), None) {
            Some(Ok(_)) => Ok(()),
            Some(Err(e)) => Err(e.into()),
            None => Err(anyhow::anyhow!("Failed to send packet: the network interface may not be available")),
        }
    }

    async fn on_packet_arrival(
        rx: &mut Box<dyn datalink::DataLinkReceiver>,
        devices: &Arc<DashMap<String, NetworkDevice>>,
        sender: &mpsc::UnboundedSender<NetworkDevice>,
    ) {
        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet_packet) = EthernetPacket::new(packet) {
                    let source_mac = ethernet_packet.get_source();
                    let source_ip = match ethernet_packet.get_ethertype() {
                        EtherTypes::Ipv4 => Ipv4Packet::new(ethernet_packet.payload())
                            .map(|p| IpAddr::V4(p.get_source())),
                        EtherTypes::Arp => ArpPacket::new(ethernet_packet.payload())
                            .map(|p| IpAddr::V4(p.get_sender_proto_addr())),
                        _ => None,
                    };

                    if let Some(ip) = source_ip {
                        let mac_address = source_mac.to_string();
                        let ip_address = ip.to_string();

                        if let Some(mut device) = devices.get_mut(&mac_address) {
                            device.last_arp_time = Some(Instant::now());
                            device.status = DeviceStatus::Active;
                            device.ip_address = ip_address;
                        } else {
                            let device = NetworkDevice {
                                ip_address,
                                mac_address: mac_address.clone(),
                                hostname: "".to_string(),
                                vendor: "".to_string(),
                                status: DeviceStatus::Active,
                                last_arp_time: Some(Instant::now()),
                                selected: false,
                                is_killed: false,
                            };
                            devices.insert(mac_address, device.clone());
                            if let Err(e) = sender.send(device) {
                                eprintln!("Failed to send device to UI: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving packet: {}", e);
            }
        }
    }

    async fn start_background_scan(devices: Arc<DashMap<String, NetworkDevice>>) {
        let mut is_alive_interval = time::interval(Duration::from_secs(30));

        loop {
            is_alive_interval.tick().await;
            for mut item in devices.iter_mut() {
                let device = item.value_mut();
                if let Some(last_arp_time) = device.last_arp_time {
                    if last_arp_time.elapsed() > Duration::from_secs(60) {
                        device.status = DeviceStatus::Inactive;
                    }
                }
            }
        }
    }
}
