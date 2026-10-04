//! Cursor local IDE injection and import.

use super::super::storage::set_cockpit_current_account;

pub fn inject_account(uid: &str) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    set_cockpit_current_account(&home, "cursor", uid);
    Ok(format!("Đã chuyển sang tài khoản Cursor {}", uid))
}
