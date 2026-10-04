//! CodeBuddy local IDE injection and import.

use super::super::storage::set_cockpit_current_account;

pub fn inject_account(platform: &str, uid: &str) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    set_cockpit_current_account(&home, platform, uid);
    Ok(format!("Đã chuyển sang tài khoản CodeBuddy {}", uid))
}
