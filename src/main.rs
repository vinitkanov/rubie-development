mod models;
mod network;
mod ui;
mod scanner;
mod restore;
mod disconnect;

use anyhow::Result;
use eframe::egui;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 650.0])
            .with_title("Network Device Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "Network Device Manager",
        options,
        Box::new(|cc| {
            // Force light mode
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Box::new(ui::NetworkManagerApp::new(cc))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
}
