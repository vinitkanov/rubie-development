
use eframe::egui;
use pnet::datalink::NetworkInterface;

pub struct InterfaceSelector {
    interfaces: Vec<NetworkInterface>,
    selected_interface: Option<NetworkInterface>,
}

impl InterfaceSelector {
    pub fn new() -> Self {
        let interfaces = pnet::datalink::interfaces()
            .into_iter()
            .filter(|iface| iface.is_up() && !iface.is_loopback() && iface.mac.is_some())
            .collect();
        Self {
            interfaces,
            selected_interface: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut selection_made = false;
        egui::Window::new("Select Network Interface")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Select the network interface to use for scanning:");
                for iface in &self.interfaces {
                    if ui.radio_value(&mut self.selected_interface, Some(iface.clone()), &iface.name).clicked() {
                        // Nothing to do here, the value is already updated
                    }
                }
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
