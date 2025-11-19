use crate::{
    disconnect::kill_selected_devices,
    interface_selector::InterfaceSelector,
    killer::Killer,
    models::{DeviceStatus, NetworkDevice},
    restore::restore_selected_devices,
    scanner::{NetworkScanner, ScanCommand},
    TOKIO_RUNTIME,
};
use dashmap::DashMap;
use eframe::egui;
use pnet::datalink::NetworkInterface;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct NetworkManagerApp {
    devices: Arc<DashMap<String, NetworkDevice>>,
    auto_refresh: bool,
    last_scan: Instant,
    interface_selector: InterfaceSelector,
    selected_interface: Arc<Mutex<Option<NetworkInterface>>>,
    device_receiver: mpsc::UnboundedReceiver<NetworkDevice>,
    command_sender: Option<mpsc::UnboundedSender<ScanCommand>>,
    error: Arc<Mutex<Option<String>>>,
}

impl NetworkManagerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (_device_sender, device_receiver) = mpsc::unbounded_channel();
        let devices = Arc::new(DashMap::new());
        let selected_interface = Arc::new(Mutex::new(None));
        let killer = Killer::new(devices.clone(), selected_interface.clone());

        let killer_clone = killer.clone();
        TOKIO_RUNTIME.spawn(async move {
            killer_clone.start().await;
        });

        Self {
            devices,
            auto_refresh: false,
            last_scan: Instant::now(),
            interface_selector: InterfaceSelector::new(),
            selected_interface,
            device_receiver,
            command_sender: None,
            error: Arc::new(Mutex::new(None)),
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.heading("Network Device Manager");
            ui.add_space(20.0);
            ui.label("Monitor and manage network devices");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::GREEN, "● Connected");
                ui.add_space(10.0);
                if ui.checkbox(&mut self.auto_refresh, "Auto-refresh").clicked() {
                    if self.auto_refresh {
                        self.last_scan = Instant::now();
                    }
                }
            });
        });
        ui.add_space(5.0);
        ui.separator();
    }

    fn render_info_panel(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(5.0);
            self.render_info_box(ui, "Network Range", "192.168.1.0/24", "🌐");
            ui.add_space(5.0);
            self.render_info_box(ui, "Gateway", "192.168.1.1", "🚪");
            ui.add_space(5.0);
            self.render_info_box(
                ui,
                "Active Devices",
                &self.devices.len().to_string(),
                "📊",
            );
        });
    }

    fn render_info_box(&self, ui: &mut egui::Ui, title: &str, value: &str, icon: &str) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(240, 240, 240))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
            .inner_margin(15.0)
            .show(ui, |ui| {
                ui.set_width(340.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(format!("{} {}", icon, title)).strong());
                    ui.label(
                        egui::RichText::new(value)
                            .size(16.0)
                            .color(egui::Color32::from_rgb(50, 50, 50)),
                    );
                });
            });
    }

    fn render_control_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(5.0);
            if ui
                .add_sized(
                    [140.0, 35.0],
                    egui::Button::new(egui::RichText::new("🔍 Scan Network").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(0, 120, 215)),
                )
                .clicked()
            {
                if let Some(sender) = &self.command_sender {
                    let _ = sender.send(ScanCommand::Scan);
                }
            }
            ui.add_space(5.0);
            if ui
                .add_sized(
                    [120.0, 35.0],
                    egui::Button::new(egui::RichText::new("🔄 Refresh").color(egui::Color32::BLACK))
                        .fill(egui::Color32::from_rgb(230, 230, 230)),
                )
                .clicked()
            {
                if let Some(sender) = &self.command_sender {
                    let _ = sender.send(ScanCommand::Scan);
                }
            }
            ui.add_space(100.0);
            let selected_count = self.devices.iter().filter(|d| d.selected).count();
            self.render_disconnect_button(ui, selected_count);
            ui.add_space(5.0);
            self.render_restore_button(ui, selected_count);
        });
    }

    fn render_disconnect_button(&mut self, ui: &mut egui::Ui, selected_count: usize) {
        if ui
            .add_sized(
                [200.0, 35.0],
                egui::Button::new(
                    egui::RichText::new(format!("✖ Disconnect Selected ({})", selected_count))
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(200, 50, 50)),
            )
            .clicked()
        {
            kill_selected_devices(&self.devices);
        }
    }

    fn render_restore_button(&mut self, ui: &mut egui::Ui, selected_count: usize) {
        if ui
            .add_sized(
                [180.0, 35.0],
                egui::Button::new(
                    egui::RichText::new(format!("✔ Restore Selected ({})", selected_count))
                        .color(egui::Color32::WHITE),
                )
                .fill(egui::Color32::from_rgb(50, 150, 50)),
            )
            .clicked()
        {
            restore_selected_devices(&self.devices);
        }
    }

    fn render_device_table(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(format!("Network Devices ({})", self.devices.len()))
                .size(16.0)
                .strong(),
        );
        ui.add_space(5.0);
        self.render_table_header(ui);
        ui.separator();
        self.render_table_content(ui);
    }

    fn render_table_header(&self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(245, 245, 245))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Select").strong().size(12.0));
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("IP Address").strong().size(12.0));
                    ui.add_space(90.0);
                    ui.label(egui::RichText::new("Hostname").strong().size(12.0));
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("MAC Address").strong().size(12.0));
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("Vendor").strong().size(12.0));
                    ui.add_space(80.0);
                    ui.label(egui::RichText::new("Status").strong().size(12.0));
                });
            });
    }

    fn render_table_content(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for (idx, mut item) in self.devices.iter_mut().enumerate() {
                    let device = item.value_mut();
                    let bg_color = if idx % 2 == 0 {
                        egui::Color32::from_rgb(255, 255, 255)
                    } else {
                        egui::Color32::from_rgb(250, 250, 250)
                    };
                    egui::Frame::none().fill(bg_color).show(ui, |ui| {
                        self.render_device_row(ui, device);
                    });
                    ui.add_space(2.0);
                }
            });
    }

    fn render_device_row(&self, ui: &mut egui::Ui, device: &mut NetworkDevice) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.checkbox(&mut device.selected, "");
            ui.add_space(30.0);
            ui.label(egui::RichText::new(&device.ip_address).size(12.0));
            ui.add_space(70.0);
            ui.label(egui::RichText::new(&device.hostname).size(12.0));
            ui.add_space(50.0);
            ui.label(egui::RichText::new(&device.mac_address).size(12.0));
            ui.add_space(50.0);
            ui.label(egui::RichText::new(&device.vendor).size(12.0));
            ui.add_space(70.0);
            let status_color = match device.status {
                DeviceStatus::Active => egui::Color32::from_rgb(50, 150, 50),
                DeviceStatus::Inactive => egui::Color32::from_rgb(100, 100, 100),
                _ => egui::Color32::from_rgb(150, 150, 150),
            };
            ui.colored_label(status_color, device.status.as_str());
        });
    }
}

impl eframe::App for NetworkManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(device) = self.device_receiver.try_recv() {
            if !self.devices.contains_key(&device.mac_address) {
                self.devices.insert(device.mac_address.clone(), device);
            }
        }

        if self.selected_interface.lock().unwrap().is_none() {
            if self.interface_selector.show(ctx) {
                if let Some(interface) = self.interface_selector.get_selected_interface() {
                    *self.selected_interface.lock().unwrap() = Some(interface.clone());
                    let (device_sender, device_receiver) = mpsc::unbounded_channel();
                    let (command_sender, command_receiver) = mpsc::unbounded_channel();
                    self.device_receiver = device_receiver;
                    self.command_sender = Some(command_sender);
                    let mut scanner = NetworkScanner::new(
                        interface.clone(),
                        self.devices.clone(),
                        device_sender,
                        command_receiver,
                    );
                    let error_clone = self.error.clone();
                    TOKIO_RUNTIME.spawn(async move {
                        if let Err(e) = scanner.start().await {
                            *error_clone.lock().unwrap() = Some(e.to_string());
                        }
                    });
                }
            }
        } else {
            if let Some(error) = self.error.lock().unwrap().as_ref() {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading("Error");
                        ui.add_space(20.0);
                        ui.label(error);
                    });
                });
                return;
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(10.0);
                self.render_header(ui);
                ui.add_space(1.0);
                self.render_info_panel(ui);
                ui.add_space(1.0);
                self.render_control_buttons(ui);
                ui.add_space(1.0);
                ui.separator();
                ui.add_space(1.0);
                self.render_device_table(ui);
            });
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
