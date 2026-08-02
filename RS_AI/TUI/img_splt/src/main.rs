mod config;
mod distributor;
mod env_check;
mod processor;
mod renamer;
mod scanner;

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
    // [Systems Thinking: Bước 1 - Ranh giới & Môi trường]
    // Hệ thống kiểm tra xem nó có đang chạy trong môi trường Terminal chuẩn không.
    // Nếu không (vd: click đúp từ GUI), nó tương tác với Môi trường bằng cách gọi HĐH tạo Terminal mới.
    if !std::io::stdout().is_terminal() {
        let exe = env::current_exe().unwrap();

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd.exe")
                .arg("/c")
                .arg("start")
                .arg("cmd.exe")
                .arg("/c")
                .arg(&exe)
                .spawn();
            return;
        }

        #[cfg(target_os = "linux")]
        {
            let terminals = [
                "gnome-terminal",
                "konsole",
                "xfce4-terminal",
                "x-terminal-emulator",
                "xterm",
            ];
            for term in terminals {
                // Thử mở terminal và execute chính file này
                if Command::new(term).arg("--").arg(&exe).spawn().is_ok()
                    || Command::new(term).arg("-e").arg(&exe).spawn().is_ok()
                {
                    return;
                }
            }
        }
    }

    // [Systems Thinking: Bước 1 tiếp theo - Chuẩn bị môi trường]
    // Khởi tạo terminal UI để ép kích thước hiển thị chuẩn
    env_check::resize_terminal();

    let current_dir = env::current_dir().unwrap();
    println!("1. Đã nhận diện và di chuyển đến: {}", current_dir.display());

    // [Systems Thinking: Bước 2 - Nạp Cấu Hình (Causality/Tính nhân quả)]
    // Đọc cấu hình từ settings.yaml. Biến `settings` này là hạt nhân khởi nguồn.
    // Mọi thay đổi ở đây sẽ lan truyền (Cascade) và quyết định hành vi của distributor và processor bên dưới.
    let settings = config::load_or_create_settings();

    // Check các công cụ môi trường như ffmpeg / ffprobe
    env_check::check_ffmpeg();

    // [Systems Thinking: Bước 3 - Thu thập dữ liệu & Vòng lặp phản hồi]
    // Quét file hiện có. Ở sâu bên trong hàm scan_files() chứa một Feedback Loop:
    // Nếu file bị khóa (Permission Denied), nó không crash mà hỏi mật khẩu rồi chạy lại lệnh `sudo` (Balancing Loop).
    let mut files = scanner::scan_files();
    let initial_count = files.len();

    if initial_count == 0 {
        println!("LỖI: Thư mục trống hoặc không có file khả dụng!");
        pause_and_exit(1);
    }

    // [Systems Thinking: Bước 4 - Tiền xử lý (Đổi tên)]
    // Dòng chảy dữ liệu (Data Flow) phải sạch. Việc chuẩn hóa tên file ở đây
    // đảm bảo các bước phân phối (Distributor) phía sau không bị lỗi khi sắp xếp.
    renamer::rename_files(&mut files);

    // Bước 2 & 3: Lọc ảnh, Process (Format + Upscale)
    let image_files: Vec<_> = files.into_iter().filter(|f| scanner::is_image_extension(f)).collect();
    if image_files.is_empty() {
        println!("LỖI: Không tìm thấy file ảnh (jpg, png, webp,...) để xử lý!");
        pause_and_exit(1);
    }

    // [Systems Thinking: Bước 5 - Xử lý cốt lõi (Emergence & Resilience)]
    // - Emergence (Tính trồi): Nhờ kết hợp 'rayon' đa luồng bên trong, quá trình này chạy song song cực nhanh.
    // - Resilience (Phục hồi): Xử lý vào thư mục ảo `_process` (Out-of-place).
    //   Lỡ sự cố mất điện giữa chừng xảy ra, file gốc vẫn nguyên vẹn không bị hỏng (Data loss).
    let _process_files = processor::process_files(&image_files, &settings);

    // Hoán đổi (Swap) file từ `_process` ra ngoài và cất file cũ vào `_old` một cách an toàn (Atomic operation).
    let process_dir = current_dir.join(format!(
        "{}_process",
        current_dir.file_name().unwrap_or_default().to_string_lossy()
    ));
    if process_dir.exists() {
        processor::swap_directories(&process_dir, &image_files);
    }

    // Rescan to get the final valid image files after swap
    let final_files: Vec<_> = scanner::scan_files()
        .into_iter()
        .filter(|f| scanner::is_image_extension(f))
        .collect();

    // [Systems Thinking: Bước 6 - Phân phối đầu ra (Interconnectedness)]
    // Khối cuối cùng tiếp nhận mọi thành quả ở trên, tính toán thuật toán chia file (Balanced/Greedy)
    // dựa vào biến `settings` ở Bước 2, và phân bổ vật lý vào các thư mục Chapter.
    distributor::distribute_files(&final_files, &settings);

    pause_and_exit(0);
}
