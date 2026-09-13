//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến xác thực (Auth).
//! Trách nhiệm: Xử lý đăng nhập, 2FA, đăng xuất, kiểm tra phiên (whoami) và quản lý danh sách account đã lưu.
//! Tương tác: Gọi trực tiếp xuống `filen_gui::auth` ở backend.

use crate::contract::{backend, success, validate, IpcResult, Req};
use filen_gui::models::{load_stored_accounts, save_stored_accounts, StoredAccount};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginReq {
    email: String,
    password: String,
    #[serde(deserialize_with = "crate::contract::present_nullable")]
    twofa_code: Option<String>,
    keep_logged: bool,
}
#[derive(Deserialize)]
pub struct TwoFaReq {
    email: String,
    password: String,
    twofa_code: String,
    keep_logged: bool,
}
#[derive(Deserialize)]
pub struct AccountReq {
    #[serde(deserialize_with = "crate::contract::present_nullable")]
    account: Option<String>,
}
#[derive(Deserialize)]
pub struct AccountsReq {
    accounts: Vec<StoredAccount>,
}
#[derive(Deserialize)]
pub struct Empty {}

/// Đăng nhập. Trả về lỗi `"2FA_REQUIRED"` khi tài khoản yêu cầu mã 2FA (dùng làm điều kiện bên frontend).
#[tauri::command]
pub async fn auth_login_terminal(request: Req<LoginReq>) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    // Chuyển cờ boolean thành chuỗi "y" hoặc "n" để tương thích với CLI của Filen
    let keep_str = if request.keep_logged { "y" } else { "n" };

    // Gọi hàm đăng nhập chính từ backend lõi
    filen_gui::auth::login_new_terminal(
        &request.email,
        &request.password,
        request.twofa_code.as_deref(), // Chuyển đổi từ Option<String> sang Option<&str>
        keep_str,
        None,
    )
    .await
    .map_err(backend)?;
    Ok(success(request_id, ()))
}

/// Xử lý bước 2 cho tài khoản yêu cầu mã 2FA: gọi lại hàm đăng nhập kèm mã xác thực.
#[tauri::command]
pub async fn auth_login_twofa_terminal(request: Req<TwoFaReq>) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    let keep_str = if request.keep_logged { "y" } else { "n" };
    filen_gui::auth::login_new_terminal(
        &request.email,
        &request.password,
        Some(&request.twofa_code),
        keep_str,
        None,
    )
    .await
    .map_err(backend)?;
    Ok(success(request_id, ()))
}

/// Đăng xuất khỏi tài khoản chỉ định.
#[tauri::command]
pub async fn auth_logout_terminal(request: Req<AccountReq>) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    filen_gui::auth::logout_terminal(&request.account)
        .await
        .map_err(backend)?;
    Ok(success(request_id, ()))
}

/// Trả về email account đang kích hoạt (None nếu chưa đăng nhập).
/// Thực hiện loại bỏ các dòng thông báo rác từ stdout của CLI Filen.
#[tauri::command]
pub async fn auth_whoami_terminal(request: Req<Empty>) -> IpcResult<Option<String>> {
    let (request_id, _) = validate(request)?;
    let email = filen_gui::auth::whoami_terminal(&None).await.map_err(backend)?;
    let email_clean = email.trim().to_string();

    // Kiểm tra xem đầu ra có bị dính thông báo rác và không phải là account ẩn danh
    if !email_clean.is_empty()
        && !email_clean.contains("Please enter")
        && !email_clean.contains("credentials")
        && email_clean != "anonymous@filen.io"
    {
        Ok(success(request_id, Some(email_clean)))
    } else {
        Ok(success(request_id, None))
    }
}

/// Lấy thông tin dung lượng sử dụng của tài khoản trên Cloud (statfs).
#[tauri::command]
pub async fn auth_statfs_terminal(request: Req<AccountReq>) -> IpcResult<(String, String)> {
    let (request_id, request) = validate(request)?;
    // Ủy quyền gọi hàm statfs từ module auth bên dưới
    let data = filen_gui::auth::statfs_terminal(&request.account)
        .await
        .map_err(backend)?;
    Ok(success(request_id, data))
}

/// Nạp danh sách tài khoản đã lưu từ file cục bộ (nhằm phục vụ tính năng đăng nhập nhanh).
#[tauri::command]
pub fn accounts_load(request: Req<Empty>) -> IpcResult<Vec<StoredAccount>> {
    let (request_id, _) = validate(request)?;
    Ok(success(request_id, load_stored_accounts()))
}

/// Lưu lại danh sách tài khoản đã đăng nhập xuống ổ cứng.
#[tauri::command]
pub fn accounts_save(request: Req<AccountsReq>) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    save_stored_accounts(&request.accounts).map_err(backend)?;
    Ok(success(request_id, ()))
}
