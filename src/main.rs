mod models;
mod network;
mod ui;
mod scanner;
mod restore;
mod disconnect;
mod privileges;
mod interface_selector;

use anyhow::Result;
use eframe::egui;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

fn run_app() -> Result<()> {
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

#[cfg(windows)]
fn main() -> Result<()> {
    if !privileges::is_admin() {
        privileges::relaunch_as_admin()?;
        return Ok(());
    }
    run_app()
}

#[cfg(not(windows))]
fn main() -> Result<()> {
    run_app()
}
