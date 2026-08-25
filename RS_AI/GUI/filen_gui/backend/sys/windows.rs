//! [INTEGRITY NOTES]
//! Mục đích: Xử lý logic đặc thù cho Windows của backend.
//! Trách nhiệm: Tìm nhị phân filen trên Windows, xử lý clipboard qua clip.exe và mount fuse.
//! Tương tác: Giao tiếp với sys/mod.rs, operations.rs

use std::path::{Path, PathBuf};
use std::process::Command;

/// Hàm copy_to_clipboard dùng để chép chuỗi vào Clipboard trên môi trường Windows.
/// Bóc tách luồng pipe vào tiến trình `clip.exe` của Windows.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // Khởi chạy hệ thống clip mặc định
    let child = Command::new("clip")
        .stdin(std::process::Stdio::piped())
        .spawn();
    
    if let Ok(mut c) = child {
        if let Some(mut stdin) = c.stdin.take() {
            use std::io::Write;
            // Ép nội dung cần sao chép vào trong stdin
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = c.wait(); // Chờ tiến trình xả buffer
        return Ok(());
    }

    Err("Không tìm thấy công cụ sao chép clipboard (clip.exe)".to_string())
}

/// Lấy lệnh tương tác trên Windows (Windows không có tình trạng bị kẹt buffer cứng như Unix nên trả về trực tiếp)
pub fn get_interactive_command(bin: &Path) -> Command {
    Command::new(bin)
}

/// Lấy lệnh tương tác bất đồng bộ trên Windows (Trả về thẳng, không bọc)
pub fn get_interactive_tokio_command(cmd: tokio::process::Command) -> tokio::process::Command {
    cmd
}

/// Trả về chuỗi hướng dẫn mount cho Windows
pub fn mount_fuse_note() -> String {
    "Mount yêu cầu WinFSP (https://winfsp.dev/rel) hoặc WinFUSE.".to_string()
}

/// Lấy tên file thực thi mặc định của filen trên Windows (Thường là file cmd do nodejs cài).
pub fn default_filen_bin_name() -> PathBuf {
    PathBuf::from("filen.cmd")
}

/// Quét các thư mục lưu trữ cục bộ để tìm binary `filen` trên Windows.
pub fn scan_local_bins(home: &Path) -> Option<PathBuf> {
    // 1. Kiểm tra trực tiếp file exe
    let win_path = home.join(".filen-cli\\bin\\filen.exe");
    if win_path.exists() {
        return Some(win_path);
    }
    
    // 2. Kiểm tra file script cmd wrapper
    let win_cmd_path = home.join(".filen-cli\\bin\\filen.cmd");
    if win_cmd_path.exists() {
        return Some(win_cmd_path);
    }

    // 3. Fallback tìm file filen cài qua npm global
    if let Ok(appdata) = std::env::var("APPDATA") {
        let npm_path = PathBuf::from(appdata).join("npm\\filen.cmd");
        if npm_path.exists() {
            return Some(npm_path);
        }
    }
    
    None
}
