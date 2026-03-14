// Hide the CMD window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

mod app;
mod assets;
mod generator;
mod parser;
mod utils;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OBS Overlay Exporter")
            .with_inner_size([700.0, 620.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "OBS Overlay Exporter",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::default()))),
    )
}
