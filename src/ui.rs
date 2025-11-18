use crate::models::DeviceStatus;
use crate::network::NetworkScanner;
use eframe::egui;
use std::time::{Duration, Instant};
use crate::interface_selector::InterfaceSelector;
use pnet::datalink::NetworkInterface;

pub struct NetworkManagerApp {
    scanner: NetworkScanner,
    auto_refresh: bool,
    last_scan: Instant,
    interface_selector: InterfaceSelector,
    selected_interface: Option<NetworkInterface>,
}

impl NetworkManagerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let scanner = NetworkScanner::new();

        Self {
            scanner,
            auto_refresh: false,
            last_scan: Instant::now(),
            interface_selector: InterfaceSelector::new(),
            selected_interface: None,
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
        let network_info = self.scanner.network_info.lock().unwrap();

        ui.horizontal(|ui| {
            ui.add_space(5.0);

            // Network Range Box
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(240, 240, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.set_width(340.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🌐 Network Range").strong());
                        ui.label(
                            egui::RichText::new(&network_info.network_range)
                                .size(16.0)
                                .color(egui::Color32::from_rgb(50, 50, 50)),
                        );
                    });
                });

            ui.add_space(5.0);

            // Gateway Box
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(240, 240, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .inner_margin(15.0)
                .show(ui, |ui| {
                    ui.set_width(330.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🚪 Gateway").strong());
                        ui.label(
                            egui::RichText::new(&network_info.gateway)
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
                            egui::RichText::new(network_info.active_devices.to_string())
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

            let scanning = *self.scanner.scanning.lock().unwrap();

            // Scan Network Button
            if ui
                .add_sized(
                    [140.0, 35.0],
                    egui::Button::new(
                        egui::RichText::new(if scanning {
                            "⏳ Scanning..."
                        } else {
                            "🔍 Scan Network"
                        })
                        .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(0, 120, 215)),
                )
                .clicked()
                && !scanning
            {
                self.scanner.scan_network();
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
                && !scanning
            {
                self.scanner.scan_network();
            }

            ui.add_space(100.0);

            let selected_count = self
                .scanner
                .devices
                .lock()
                .unwrap()
                .iter()
                .filter(|d| d.selected)
                .count();

            // Kill Selected Button
            if ui
                .add_sized(
                    [160.0, 35.0],
                    egui::Button::new(
                        egui::RichText::new(format!("✖ Kill Selected ({})", selected_count))
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(200, 50, 50)),
                )
                .clicked()
            {
                self.scanner.kill_selected_devices();
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
                self.scanner.restore_selected_devices();
            }

            ui.add_space(5.0);

            // Restore All Button
            if ui
                .add_sized(
                    [130.0, 35.0],
                    egui::Button::new(
                        egui::RichText::new("✔ Restore All").color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(60, 130, 60)),
                )
                .clicked()
            {
                self.scanner.restore_all_devices();
            }

            ui.add_space(5.0);

            // Kill All Button
            if ui
                .add_sized(
                    [120.0, 35.0],
                    egui::Button::new(egui::RichText::new("⚠ Kill All").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(180, 40, 40)),
                )
                .clicked()
            {
                self.scanner.kill_all_devices();
            }
        });
    }

    fn render_device_table(&mut self, ui: &mut egui::Ui) {
        let mut devices = self.scanner.devices.lock().unwrap();

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
                    ui.add_space(80.0);
                    ui.label(egui::RichText::new("Response Time").strong().size(12.0));
                });
            });

        ui.separator();

        // Table content with scrollable area
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

                            // Checkbox
                            ui.checkbox(&mut device.selected, "");

                            ui.add_space(30.0);

                            // IP Address
                            ui.label(egui::RichText::new(&device.ip_address).size(12.0));
                            ui.add_space(70.0);

                            // Hostname
                            ui.label(egui::RichText::new(&device.hostname).size(12.0));
                            ui.add_space(50.0);

                            // MAC Address
                            ui.label(egui::RichText::new(&device.mac_address).size(12.0));
                            ui.add_space(50.0);

                            // Vendor
                            ui.label(egui::RichText::new(&device.vendor).size(12.0));
                            ui.add_space(70.0);

                            // Status
                            let status_color = match device.status {
                                DeviceStatus::Active => egui::Color32::from_rgb(50, 150, 50),
                                DeviceStatus::Inactive => egui::Color32::from_rgb(100, 100, 100),
                                DeviceStatus::Blocked => egui::Color32::from_rgb(200, 50, 50),
                                DeviceStatus::Unknown => egui::Color32::from_rgb(150, 150, 150),
                            };
                            ui.colored_label(status_color, device.status.as_str());
                            ui.add_space(60.0);

                            // Response Time
                            ui.label(
                                egui::RichText::new(format!("{:.1}ms", device.response_time))
                                    .size(12.0),
                            );
                        });
                    });

                    ui.add_space(2.0);
                }
            });
    }
}

impl eframe::App for NetworkManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.selected_interface.is_none() {
            if self.interface_selector.show(ctx) {
                self.selected_interface = self.interface_selector.get_selected_interface();
                if let Some(interface) = &self.selected_interface {
                    *self.scanner.interface.lock().unwrap() = Some(interface.clone());
                    if let Ok(info) = self.scanner.get_local_network_info() {
                        let mut network_info = self.scanner.network_info.lock().unwrap();
                        *network_info = info;
                    }
                    self.scanner.scan_network();
                }
            }
        } else {
            // Auto-refresh logic
            if self.auto_refresh && self.last_scan.elapsed() >= Duration::from_secs(60) {
                if !*self.scanner.scanning.lock().unwrap() {
                    self.scanner.scan_network();
                    self.last_scan = Instant::now();
                }
            }

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

            // Request repaint for smooth updates
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
