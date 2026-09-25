/*
[INTEGRITY NOTES]
- Mục đích: Đo kích thước thư mục/remote (tổng số file, tổng byte) qua `rclone size --json`.
- Trách nhiệm:
  + `SizePlan` & `plan_size`: Lập kế hoạch thuần túy (chuẩn hóa target, route).
  + `execute_size` / `size`: Thực thi gọi rclone, ghi log chẩn đoán qua `core::debug`.
- Tương tác: Được gọi bởi `api::remote_manager`, không đụng frontend.
*/

use crate::actions::types::RemoteKind;
use crate::core::path::cut_remote_path;
use crate::core::rclone_caller;
use serde_json::Value;

/// Kế hoạch đo kích thước thuần túy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizePlan {
    pub remote: String,
    pub route: RemoteKind,
    pub target: String,
}

/// Chuẩn hoá target và lập kế hoạch đo kích thước thuần túy.
pub fn plan_size(input: &str) -> Result<SizePlan, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Thiếu tên remote hoặc đường dẫn để kiểm tra kích thước.".to_string());
    }

    let (remote, path) = if trimmed.contains("::") {
        let (r, p) = cut_remote_path(trimmed);
        if r == "Local" {
            ("Local".to_string(), super::expand_local_path(&p))
        } else {
            (r, p)
        }
    } else if trimmed == "Local" || trimmed == "Local:" {
        ("Local".to_string(), "/".to_string())
    } else if trimmed.starts_with('/') || trimmed.starts_with('.') || trimmed.starts_with('~') {
        ("Local".to_string(), super::expand_local_path(trimmed))
    } else {
        let clean = trimmed.trim_end_matches(':');
        (clean.to_string(), String::new())
    };

    let route = RemoteKind::classify(&remote);
    let target = match route {
        RemoteKind::Local => {
            if path.is_empty() {
                "/".to_string()
            } else {
                path
            }
        }
        RemoteKind::Remote => {
            if path.is_empty() {
                format!("{remote}:")
            } else {
                rclone_caller::build_target(&remote, &path)
            }
        }
    };

    Ok(SizePlan {
        remote,
        route,
        target,
    })
}

/// Đo kích thước hệ thống/remote qua `rclone size --json`.
pub fn execute_size(input: &str) -> Result<Value, String> {
    let plan = plan_size(input)?;
    super::run_json_query("size", &plan.target)
}

/// Alias tương thích.
pub fn size(remote: &str) -> Result<Value, String> {
    execute_size(remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rclone_present() -> bool {
        crate::core::rclone_caller::run_cmd(&["version"])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn seed_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rclone_gui_size_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("gieo thu muc");
        std::fs::write(dir.join("a.txt"), b"12345").expect("gieo file");
        std::fs::write(dir.join("b.txt"), b"1234567890").expect("gieo file");
        dir
    }

    #[test]
    fn plan_size_resolves_local_path() {
        let plan = plan_size("Local::/tmp/test").expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert_eq!(plan.target, "/tmp/test");
    }

    #[test]
    fn plan_size_expands_tilde() {
        let plan = plan_size("~/test").expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert!(!plan.target.starts_with('~'));
        assert!(plan.target.ends_with("/test"));
    }

    #[test]
    fn plan_size_resolves_cloud_remote_colon() {
        let plan = plan_size("GDrive").expect("plan");
        assert_eq!(plan.route, RemoteKind::Remote);
        assert_eq!(plan.target, "GDrive:");

        let plan2 = plan_size("GDrive:").expect("plan");
        assert_eq!(plan2.target, "GDrive:");
    }

    #[test]
    fn plan_size_empty_returns_error() {
        assert!(plan_size("").is_err());
        assert!(plan_size("   ").is_err());
    }

    #[test]
    fn live_size_local_counts_entries() {
        if !rclone_present() {
            return;
        }
        let dir = seed_dir("test");
        let v = size(&dir.to_string_lossy()).expect("size thu muc gieo");
        let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
        let bytes = v.get("bytes").and_then(|x| x.as_u64()).unwrap_or(0);
        assert_eq!(count, 2, "gieo đúng 2 file");
        assert_eq!(bytes, 15, "5 + 10 byte");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
