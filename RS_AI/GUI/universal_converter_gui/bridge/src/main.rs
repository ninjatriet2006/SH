#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = universal_converter_gui_bridge::run() {
        eprintln!("failed to run Universal Converter bridge: {error}");
    }
}
