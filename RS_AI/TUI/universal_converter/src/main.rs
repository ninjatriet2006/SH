use std::env;
use std::path::PathBuf;

mod config;
mod core;
mod engine;
mod system;
mod ui;

use crate::config::ConfigManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Watchdog (panic hooks and Ctrl+C processes cleaner)
    core::watchdog::init_watchdog();

    // Check if launched via Context Menu
    let args: Vec<String> = env::args().collect();

    if args.len() >= 4 && args[1] == "--context" {
        // Run from Context Menu (Skip Menu, immediately display action prompts for preselected files)
        let mode = &args[2];
        let file_args = &args[3..];

        let mut files = Vec::new();
        for f in file_args {
            let path = PathBuf::from(f);
            if path.exists() {
                files.push(core::scanner::classify_file(path));
            }
        }

        if files.is_empty() {
            println!("[⚠️] Không tìm thấy file hợp lệ được chọn từ Context Menu!");
            system::os_utils::hold_terminal();
            return Ok(());
        }

        // Initialize configs
        let config_mgr = ConfigManager::new();
        let _ = config_mgr.init_all_configs();

        println!("\n🚀 Khởi tạo menu hành động cho các tệp đã chọn (Chế độ: {})...", mode);
        let run_res = ui::prompt::process_selected_files(files, Some(mode)).await;

        if let Err(e) = run_res {
            println!("[❌] Xử lý thất bại: {}", e);
        }

        system::os_utils::hold_terminal();
        return Ok(());
    }

    // 1. Ensure we are running in a terminal, or spawn one if clicked from GUI.
    system::os_utils::ensure_terminal()?;

    // 2. Async Boot: Check dependencies in the background.
    let deps_check = tokio::spawn(system::dependencies::check_all());

    // 3. UI: Show Main Menu
    ui::prompt::show_main_menu().await?;

    // Wait for dependencies check to finish if it hasn't already
    if let Ok(Ok(deps_result)) = deps_check.await
        && !deps_result.is_ok
    {
        system::dependencies::prompt_install(&deps_result.missing)?;
    }

    // Hold terminal before exit if needed
    system::os_utils::hold_terminal();

    Ok(())
}
