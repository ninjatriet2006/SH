//! Nhóm Information Actions: kiểm tra dung lượng (`about`) và kích thước (`size`).
//! Bọc qua fastlane tại `api::remote_manager`.

pub mod about;
pub mod hash;
pub mod size;
pub mod stat;

pub use about::{AboutPlan, about, execute_about, plan_about};
pub use hash::{IntegrityCheckResult, execute_check_integrity, execute_hashsum, get_supported_hashes};
pub use size::{SizePlan, execute_size, plan_size, size};
pub use stat::{StatInfo, StatPlan, execute_stat, plan_stat};

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
