/*
[INTEGRITY NOTES]
- Mục đích: Đo trạng thái chi tiết file/thư mục qua `rclone size --json` + `lsjson -R --dirs-only`.
- Trách nhiệm:
  + `StatPlan` & `plan_stat`: Lập kế hoạch thuần túy (chuẩn hóa target, route, tilde expansion, flags).
  + `execute_stat`: Thực thi đồng bộ trong fastlane, đọc metadata Unix cho Local, ghi log qua `core::debug`.
- Tương tác: Được gọi bởi `api::files_view` (`fs_stat_advanced`).
*/

use crate::actions::types::RemoteKind;
use crate::core::rclone_caller;
use crate::logic::fastlane::fastlane;
use serde::{Deserialize, Serialize};

/// Kết quả `fs_stat_advanced` (DTO gốc ở `stat`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatInfo {
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub permissions: u32,
    pub uid: u32,
    pub gid: u32,
}

/// Đặc tả thuần cho `stat`: lệnh `size --json` + đếm thư mục `lsjson`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatPlan {
    pub remote: String,
    pub real_path: String,
    pub target: String,
    pub route: RemoteKind,
    pub size_args: Vec<String>,
    pub dirs_args: Vec<String>,
}

/// Dựng plan `stat` (`size --json` + `lsjson -R --dirs-only`).
/// UNIVERSAL: `fast_list` bật thì gắn `--fast-list` cho nhánh đếm đệ quy.
pub fn plan_stat(path: &str, fast_list: bool) -> Result<StatPlan, String> {
    let info = super::resolve_target(path)?;

    let size_args = vec![info.target.clone(), "--json".to_string()];
    let mut dirs_args = vec![info.target.clone(), "-R".to_string(), "--dirs-only".to_string()];
    if fast_list {
        dirs_args.push("--fast-list".to_string());
    }

    Ok(StatPlan {
        remote: info.remote,
        real_path: info.real_path,
        target: info.target,
        route: info.route,
        size_args,
        dirs_args,
    })
}

#[derive(Deserialize)]
struct RcloneSizeOutput {
    count: u64,
    bytes: u64,
}

/// Đọc mode/uid/gid thật của đường dẫn Local; (0,0,0) trên remote/non-unix.
fn read_local_ownership(route: RemoteKind, target: &str) -> (u32, u32, u32) {
    if route != RemoteKind::Local {
        return (0, 0, 0);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(meta) = std::fs::metadata(target) {
            return (meta.mode(), meta.uid(), meta.gid());
        }
    }
    #[cfg(not(unix))]
    let _ = target;
    (0, 0, 0)
}

/// Thực thi stat — `plan_stat` (fast-list từ cờ engine) + `fastlane` run + parse output.
/// UNIVERSAL: `size --json` + đếm `lsjson -R --dirs-only`; ownership chỉ Local.
pub async fn execute_stat(path: String) -> Result<StatInfo, String> {
    let fast_list = crate::settings::engine::load_engine_flags()
        .map(|f| f.switches.fast_list)
        .unwrap_or(false);
    let plan = plan_stat(&path, fast_list)?;

    crate::core::debug::info(
        None,
        "actions/information/stat",
        format!("BẮT ĐẦU Stat | target='{}'", plan.target),
    );
    let start = std::time::Instant::now();

    let plan_clone = plan.clone();
    let res = fastlane(move || {
        let mut size_cmd = vec!["size"];
        size_cmd.extend(plan_clone.size_args.iter().map(|s| s.as_str()));
        let output = rclone_caller::run_cmd(&size_cmd)?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("Lỗi rclone size: {err_msg}"));
        }
        let parsed: RcloneSizeOutput = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Lỗi phân tích JSON rclone size: {e}"))?;

        let mut dirs_cmd = vec!["lsjson"];
        dirs_cmd.extend(plan_clone.dirs_args.iter().map(|s| s.as_str()));
        let dir_count = match rclone_caller::run_cmd(&dirs_cmd) {
            Ok(out) if out.status.success() => {
                serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
                    .map(|v| v.len() as u64)
                    .unwrap_or(0)
            }
            Ok(out) => {
                crate::core::debug::warn(
                    None,
                    "actions/information/stat",
                    format!("lsjson đếm thư mục hỏng: ok={}", out.status.success()),
                );
                0
            }
            Err(e) => {
                crate::core::debug::warn(
                    None,
                    "actions/information/stat",
                    format!("lsjson đếm thư mục lỗi spawn: {e}"),
                );
                0
            }
        };

        let (permissions, uid, gid) = read_local_ownership(plan_clone.route, &plan_clone.target);
        Ok(StatInfo {
            size: parsed.bytes,
            file_count: parsed.count,
            dir_count,
            permissions,
            uid,
            gid,
        })
    })
    .await;

    match &res {
        Ok(info) => {
            crate::core::debug::info(
                None,
                "actions/information/stat",
                format!(
                    "XONG Stat | target='{}' size={} files={} dirs={} ({:.2?})",
                    plan.target, info.size, info.file_count, info.dir_count, start.elapsed()
                ),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                None,
                "actions/information/stat",
                format!("LỖI Stat | target='{}' | err={} ({:.2?})", plan.target, e, start.elapsed()),
            );
        }
    }

    res
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
        let dir = std::env::temp_dir().join(format!("rclone_gui_stat_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("gieo thu muc");
        std::fs::create_dir_all(dir.join("sub")).expect("gieo thu muc con");
        std::fs::write(dir.join("a.txt"), b"12345").expect("gieo file");
        dir
    }

    #[test]
    fn stat_fast_list_flag() {
        let on = plan_stat("Local::/tmp", true).expect("plan");
        assert!(on.dirs_args.contains(&"--fast-list".to_string()));
        let off = plan_stat("Local::/tmp", false).expect("plan");
        assert!(!off.dirs_args.contains(&"--fast-list".to_string()));
    }

    #[test]
    fn plan_stat_empty_returns_error() {
        assert!(plan_stat("", false).is_err());
        assert!(plan_stat("   ", false).is_err());
    }

    #[test]
    fn plan_stat_expands_tilde() {
        let plan = plan_stat("~/MyFolder", false).expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert!(!plan.target.starts_with('~'));
        assert!(plan.target.ends_with("/MyFolder"));

        let plan_colon = plan_stat("Local::~/MyFolder", false).expect("plan");
        assert_eq!(plan_colon.route, RemoteKind::Local);
        assert!(!plan_colon.target.starts_with('~'));
        assert!(plan_colon.target.ends_with("/MyFolder"));
    }

    #[test]
    fn plan_stat_resolves_local_root() {
        let plan = plan_stat("Local", false).expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert_eq!(plan.target, "/");

        let plan_colon = plan_stat("Local:", false).expect("plan");
        assert_eq!(plan_colon.target, "/");
    }

    #[test]
    fn plan_stat_cloud_remote() {
        let plan = plan_stat("GDrive", false).expect("plan");
        assert_eq!(plan.route, RemoteKind::Remote);
        assert_eq!(plan.target, "GDrive:");

        let plan2 = plan_stat("GDrive::/Docs", false).expect("plan");
        assert_eq!(plan2.target, "GDrive:/Docs");
    }

    #[tokio::test]
    async fn live_stat_local_counts() {
        if !rclone_present() {
            return;
        }
        let dir = seed_dir("live");
        let info = execute_stat(dir.to_string_lossy().to_string()).await.expect("stat");
        assert_eq!(info.file_count, 1);
        assert_eq!(info.dir_count, 1);
        assert_eq!(info.size, 5);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
