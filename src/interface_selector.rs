
use eframe::egui;
use pnet::datalink::NetworkInterface;

pub struct InterfaceSelector {
    interfaces: Vec<NetworkInterface>,
    selected_interface: Option<NetworkInterface>,
    selected_interface_name: String,
}

impl InterfaceSelector {
    pub fn new() -> Self {
        let interfaces = pnet::datalink::interfaces()
            .into_iter()
            .filter(|iface| !iface.is_loopback() && !iface.ips.is_empty())
            .collect();
        Self {
            interfaces,
            selected_interface: None,
            selected_interface_name: "Select an interface".to_string(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut selection_made = false;
        egui::Window::new("Select Network Interface")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Select the network interface to use for scanning:");

                egui::ComboBox::from_label("")
                    .selected_text(&self.selected_interface_name)
                    .show_ui(ui, |ui| {
                        for iface in &self.interfaces {
                            if ui
                                .selectable_value(
                                    &mut self.selected_interface,
                                    Some(iface.clone()),
                                    &iface.description,
                                )
                                .clicked()
                            {
                                self.selected_interface_name = iface.description.clone();
                            }
                        }
                    });

                if ui.button("Select").clicked() {
                    if self.selected_interface.is_some() {
                        selection_made = true;
                    }
                }
            });
        selection_made
    }

    pub fn get_selected_interface(&self) -> Option<NetworkInterface> {
        self.selected_interface.clone()
    }
}
