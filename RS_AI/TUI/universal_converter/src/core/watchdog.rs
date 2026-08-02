use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use sysinfo::System;

// Global list of running child processes
static ACTIVE_PROCESSES: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));
static SYSTEM_MONITOR: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::new_all()));

pub fn init_watchdog() {
    // 1. Set panic hook to clean up processes if Rust code crashes
    std::panic::set_hook(Box::new(|info| {
        eprintln!("\n[🚨 CRITICAL ERROR] Chương trình bị crash đột ngột: {:?}", info);
        kill_all_active_children();
        std::process::exit(1);
    }));

    // 2. Set Ctrl+C handler to kill children immediately
    let _ = ctrlc::set_handler(|| {
        println!("\n[🛑] Đang hủy toàn bộ tiến trình con đang chạy...");
        kill_all_active_children();
        std::process::exit(130);
    });
}

pub fn register_child(pid: u32) {
    if let Ok(mut pids) = ACTIVE_PROCESSES.lock() {
        pids.push(pid);
    }
}

pub fn deregister_child(pid: u32) {
    if let Ok(mut pids) = ACTIVE_PROCESSES.lock() {
        pids.retain(|&x| x != pid);
    }
}

pub fn kill_all_active_children() {
    if let Ok(mut pids) = ACTIVE_PROCESSES.lock() {
        for pid in pids.iter() {
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("taskkill")
                    .args(&["/F", "/PID", &pid.to_string()])
                    .status();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
        }
        pids.clear();
    }
}

// Support command helper to call shell-specific kills if needed
use std::process::Command;

pub fn wait_if_overloaded() {
    if let Ok(mut sys) = SYSTEM_MONITOR.lock() {
        sys.refresh_cpu();
        // Wait until CPU drops below 90%
        let mut loop_count = 0;
        while sys.global_cpu_info().cpu_usage() > 90.0 {
            // Prevent infinite loop if something goes wrong, break after 10 seconds
            if loop_count > 20 {
                break;
            }
            thread::sleep(Duration::from_millis(500));
            sys.refresh_cpu();
            loop_count += 1;
        }
    }
}
