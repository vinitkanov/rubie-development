use crate::{
    disconnect::kill_selected_devices,
    models::{DeviceStatus, NetworkDevice},
    restore::restore_selected_devices,
    scanner::NetworkScanner,
    TOKIO_RUNTIME,
};
use eframe::egui;
use pnet::datalink::NetworkInterface;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::interface_selector::InterfaceSelector;

pub struct NetworkManagerApp {
    devices: Arc<Mutex<Vec<NetworkDevice>>>,
    auto_refresh: bool,
    last_scan: Instant,
    interface_selector: InterfaceSelector,
    selected_interface: Option<NetworkInterface>,
    device_receiver: mpsc::UnboundedReceiver<NetworkDevice>,
}

impl NetworkManagerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (_device_sender, device_receiver) = mpsc::unbounded_channel();
        let devices = Arc::new(Mutex::new(Vec::new()));

        // TOKIO_RUNTIME.spawn(async move {
        //     let scanner = NetworkScanner::new(device_sender);
        //     scanner.run().await;
        // });

        Self {
            devices,
            auto_refresh: false,
            last_scan: Instant::now(),
            interface_selector: InterfaceSelector::new(),
            selected_interface: None,
            device_receiver,
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);

            // Title
            ui.heading("🖧 Network Device Manager");
            ui.add_space(20.0);
            ui.label("Monitor and manage network devices");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                // Connection status
                ui.colored_label(egui::Color32::GREEN, "● Connected");
                ui.add_space(10.0);

                // Auto-refresh toggle
                if ui.checkbox(&mut self.auto_refresh, "🗘 Auto-refresh").clicked() {
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

            // Placeholder for Network Range
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(240, 240, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🌐 Network Range").strong());
                        ui.label(
                            egui::RichText::new("192.168.1.0/24") // Placeholder
                                .size(16.0)
                                .color(egui::Color32::from_rgb(50, 50, 50)),
                        );
                    });
                });

            ui.add_space(5.0);

            // Placeholder for Gateway
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(240, 240, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.set_width(330.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🚪 Gateway").strong());
                        ui.label(
                            egui::RichText::new("192.168.1.1") // Placeholder
                                .size(16.0)
                                .color(egui::Color32::from_rgb(50, 50, 50)),
                        );
                    });
                });

            ui.add_space(5.0);

            // Active Devices Box
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(240, 240, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📊 Active Devices").strong());
                        ui.label(
                            egui::RichText::new(self.devices.lock().unwrap().len().to_string())
                                .size(16.0)
                                .color(egui::Color32::from_rgb(50, 50, 50)),
                        );
                    });
                });
        });
    }

    fn render_control_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(5.0);

            // Scan Network Button
            if ui
                .add_sized(
                    [140.0, 35.0],
                    egui::Button::new(egui::RichText::new("🔍 Scan Network").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(0, 120, 215)),
                )
                .clicked()
            {
                // To be implemented
            }

            ui.add_space(5.0);

            // Refresh Button
            if ui
                .add_sized(
                    [120.0, 35.0],
                    egui::Button::new(egui::RichText::new("🔄 Refresh").color(egui::Color32::BLACK))
                        .fill(egui::Color32::from_rgb(230, 230, 230)),
                )
                .clicked()
            {
                // To be implemented
            }

            ui.add_space(100.0);

            let selected_count = self.devices.lock().unwrap().iter().filter(|d| d.selected).count();

            // Disconnect Selected Button
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
                let _devices_to_disconnect: Vec<NetworkDevice> = self
                    .devices
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|d| d.selected)
                    .cloned()
                    .collect();
                kill_selected_devices(&self.devices);
            }

            ui.add_space(5.0);

            // Restore Selected Button
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
                let _devices_to_restore: Vec<NetworkDevice> = self
                    .devices
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|d| d.selected)
                    .cloned()
                    .collect();
                restore_selected_devices(&self.devices);
            }
        });
    }

    fn render_device_table(&mut self, ui: &mut egui::Ui) {
        let mut devices = self.devices.lock().unwrap();

        ui.label(
            egui::RichText::new(format!("Network Devices ({})", devices.len()))
                .size(16.0)
                .strong(),
        );

        ui.add_space(5.0);

        // Table header
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

        ui.separator();

        // Table content
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for (idx, device) in devices.iter_mut().enumerate() {
                    let bg_color = if idx % 2 == 0 {
                        egui::Color32::from_rgb(255, 255, 255)
                    } else {
                        egui::Color32::from_rgb(250, 250, 250)
                    };

                    egui::Frame::none().fill(bg_color).show(ui, |ui| {
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
                    });

                    ui.add_space(2.0);
                }
            });
    }
}

impl eframe::App for NetworkManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(device) = self.device_receiver.try_recv() {
            let mut devices = self.devices.lock().unwrap();
            if !devices.iter().any(|d| d.ip_address == device.ip_address) {
                devices.push(device);
            }
        }

        if self.selected_interface.is_none() {
            if self.interface_selector.show(ctx) {
                self.selected_interface = self.interface_selector.get_selected_interface();
                if let Some(interface) = &self.selected_interface {
                    let (device_sender, device_receiver) = mpsc::unbounded_channel();
                    self.device_receiver = device_receiver;
                    let scanner = NetworkScanner::new(interface.clone(), device_sender);
                    TOKIO_RUNTIME.spawn(async move {
                        if let Err(e) = scanner.start().await {
                            eprintln!("Failed to start scanner: {}", e);
                        }
                    });
                }
            }
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(10.0);
                self.render_header(ui);
                ui.add_space(15.0);
                self.render_info_panel(ui);
                ui.add_space(15.0);
                self.render_control_buttons(ui);
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);
                self.render_device_table(ui);
            });
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
