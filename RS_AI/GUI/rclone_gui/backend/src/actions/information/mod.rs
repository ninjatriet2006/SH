//! Nhóm Information Actions: tất cả các thao tác ĐỌC / KHẢO SÁT / TRA CỨU (100% Read-Only).
//! Bọc qua fastlane để chạy song song trên threadpool, không block Tokio runtime.

pub mod about;
pub mod conflicts;
pub mod hash;
pub mod list;
pub mod search;
pub mod size;
pub mod stat;
pub mod trash_list;
pub mod view;

pub use about::{AboutPlan, about, execute_about, plan_about};
pub use conflicts::{ConflictInfo, PreJobConflictInfo, check_conflicts, check_rename_conflict};
pub use hash::{IntegrityCheckResult, execute_check_integrity, execute_hashsum, get_supported_hashes};
pub use list::{FileItem, ListPlan, execute_list, plan_list};
pub use search::{SearchPlan, SearchResultItem, execute_search, plan_search};
pub use size::{SizePlan, execute_size, plan_size, size};
pub use stat::{StatInfo, StatPlan, execute_stat, plan_stat};
pub use trash_list::{
    Route as TrashRoute, TrashItemLocal, list_local as trash_list_local,
    list_remote as trash_list_remote,
};
pub use view::{
    ThumbnailGroup, ThumbnailPlan, ViewPlan, execute_thumbnail, plan_thumbnail,
    plan_view_download, sys_open_with,
};

use crate::core::rclone_caller;
use serde_json::Value;

/// Helper chung thực thi truy vấn rclone dạng JSON (`about`, `size`),
/// đo lường thời gian và ghi nhận log chẩn đoán qua `core::debug`.
pub(crate) fn run_json_query(cmd: &str, target: &str) -> Result<Value, String> {
    crate::core::debug::info(
        None,
        &format!("actions/information/{cmd}"),
        format!("BẮT ĐẦU {} | target='{}'", cmd, target),
    );
    let start = std::time::Instant::now();

    let res = (|| -> Result<Value, String> {
        let output = rclone_caller::run_cmd(&[cmd, target, "--json"])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("Lỗi rclone {cmd}: {err_msg}"));
        }
        let json_str = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&json_str).map_err(|e| format!("Lỗi phân tích JSON {cmd}: {e}"))
    })();

    match &res {
        Ok(_) => {
            crate::core::debug::info(
                None,
                &format!("actions/information/{cmd}"),
                format!("XONG {} | target='{}' ({:.2?})", cmd, target, start.elapsed()),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                None,
                &format!("actions/information/{cmd}"),
                format!("LỖI {} | target='{}' | err={} ({:.2?})", cmd, target, e, start.elapsed()),
            );
        }
    }

    res
}

/// Chuẩn hoá đường dẫn cục bộ (hỗ trợ mở rộng `~` thành thư mục `$HOME`).
pub(crate) fn expand_local_path(path: &str) -> String {
    if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{home}/{rest}")
    } else {
        path.to_string()
    }
}

use crate::actions::types::RemoteKind;
use crate::core::path::cut_remote_path;

/// Kết quả chuẩn hóa đường dẫn phân tuyến cho nhóm information (`about`, `size`, `stat`, `hash`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInfo {
    pub remote: String,
    pub real_path: String,
    pub target: String,
    pub route: RemoteKind,
}

/// Chuẩn hoá đường dẫn 4 nhánh dùng chung:
/// 1. Cặp Remote::path hoặc Local::path
/// 2. "Local" hoặc "Local:"
/// 3. Đường dẫn cục bộ bắt đầu bằng /, ., ~
/// 4. Remote trần (kết thúc hoặc không kết thúc bằng :)
pub fn resolve_target(input: &str) -> Result<TargetInfo, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Thiếu tên remote hoặc đường dẫn.".to_string());
    }

    let (remote, real_path) = if trimmed.contains("::") {
        let (r, p) = cut_remote_path(trimmed);
        if r == "Local" {
            ("Local".to_string(), expand_local_path(&p))
        } else {
            (r, p)
        }
    } else if trimmed == "Local" || trimmed == "Local:" {
        ("Local".to_string(), "/".to_string())
    } else if trimmed.starts_with('/') || trimmed.starts_with('.') || trimmed.starts_with('~') {
        ("Local".to_string(), expand_local_path(trimmed))
    } else {
        let clean = trimmed.trim_end_matches(':');
        (clean.to_string(), String::new())
    };

    let route = RemoteKind::classify(&remote);
    let target = match route {
        RemoteKind::Local => {
            if real_path.is_empty() {
                "/".to_string()
            } else {
                real_path.clone()
            }
        }
        RemoteKind::Remote => {
            if real_path.is_empty() {
                format!("{remote}:")
            } else {
                rclone_caller::build_target(&remote, &real_path)
            }
        }
    };

    Ok(TargetInfo {
        remote,
        real_path,
        target,
        route,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_local_path_tilde() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(expand_local_path("~"), home);
        assert_eq!(expand_local_path("~/test"), format!("{home}/test"));
        assert_eq!(expand_local_path("/tmp/test"), "/tmp/test");
        assert_eq!(expand_local_path("relative/path"), "relative/path");
    }
}
