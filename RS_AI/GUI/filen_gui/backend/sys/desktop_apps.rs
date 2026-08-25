//! [INTEGRITY NOTES]
//! Mục đích: Phân tích và liệt kê các ứng dụng desktop đã cài trên Linux cho tính năng "Open With".
//! Trách nhiệm: Đọc file `.desktop` từ chuẩn XDG, trích xuất MIME-types và trả về danh sách ứng dụng.
//! Tương tác: Giao tiếp với sys/mod.rs, operations.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub mime_types: Vec<String>,
}

/// Trả về danh sách tất cả các ứng dụng có thể mở file trên Linux
pub fn get_desktop_apps() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    // 1. Quét thư mục chuẩn chứa các file .desktop trên hệ thống
    let mut paths = vec![PathBuf::from("/usr/share/applications")];
    // 2. Quét thêm thư mục .desktop riêng của user hiện tại
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/share/applications"));
    }

    // Duyệt qua từng thư mục
    for path in paths {
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let p = entry.path();
                // Chỉ xử lý các file có phần mở rộng là .desktop
                if p.is_file() && p.extension().map_or(false, |ext| ext == "desktop") {
                    // Trích xuất metadata thành đối tượng DesktopApp
                    if let Some(app) = parse_desktop_file(&p) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    // Sắp xếp danh sách ứng dụng theo bảng chữ cái
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    // Loại bỏ các ứng dụng trùng lặp tên (ưu tiên cái quét được trước)
    apps.dedup_by(|a, b| a.name == b.name);

    apps
}

/// Hàm nội bộ: Đọc nội dung file .desktop và trích xuất cấu hình (INI format)
fn parse_desktop_file(path: &PathBuf) -> Option<DesktopApp> {
    let content = fs::read_to_string(path).ok()?;
    
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut mime_types = Vec::new();
    let mut is_app = false; // Cờ kiểm tra đây có đúng là "Application" không
    let mut no_display = false; // Cờ kiểm tra ứng dụng có bị ẩn không

    let mut in_desktop_entry = false; // Đánh dấu con trỏ đang đọc đúng block [Desktop Entry]

    for line in content.lines() {
        let line = line.trim();
        // Kiểm tra block header
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        // Bỏ qua nếu không nằm trong block [Desktop Entry]
        if !in_desktop_entry {
            continue;
        }

        // Phân tách key=value
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            
            match key {
                "Type" if val == "Application" => is_app = true,
                "Name" if name.is_empty() => name = val.to_string(), // Chỉ lấy Name chuẩn (bỏ qua bản dịch ngôn ngữ)
                "Exec" => exec = val.to_string(), // Lệnh thực thi
                "Icon" => icon = val.to_string(), // Tên/đường dẫn icon
                "MimeType" => {
                    // Cắt các MimeType được phân tách bằng dấu chấm phẩy
                    mime_types = val.split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                },
                "NoDisplay" if val.to_lowercase() == "true" => no_display = true, // Bị đánh dấu ẩn
                _ => {}
            }
        }
    }

    // Chỉ trả về nếu là app hợp lệ, không bị ẩn và có đủ Name + Exec
    if is_app && !no_display && !name.is_empty() && !exec.is_empty() {
        // Dọn dẹp chuỗi lệnh Exec: xóa bỏ các ký hiệu tham số (placeholders) mặc định của môi trường desktop Linux
        let clean_exec = exec
            .replace("%f", "")
            .replace("%F", "")
            .replace("%u", "")
            .replace("%U", "")
            .replace("%c", "")
            .replace("%k", "")
            .trim()
            .to_string();

        Some(DesktopApp {
            name,
            exec: clean_exec,
            icon,
            mime_types,
        })
    } else {
        None
    }
}
