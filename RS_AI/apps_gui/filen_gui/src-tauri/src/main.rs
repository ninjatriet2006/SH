// Thin entry point: delegates to the library so the same code can run on
// desktop and mobile. All app logic lives in `lib.rs`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    filen_gui_tauri::run();
}