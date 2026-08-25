//! [INTEGRITY NOTES]
//! Mục đích: Xử lý logic đặc thù cho Unix của backend.
//! Trách nhiệm: Tìm nhị phân filen trên Unix, xử lý clipboard qua wl-copy/xclip/xsel và mount fuse.
//! Tương tác: Giao tiếp với sys/mod.rs, operations.rs

use std::path::{Path, PathBuf};
use std::process::Command;

/// Hàm copy_to_clipboard dùng để chép chuỗi vào Clipboard trên môi trường Unix.
/// Sẽ thử lần lượt ưu tiên: wl-copy (Wayland) -> xclip (X11) -> xsel (X11).
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // 1. Thử bóc tách pipe vào `wl-copy` (dành cho Wayland)
    let child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped()) // Mở luồng ghi stdin
        .spawn();
    if let Ok(mut c) = child {
        if let Some(mut stdin) = c.stdin.take() {
            use std::io::Write;
            // Ghi nội dung vào luồng stdin của tiến trình con
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = c.wait(); // Chờ tiến trình exit
        return Ok(());
    }

    // 2. Fallback thử `xclip` nếu không có Wayland (X11)
    let child = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(std::process::Stdio::piped())
        .spawn();
    if let Ok(mut c) = child {
        if let Some(mut stdin) = c.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = c.wait();
        return Ok(());
    }

    // 3. Fallback cuối thử `xsel` (Công cụ clipboard thế hệ cũ trên X11)
    let child = Command::new("xsel")
        .arg("--clipboard")
        .arg("--input")
        .stdin(std::process::Stdio::piped())
        .spawn();
    if let Ok(mut c) = child {
        if let Some(mut stdin) = c.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = c.wait();
        return Ok(());
    }

    // Trả về lỗi nếu không tool nào tồn tại trên hệ thống
    Err("Không tìm thấy công cụ sao chép clipboard (wl-copy, xclip, xsel)".to_string())
}

/// Bọc đối tượng Command qua `stdbuf` để bỏ đệm (buffer) output. 
/// Cần thiết cho việc pipe các log real-time mà không bị kẹt vì đệm của HĐH.
pub fn get_interactive_command(bin: &Path) -> Command {
    // Nếu hệ thống Unix có công cụ `stdbuf` (có trong coreutils)
    if which::which("stdbuf").is_ok() {
        let mut c = Command::new("stdbuf");
        // -o0: Vô hiệu đệm stdout; -e0: Vô hiệu đệm stderr
        c.arg("-o0").arg("-e0").arg(bin);
        c
    } else {
        Command::new(bin)
    }
}

/// Bọc đối tượng tokio::process::Command qua `stdbuf` cho các tác vụ bất đồng bộ.
pub fn get_interactive_tokio_command(cmd: tokio::process::Command) -> tokio::process::Command {
    if which::which("stdbuf").is_ok() {
        // Trích xuất binary name từ command gốc
        let program = cmd.as_std().get_program().to_os_string();
        // Trích xuất mảng arguments gốc
        let args: Vec<std::ffi::OsString> = cmd.as_std().get_args().map(ToOwned::to_owned).collect();
        
        // Khởi tạo lại bằng `stdbuf`
        let mut c = tokio::process::Command::new("stdbuf");
        c.arg("-o0").arg("-e0").arg(&program).args(&args);
        c
    } else {
        cmd
    }
}

/// Trả về thông báo lỗi hướng dẫn cài đặt thư viện mount FUSE trên Unix.
pub fn mount_fuse_note() -> String {
    if cfg!(target_os = "macos") {
        "Mount yêu cầu FUSE-T (https://www.fuse-t.org) hoặc macFUSE (https://osxfuse.github.io).".to_string()
    } else {
        "Mount yêu cầu FUSE3 (https://github.com/libfuse/libfuse). Cài đặt thêm gói fuse3 nếu chưa có. Trên Linux mount point phải nằm trong thư mục home.".to_string()
    }
}

/// Lấy tên file thực thi mặc định của filen (Unix không có đuôi exe).
pub fn default_filen_bin_name() -> PathBuf {
    PathBuf::from("filen")
}

/// Hàm quét tìm nhị phân `filen` bị ẩn sâu trong các cấu hình riêng của user.
pub fn scan_local_bins(home: &Path) -> Option<PathBuf> {
    // Trường hợp 1: Đường dẫn mặc định của script cài đặt
    let unix_path = home.join(".filen-cli/bin/filen");
    if unix_path.exists() {
        return Some(unix_path);
    }
    
    // Trường hợp 2: Đường dẫn thư mục config
    let unix_config_path = home.join(".config/filen-cli/bin/filen");
    if unix_config_path.exists() {
        return Some(unix_config_path);
    }
    None
}
