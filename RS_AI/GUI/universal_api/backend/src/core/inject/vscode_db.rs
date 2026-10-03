//! VS Code / Electron SQLite Database (`state.vscdb`) Direct Injection.
//!
//! Đọc và ghi trực tiếp vào bảng `ItemTable` trong file `User/globalStorage/state.vscdb`.
//! Hỗ trợ nạp credentials cho CodeBuddy CN, CodeBuddy Global, GitHub Copilot, v.v.

use std::fs;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};

/// Xác định đường dẫn file `state.vscdb` từ thư mục profile `--user-data-dir`.
pub fn state_db_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("User").join("globalStorage").join("state.vscdb")
}

/// Khởi tạo SQLite connection tới `state.vscdb`, tự động tạo bảng `ItemTable` nếu chưa có.
pub fn open_state_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create dir {}: {e}", parent.display()))?;
        }
    }

    let conn = Connection::open(db_path).map_err(|e| format!("cannot open sqlite db {}: {e}", db_path.display()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY, value BLOB)",
        [],
    )
    .map_err(|e| format!("init ItemTable failed: {e}"))?;

    Ok(conn)
}

/// Đọc giá trị từ `ItemTable` theo `key`.
pub fn read_item_table_value(user_data_dir: &Path, key: &str) -> Result<Option<String>, String> {
    let db_p = state_db_path(user_data_dir);
    if !db_p.exists() {
        return Ok(None);
    }
    let conn = open_state_db(&db_p)?;
    let mut stmt = conn
        .prepare("SELECT value FROM ItemTable WHERE key = ?1")
        .map_err(|e| format!("prepare select: {e}"))?;

    let mut rows = stmt
        .query(params![key])
        .map_err(|e| format!("query select: {e}"))?;

    if let Some(row) = rows.next().map_err(|e| format!("fetch row: {e}"))? {
        let val: String = row.get(0).map_err(|e| format!("get column 0: {e}"))?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

/// Ghi đè hoặc thêm mới một key-value vào `ItemTable`.
pub fn write_item_table_value(user_data_dir: &Path, key: &str, value: &str) -> Result<(), String> {
    let db_p = state_db_path(user_data_dir);
    let conn = open_state_db(&db_p)?;

    conn.execute(
        "INSERT INTO ItemTable (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )
    .map_err(|e| format!("upsert ItemTable error: {e}"))?;

    Ok(())
}

/// Tiêm token cho CodeBuddy CN (`planning-genie.new.accessTokencn`).
pub fn inject_codebuddy_cn(user_data_dir: &Path, token: &str) -> Result<(), String> {
    use crate::core::platforms::codebuddy_cn::inject::SECRET_KEY;
    write_item_table_value(user_data_dir, SECRET_KEY, token)
}

/// Tiêm token cho CodeBuddy Global (`planning-genie.new.accessToken`).
pub fn inject_codebuddy_global(user_data_dir: &Path, token: &str) -> Result<(), String> {
    use crate::core::platforms::codebuddy_global::inject::SECRET_KEY;
    write_item_table_value(user_data_dir, SECRET_KEY, token)
}

/// Tiêm token cho GitHub Copilot.
pub fn inject_github_copilot(user_data_dir: &Path, username: &str, token: &str) -> Result<(), String> {
    let session = serde_json::json!({
        "id": format!("copilot_{}", username),
        "account": {
            "label": username,
            "id": username
        },
        "scopes": ["read:user", "user:email", "repo"],
        "accessToken": token
    });
    let sessions_array = serde_json::json!([session]);
    write_item_table_value(user_data_dir, "github.auth.sessions", &sessions_array.to_string())
}
