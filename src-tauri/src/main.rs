// Hides the CMD window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    obs_overlay_exporter_lib::run();
}
