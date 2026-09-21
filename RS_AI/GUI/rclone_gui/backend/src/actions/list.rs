/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `list_files` thành plan thuần (`lsjson --max-depth 1`).
- Trách nhiệm: parse → build_target → chọn lệnh `lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `core::path::cut_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::api::files::FileItem;
use crate::core::rclone_caller;
use crate::logic::fastlane::fastlane;
use crate::core::rclone_caller::build_target;
use crate::logic::app_state::AppState;
use crate::core::path::cut_remote_path;
use crate::logic::watcher;
use serde::Deserialize;

/// Đặc tả thuần cho `list`: lệnh `lsjson --max-depth 1` + ghi chú watcher pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPlan {
    pub target: String,
    pub rclone_args: Vec<String>,
    /// Đường dẫn Local để pane gắn watcher/inotify; `None` trên remote.
    pub watch_path: Option<String>,
}

/// Dựng plan `list` (`lsjson --max-depth 1` + ghi chú watcher cho pane).
pub fn plan_list(path: &str) -> Result<ListPlan, String> {
    let (remote, real) = cut_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let safe = if remote == "Local" && real.is_empty() {
        "/".to_string()
    } else {
        real
    };
    let target = build_target(&remote, &safe);
    let rclone_args = match kind {
        // UNIVERSAL: cả Local và remote đều liệt kê nông qua `lsjson --max-depth 1`.
        RemoteKind::Local | RemoteKind::Remote => vec![
            "lsjson".to_string(),
            target.clone(),
            "--max-depth".to_string(),
            "1".to_string(),
        ],
    };
    let watch_path = match kind {
        // UNIVERSAL: Local gắn watcher/inotify theo thư mục pane đang xem.
        RemoteKind::Local => Some(safe),
        // UNIVERSAL: remote cloud không theo dõi được nên ngừng watch.
        RemoteKind::Remote => None,
    };
    Ok(ListPlan {
        target,
        rclone_args,
        watch_path,
    })
}

/// Rclone `lsjson` một dòng (giữ cục bộ để parse; struct dùng chung ở `api::files`).
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct RcloneFile {
    Path: String,
    Name: String,
    Size: i64,
    MimeType: String,
    ModTime: String,
    IsDir: bool,
}

/// S2: thực thi list — `plan_list` + watcher pane + `fastlane` run + parse + sort cũ.
/// UNIVERSAL: Local gắn watcher theo pane; remote cloud ngừng watch.
pub async fn execute_list(
    app_handle: tauri::AppHandle,
    path: String,
    pane: Option<String>,
) -> Result<Vec<FileItem>, String> {
    use tauri::Manager;
    let plan = plan_list(&path)?;
    if let Some(pane) = pane.as_deref() {
        let state = app_handle.state::<AppState>();
        watcher::watch_pane(&state, pane, plan.watch_path.as_deref());
    }
    let files = fastlane(move || {
        let args: Vec<&str> = plan.rclone_args.iter().map(|s| s.as_str()).collect();
        let output = rclone_caller::run_cmd(&args)?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Lỗi liệt kê file '{}': {}", plan.target, err_msg));
        }
        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() {
            return Ok(Vec::new());
        }
        let parsed: Vec<RcloneFile> = serde_json::from_str(&json_str)
            .map_err(|e| format!("Lỗi phân tích JSON rclone_files: {}", e))?;
        Ok(parsed)
    })
    .await?;

    let mut files: Vec<FileItem> = files
        .into_iter()
        .map(|f| FileItem {
            uuid: f.Path,
            name: f.Name,
            size: f.Size,
            is_dir: f.IsDir,
            mod_time: f.ModTime,
            file_type: if f.MimeType.is_empty() { None } else { Some(f.MimeType) },
        })
        .collect();
    files.sort_by(|a, b| match (b.is_dir, a.is_dir) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_local_has_watcher_remote_none() {
        let local = plan_list("Local::/tmp").expect("plan Local");
        assert_eq!(
            local.rclone_args,
            vec!["lsjson", "/tmp", "--max-depth", "1"]
        );
        assert_eq!(local.watch_path.as_deref(), Some("/tmp"));
        let remote = plan_list("GDrive::/docs").expect("plan remote");
        assert_eq!(remote.target, "GDrive:/docs");
        assert!(remote.watch_path.is_none());
    }
}
