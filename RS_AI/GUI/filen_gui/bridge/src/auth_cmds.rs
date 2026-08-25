//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến xác thực (Auth).
//! Trách nhiệm: Xử lý đăng nhập, 2FA, đăng xuất, kiểm tra phiên (whoami) và quản lý danh sách account đã lưu.
//! Tương tác: Gọi trực tiếp xuống `filen_gui::auth` ở backend.

use filen_gui::models::{StoredAccount, load_stored_accounts, save_stored_accounts};

/// Đăng nhập. Trả về lỗi `"2FA_REQUIRED"` khi tài khoản yêu cầu mã 2FA (dùng làm điều kiện bên frontend).
#[tauri::command]
pub async fn auth_login_terminal(
    email: String,
    password: String,
    twofa_code: Option<String>,
    keep_logged: bool,
) -> Result<(), String> {
    // Chuyển cờ boolean thành chuỗi "y" hoặc "n" để tương thích với CLI của Filen
    let keep_str = if keep_logged { "y" } else { "n" };
    
    // Gọi hàm đăng nhập chính từ backend lõi
    filen_gui::auth::login_new_terminal(
        &email,
        &password,
        twofa_code.as_deref(), // Chuyển đổi từ Option<String> sang Option<&str>
        keep_str,
        None,
    )
    .await
}

/// Xử lý bước 2 cho tài khoản yêu cầu mã 2FA: gọi lại hàm đăng nhập kèm mã xác thực.
#[tauri::command]
pub async fn auth_login_twofa_terminal(
    email: String,
    password: String,
    twofa_code: String,
    keep_logged: bool,
) -> Result<(), String> {
    let keep_str = if keep_logged { "y" } else { "n" };
    filen_gui::auth::login_new_terminal(&email, &password, Some(&twofa_code), keep_str, None).await
}

/// Đăng xuất khỏi tài khoản chỉ định.
#[tauri::command]
pub async fn auth_logout_terminal(account: Option<String>) -> Result<(), String> {
    filen_gui::auth::logout_terminal(&account).await
}

/// Trả về email account đang kích hoạt (None nếu chưa đăng nhập).
/// Thực hiện loại bỏ các dòng thông báo rác từ stdout của CLI Filen.
#[tauri::command]
pub async fn auth_whoami_terminal() -> Result<Option<String>, String> {
    let email = filen_gui::auth::whoami_terminal(&None).await?;
    let email_clean = email.trim().to_string();
    
    // Kiểm tra xem đầu ra có bị dính thông báo rác và không phải là account ẩn danh
    if !email_clean.is_empty()
        && !email_clean.contains("Please enter")
        && !email_clean.contains("credentials")
        && email_clean != "anonymous@filen.io"
    {
        Ok(Some(email_clean))
    } else {
        Ok(None)
    }
}

/// Lấy thông tin dung lượng sử dụng của tài khoản trên Cloud (statfs).
#[tauri::command]
pub async fn auth_statfs_terminal(
    account: Option<String>,
) -> Result<(String, String), String> {
    // Ủy quyền gọi hàm statfs từ module auth bên dưới
    filen_gui::auth::statfs_terminal(&account).await
}

/// Nạp danh sách tài khoản đã lưu từ file cục bộ (nhằm phục vụ tính năng đăng nhập nhanh).
#[tauri::command]
pub fn accounts_load() -> Vec<StoredAccount> {
    load_stored_accounts()
}

/// Lưu lại danh sách tài khoản đã đăng nhập xuống ổ cứng.
#[tauri::command]
pub fn accounts_save(accounts: Vec<StoredAccount>) -> Result<(), String> {
    save_stored_accounts(&accounts)
}
