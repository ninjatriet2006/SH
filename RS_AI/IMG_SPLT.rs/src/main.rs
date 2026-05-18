mod config;
mod env_check;
mod scanner;
mod renamer;
mod processor;
mod distributor;

use std::env;
use std::io::IsTerminal;
use std::process::Command;

fn pause_and_exit(code: i32) -> ! {
    println!("\nNhấn Enter để thoát...");
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
    std::process::exit(code);
}

fn main() {
    // Nếu không được chạy trong terminal thực sự (ví dụ: Double click trên GUI file manager)
    if !std::io::stdout().is_terminal() {
        let exe = env::current_exe().unwrap();
        
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd.exe").arg("/c").arg("start").arg("cmd.exe").arg("/c").arg(&exe).spawn();
            return;
        }

        #[cfg(target_os = "linux")]
        {
            let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "x-terminal-emulator", "xterm"];
            for term in terminals {
                // Thử mở terminal và execute chính file này
                if Command::new(term).arg("--").arg(&exe).spawn().is_ok() || 
                   Command::new(term).arg("-e").arg(&exe).spawn().is_ok() {
                    return;
                }
            }
        }
    }

    // Khởi tạo terminal UI
    env_check::resize_terminal();
    
    let current_dir = env::current_dir().unwrap();
    println!("1. Đã nhận diện và di chuyển đến: {}", current_dir.display());

    // Đọc cấu hình
    let settings = config::load_or_create_settings();

    // Check ffmpeg / ffprobe
    env_check::check_ffmpeg();

    // Scan & lọc file
    let mut files = scanner::scan_files();
    let initial_count = files.len();
    
    if initial_count == 0 {
        println!("LỖI: Thư mục trống hoặc không có file khả dụng!");
        pause_and_exit(1);
    }
    
    // Bước 1: Rename
    renamer::rename_files(&mut files);

    // Bước 2 & 3: Lọc ảnh, Process (Format + Upscale)
    let image_files: Vec<_> = files.into_iter().filter(|f| scanner::is_image_extension(f)).collect();
    if image_files.is_empty() {
        println!("LỖI: Không tìm thấy file ảnh (jpg, png, webp,...) để xử lý!");
        pause_and_exit(1);
    }

    // Process files into `_process`
    let _process_files = processor::process_files(&image_files, &settings);

    // Swap back
    let process_dir = current_dir.join(format!("{}_process", current_dir.file_name().unwrap_or_default().to_string_lossy()));
    if process_dir.exists() {
        processor::swap_directories(&process_dir, &image_files);
    }

    // Rescan to get the final valid image files after swap
    let final_files: Vec<_> = scanner::scan_files().into_iter().filter(|f| scanner::is_image_extension(f)).collect();

    // Phân phối thư mục
    distributor::distribute_files(&final_files, &settings);

    pause_and_exit(0);
}
