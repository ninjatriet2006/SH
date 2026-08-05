// Tauri v2 app shell for filen_gui.
//
// Phase 1 (scaffold): opens a window that loads the Vite frontend. Core
// commands (operations.rs / transfer.rs) are wired in later phases.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            // Phase 2+: manage AppState, register commands, spawn whoami worker.
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}