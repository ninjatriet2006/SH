//! [INTEGRITY NOTES]
//! Mục đích: Xử lý chức năng đồng bộ hóa (Sync).
//! Trách nhiệm: Đọc file cấu hình `syncPairs.json`, khởi chạy các tiến trình đồng bộ (`sync`, `sync_once`).
//! Tương tác: Giao tiếp với CLI `filen` và `models`.
//!
//! [KHỐI SYNC]


use crate::models::*;

pub fn sync_pairs() -> Result<Vec<SyncPair>, String> {
    let path = sync_pairs_path().ok_or_else(|| "Không tìm thấy thư mục dữ liệu".to_string())?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Không đọc được syncPairs.json ({}): {e}", path.display()))?;
    crate::models::parse_sync_pairs_json(&content)
}

// Chạy sync (sync <locations...> [--continuous])

pub async fn sync_terminal(
    active_account: &Option<String>,
    locations: &[String],
    continuous: bool,
) -> Result<(), String> {
    if locations.is_empty() {
        return Err("Không có cặp đồng bộ nào để chạy.".to_string());
    }
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(sync_args(locations, continuous));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 60).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Chạy sync 1 lần cho cặp local:remote (dạng `/local:/cloud`)

pub async fn sync_once_terminal(
    active_account: &Option<String>,
    local: &str,
    remote: &str,
) -> Result<(), String> {
    crate::sync::sync_terminal(active_account, &[format!("{local}:{remote}")], false).await
}

// Chạy sync 1 lần cho một pair đã đọc từ syncPairs.json

pub async fn sync_pair_once_terminal(
    active_account: &Option<String>,
    pair: &SyncPair,
) -> Result<(), String> {
    crate::sync::sync_terminal(active_account, &[crate::models::sync_pair_arg(pair)], false).await
}


