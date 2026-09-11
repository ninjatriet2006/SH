#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = img_splt_gui_bridge::run() {
        eprintln!("failed to run Image Splitter bridge: {error}");
    }
}
