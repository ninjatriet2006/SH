//! CodeBuddy Global In-place Injection Constants & Helpers.
//!
//! Khớp chính xác với cơ chế lưu trữ của extension `tencent-cloud.coding-copilot` trong VS Code / CodeBuddy Quốc tế:
//! Secret key: `planning-genie.new.accessToken` (Không có hậu tố `cn`)

pub const EXTENSION_ID: &str = "tencent-cloud.coding-copilot";
pub const SECRET_KEY: &str = "planning-genie.new.accessToken";

/// Trả về khóa định danh lưu trữ trong SQLite `state.vscdb` (ItemTable).
pub fn secret_storage_key() -> &'static str {
    SECRET_KEY
}

/// Trả về extension ID sở hữu secret này.
pub fn extension_id() -> &'static str {
    EXTENSION_ID
}
