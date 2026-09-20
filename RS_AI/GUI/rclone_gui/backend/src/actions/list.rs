/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `list_files` thành plan thuần (`lsjson --max-depth 1`).
- Trách nhiệm: parse → build_target → chọn lệnh `lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`,
  `core::rclone::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone::build_target;
use crate::logic::file_ops::parse_remote_path;

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
    let (remote, real) = parse_remote_path(path);
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
