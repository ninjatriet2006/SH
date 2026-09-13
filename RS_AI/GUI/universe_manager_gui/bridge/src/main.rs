#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = universe_manager_gui_bridge::run() {
        eprintln!("failed to run Universe Manager bridge: {error}");
    }
}
