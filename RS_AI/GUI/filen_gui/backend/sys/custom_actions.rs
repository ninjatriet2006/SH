//! [INTEGRITY NOTES]
//! Mục đích: Phân tích và liệt kê các action tùy chỉnh của người dùng cho Context Menu.
//! Trách nhiệm: Đọc file `.action` từ `~/.local/share/filen_gui/actions/`, phân tích cú pháp INI để lấy metadata.
//! Tương tác: Giao tiếp với sys/mod.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAction {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub selection: String,
    pub extensions: Vec<String>,
}

/// Trả về danh sách các script tùy chỉnh của người dùng cấu hình qua định dạng .action
pub fn get_custom_actions() -> Vec<CustomAction> {
    let mut actions = Vec::new();
    let mut paths = Vec::new();

    // 1. Xác định thư mục chứa actions riêng của filen_gui
    if let Some(home) = dirs::home_dir() {
        let action_dir = home.join(".local/share/filen_gui/actions");
        // Tự động tạo thư mục nếu nó chưa từng tồn tại
        let _ = fs::create_dir_all(&action_dir);
        paths.push(action_dir);
    }

    // 2. Duyệt qua các thư mục (có thể mở rộng thêm hệ thống sau này)
    for path in paths {
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let p = entry.path();
                // Lọc lấy các file có đuôi mở rộng là .action
                if p.is_file() && p.extension().map_or(false, |ext| ext == "action") {
                    // Dịch file text thành đối tượng struct
                    if let Some(action) = parse_action_file(&p) {
                        actions.push(action);
                    }
                }
            }
        }
    }

    // Sắp xếp actions theo alphabet để hiển thị Menu đẹp mắt
    actions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    actions
}

/// Hàm nội bộ: Đọc file text cấu trúc INI để load Action Metadata
fn parse_action_file(path: &PathBuf) -> Option<CustomAction> {
    let content = fs::read_to_string(path).ok()?;
    
    // Sử dụng tên file (bỏ đuôi .action) làm ID
    let id = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut selection = "any".to_string(); // Mặc định là áp dụng cho mọi selection
    let mut extensions = Vec::new();
    
    let mut active = true; // Cờ bật/tắt của Action
    let mut in_action_entry = false; // Cờ báo hiệu đang đứng trong Block cấu hình

    for line in content.lines() {
        let line = line.trim();
        // Kiểm tra block header
        if line.starts_with('[') && line.ends_with(']') {
            // Hỗ trợ cả chuẩn [Nemo Action] cũ và chuẩn [Action] mới
            in_action_entry = line == "[Nemo Action]" || line == "[Action]";
            continue;
        }

        // Bỏ qua tất cả text nếu không nằm đúng Block
        if !in_action_entry {
            continue;
        }

        // Phân tách key=value
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            
            match key {
                "Active" if val.to_lowercase() == "false" => active = false, // Vô hiệu hoá action
                "Name" if name.is_empty() => name = val.to_string(), // Lấy Name đầu tiên tìm thấy
                "Exec" => exec = val.to_string(), // Script sẽ chạy
                "Icon" => icon = val.to_string(), // Icon hiển thị
                "Selection" => selection = val.to_lowercase(),
                "Extensions" => {
                    // Cắt chuỗi các phần mở rộng áp dụng (ví dụ: png;jpg;jpeg)
                    extensions = val.split(';')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                },
                _ => {}
            }
        }
    }

    // Chỉ kết xuất Action nếu được bật, có Tên và Lệnh chạy
    if active && !name.is_empty() && !exec.is_empty() {
        Some(CustomAction {
            id,
            name,
            exec,
            icon,
            selection,
            extensions,
        })
    } else {
        None
    }
}
