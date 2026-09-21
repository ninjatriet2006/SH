/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_stat_advanced` thành plan thuần (`size --json`).
- Trách nhiệm: parse → build_target → chọn lệnh `size`/`lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `core::path::cut_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone_caller;
use crate::logic::fastlane::fastlane;
use crate::core::rclone_caller::build_target;
use crate::core::path::cut_remote_path;
use serde::Deserialize;
use serde::Serialize;

/// Kết quả `fs_stat_advanced` (DTO gốc ở thợ `stat`).
#[derive(Serialize)]
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
    pub target: String,
    pub size_args: Vec<String>,
    pub dirs_args: Vec<String>,
}

/// Dựng plan `stat` (`size --json` + `lsjson -R --dirs-only`).
/// UNIVERSAL: `fast_list` bật thì gắn `--fast-list` cho nhánh đếm đệ quy.
pub fn plan_stat(path: &str, fast_list: bool) -> Result<StatPlan, String> {
    let (remote, real) = cut_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let target = build_target(&remote, &real);
    let size_args = vec![target.clone(), "--json".to_string()];
    // UNIVERSAL: đếm đệ quy + fast-list khi bật cờ engine; tắt thì giữ args cũ.
    let dirs_args = match kind {
        // UNIVERSAL: cả Local và remote đều đếm thư mục con qua `lsjson -R --dirs-only`.
        RemoteKind::Local | RemoteKind::Remote => {
            let mut v = vec![target.clone(), "-R".to_string(), "--dirs-only".to_string()];
            if fast_list {
                v.push("--fast-list".to_string());
            }
            v
        }
    };
    Ok(StatPlan {
        target,
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
fn read_local_ownership(remote: &str, target: &str) -> (u32, u32, u32) {
    if remote != "Local" {
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

/// S2: thực thi stat — `plan_stat` (fast-list từ cờ engine) + `fastlane` run + parse cũ.
/// UNIVERSAL: `size --json` + đếm `lsjson -R --dirs-only`; ownership chỉ Local.
pub async fn execute_stat(path: String) -> Result<StatInfo, String> {
    let fast_list = crate::settings::engine::load_engine_flags()
        .map(|f| f.switches.fast_list)
        .unwrap_or(false);
    let plan = plan_stat(&path, fast_list)?;
    let (remote, _) = cut_remote_path(&path);
    fastlane(move || {
        let mut size_cmd = vec!["size"];
        size_cmd.extend(plan.size_args.iter().map(|s| s.as_str()));
        let output = rclone_caller::run_cmd(&size_cmd)?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        let parsed: RcloneSizeOutput = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Lỗi phân tích JSON rclone size: {}", e))?;
        let mut dirs_cmd = vec!["lsjson"];
        dirs_cmd.extend(plan.dirs_args.iter().map(|s| s.as_str()));
        let dir_count = match rclone_caller::run_cmd(&dirs_cmd) {
            Ok(out) if out.status.success() => serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
                .map(|v| v.len() as u64)
                .unwrap_or(0),
            // UNIVERSAL: đếm hỏng → warn rồi rớt về 0 như cũ.
            Ok(out) => {
                crate::core::debug::warn(None, "stat/execute_stat", format!("lsjson đếm hỏng: ok={}", out.status.success()));
                0
            }
            Err(e) => {
                crate::core::debug::warn(None, "stat/execute_stat", format!("lsjson đếm lỗi spawn: {e}"));
                0
            }
        };
        let (permissions, uid, gid) = read_local_ownership(&remote, &plan.target);
        Ok(StatInfo {
            size: parsed.bytes,
            file_count: parsed.count,
            dir_count,
            permissions,
            uid,
            gid,
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_fast_list_flag() {
        // UNIVERSAL: bật fast-list thì dirs_args có cờ, tắt thì không.
        let on = plan_stat("Local::/tmp", true).expect("plan");
        assert!(on.dirs_args.contains(&"--fast-list".to_string()));
        let off = plan_stat("Local::/tmp", false).expect("plan");
        assert!(!off.dirs_args.contains(&"--fast-list".to_string()));
    }
}
