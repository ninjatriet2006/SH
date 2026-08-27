/* 
[INTEGRITY NOTES]
- Mục đích: Xử lý các thao tác liên quan đến xác thực tài khoản và kiểm tra dung lượng (Stat FS).
- Trách nhiệm: Cung cấp API cho AuthModal (đăng nhập) và ServersDashboard (kiểm tra dung lượng lưu trữ).
- Tương tác: Trả về dữ liệu cho frontend thông qua Tauri invoke. Dùng `utils` để gọi rclone.
*/

use std::process::Command;
use crate::core::rclone; // Sử dụng helper từ thư viện dùng chung

// ====================================================================================
// BLOCK: XÁC THỰC TÀI KHOẢN
// ====================================================================================

/// Tên hàm: auth_login_terminal
/// Mô tả: Đăng nhập bằng email và mật khẩu (Thường dùng cho WebDAV, Mega, v.v...)
#[tauri::command]
pub async fn auth_login_terminal(_email: String, _password: String, _twofa_code: Option<String>, _keep_logged: bool) -> Result<(), String> {
    // Tạm thời trả về lỗi vì việc xử lý thông tin đăng nhập tự động rclone khá phức tạp
    // và cần định cấu hình (rclone config create)
    Err("Đăng nhập trực tiếp chưa được hỗ trợ. Vui lòng cấu hình qua rclone config.".to_string())
}

/// Tên hàm: auth_login_twofa_terminal
/// Mô tả: Đăng nhập kết hợp mã bảo mật 2FA
#[tauri::command]
pub async fn auth_login_twofa_terminal(_email: String, _password: String, _twofa_code: String, _keep_logged: bool) -> Result<(), String> {
    Err("Đăng nhập 2FA trực tiếp chưa được hỗ trợ.".to_string())
}

// ====================================================================================
// BLOCK: KIỂM TRA DUNG LƯỢNG (STAT FS)
// ====================================================================================

/// Tên hàm: auth_statfs_terminal
/// Mô tả: Lấy thông tin dung lượng lưu trữ của một Remote. Trả về mảng (tuple) gồm [Đã dùng, Tổng cộng].
#[tauri::command]
pub async fn auth_statfs_terminal(account: Option<String>) -> Result<(String, String), String> {
    let remote_name = account.unwrap_or_else(|| "Local".to_string());
    
    // Nếu là Local thì lấy thông tin dung lượng đĩa gốc của hệ điều hành Linux (bằng lệnh df)
    if remote_name == "Local" {
        let output = Command::new("df")
            .arg("-h")
            .arg("/")
            .output()
            .map_err(|e| format!("Lỗi hệ thống khi gọi df: {}", e))?;
        
        let out_str = String::from_utf8_lossy(&output.stdout);
        // Định dạng đầu ra của df -h /:
        // Filesystem      Size  Used Avail Use% Mounted on
        // /dev/sda1        50G   20G   30G  40% /
        let lines: Vec<&str> = out_str.lines().collect();
        if lines.len() > 1 {
            let cols: Vec<&str> = lines[1].split_whitespace().collect();
            if cols.len() >= 4 {
                let total = cols[1].to_string();
                let used = cols[2].to_string();
                return Ok((used, total));
            }
        }
        return Ok(("0B".to_string(), "0B".to_string()));
    }
    
    // Nếu là Cloud remote, sử dụng `rclone about`
    let target = format!("{}:", remote_name);
    let output = rclone::run_cmd(&["about", &target, "--json"])
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let out_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&out_str) {
            let used_bytes = json["used"].as_u64().unwrap_or(0);
            let total_bytes = json["total"].as_u64().unwrap_or(0);
            
            // Định dạng chuỗi (VD: 1.5 GB)
            let used_str = format_size(used_bytes);
            let total_str = format_size(total_bytes);
            
            return Ok((used_str, total_str));
        }
    }
    
    Ok(("Không rõ".to_string(), "Không rõ".to_string()))
}

/// Tên hàm: format_size
/// Hàm phụ trợ: Format bytes sang chuỗi dễ đọc (giống format.ts bên frontend)
fn format_size(bytes: u64) -> String {
    if bytes == 0 { return "0 B".to_string(); }
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut i = 0;
    while value >= 1024.0 && i < units.len() - 1 {
        value /= 1024.0;
        i += 1;
    }
    format!("{:.2} {}", value, units[i])
}
