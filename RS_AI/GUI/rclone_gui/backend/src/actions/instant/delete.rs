/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc logic `fs_delete` (`purge`/`deletefile` + sudo fallback) thành
  `execute_delete`, cộng thêm nhánh Trash; `fs_delete` cũ giữ nguyên hành vi.
- Trách nhiệm:
  + `DeletePlan` & `plan_delete`: Lập kế hoạch thuần túy (Route, DeleteScope, EmptyDirs, is_dir).
  + `execute_delete_sync`: Bản đồng bộ cho worker job/queue (NoTrash qua escalate rm).
  + `execute_delete_with_empty_dirs` / `execute_delete`: Bản bất đồng bộ qua fastlane.
  + Ghi nhận nhật ký chẩn đoán qua `core::debug`.
- Tương tác: Gọi `core::path::cut_remote_path`, `core::rclone_caller`, `actions::perm::escalate`,
  `logic::fastlane`, `core::debug`. Không đụng copy/move/ipc/frontend.
*/

pub use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::core::path::cut_remote_path;
use crate::core::rclone_caller;
use crate::logic::fastlane;

/// Tuyến xóa dùng chung từ `actions::types::RemoteKind` (`"Local"` = ổ máy, còn lại là Remote) — 1 não duy nhất.
pub use crate::actions::types::RemoteKind as Route;

/// Kế hoạch xóa thuần túy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePlan {
    pub remote: String,
    pub real_path: String,
    pub target: String,
    pub route: Route,
    pub scope: DeleteScope,
    pub empty_dirs: EmptyDirs,
    pub is_dir: bool,
}

/// Dựng plan xóa thuần túy từ đường dẫn, phạm vi và cờ dọn thư mục rỗng.
pub fn plan_delete(
    path: &str,
    scope: DeleteScope,
    empty_dirs: EmptyDirs,
    is_dir_hint: Option<bool>,
) -> Result<DeletePlan, String> {
    let (remote, real_path) = cut_remote_path(path);
    if real_path.is_empty() {
        return Err("Thiếu đường dẫn cần xóa.".to_string());
    }
    let route = Route::classify(&remote);
    let target = rclone_caller::build_target(&remote, &real_path);
    let is_dir = match is_dir_hint {
        Some(v) => v,
        None => crate::actions::types::is_dir(&target).unwrap_or(true),
    };
    Ok(DeletePlan {
        remote,
        real_path,
        target,
        route,
        scope,
        empty_dirs,
        is_dir,
    })
}

/// Thực thi xóa vĩnh viễn mức rclone/escalate cho một target (thư mục: purge, file: deletefile, fallback lẫn nhau).
fn run_delete_notrash(
    remote: &str,
    real_path: &str,
    target: &str,
    is_dir: bool,
    policy: crate::actions::perm::Policy,
) -> Result<(), String> {
    let (first, second) = if is_dir {
        ("purge", "deletefile")
    } else {
        ("deletefile", "purge")
    };
    crate::actions::perm::escalate(
        policy,
        remote,
        "rm",
        &[real_path.to_string()],
        || {
            let output = rclone_caller::run_cmd(&[first, target])?;
            if output.status.success() {
                return Ok(());
            }
            let retry = rclone_caller::run_cmd(&[second, target])?;
            if retry.status.success() {
                return Ok(());
            }
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        },
    )
}

/// UNIVERSAL: bản ĐỒNG BỘ cho worker job/queue (sync, không AppHandle/State) —
/// cùng ngữ nghĩa nhánh `NoTrash` của `execute_delete` (xóa vĩnh viễn như
/// `fs_delete`): phán đoán kiểu trước qua `is_dir` rồi `purge`/`deletefile` +
/// thử lệnh còn lại, qua `perm::escalate` với policy đã đóng dấu lúc dispatch.
pub fn execute_delete_sync(
    path: &str,
    policy: crate::actions::perm::Policy,
    is_dir: Option<bool>,
) -> Result<(), String> {
    let plan = plan_delete(path, DeleteScope::NoTrash, EmptyDirs::Keep, is_dir)?;
    let target = &plan.target;

    crate::core::debug::info(
        None,
        "actions/instant/delete",
        format!("BẮT ĐẦU DeleteSync | target='{}' is_dir={}", target, plan.is_dir),
    );
    let start = std::time::Instant::now();

    let res = run_delete_notrash(&plan.remote, &plan.real_path, target, plan.is_dir, policy);

    match &res {
        Ok(()) => {
            crate::core::debug::info(
                None,
                "actions/instant/delete",
                format!("XONG DeleteSync | target='{}' ({:.2?})", target, start.elapsed()),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                None,
                "actions/instant/delete",
                format!("LỖI DeleteSync | target='{}' | err={} ({:.2?})", target, e, start.elapsed()),
            );
        }
    }

    res
}

/// Thực thi xóa theo phạm vi (nhánh `NoTrash` = xóa vĩnh viễn như IPC `fs_delete` cũ).
pub async fn execute_delete(path: String, scope: DeleteScope) -> Result<(), String> {
    execute_delete_with_empty_dirs(path, scope, EmptyDirs::Keep).await
}

/// S1 bổ sung cờ: như [`execute_delete`], cộng dọn thư mục rỗng theo [`EmptyDirs`].
pub async fn execute_delete_with_empty_dirs(
    path: String,
    scope: DeleteScope,
    empty_dirs: EmptyDirs,
) -> Result<(), String> {
    let plan = plan_delete(&path, scope, empty_dirs, None)?;
    let target = plan.target.clone();
    let cleanup_target = target.clone();
    let real_path = plan.real_path.clone();
    let remote = plan.remote.clone();
    let is_dir = plan.is_dir;

    crate::core::debug::info(
        None,
        "actions/instant/delete",
        format!("BẮT ĐẦU Delete | target='{}' scope={:?}", target, scope),
    );
    let start = std::time::Instant::now();

    let res: Result<(), String> = match (plan.route, scope) {
        // UNIVERSAL: Local + Trash — `gio trash` đưa vào thùng rác FreeDesktop.
        (Route::Local, DeleteScope::Trash) => {
            fastlane::fastlane(move || {
                let output = std::process::Command::new("gio")
                    .args(["trash", &real_path])
                    .output()
                    .map_err(|e| format!("Lỗi khi gọi gio trash: {}", e))?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                }
                Ok(())
            })
            .await?;
            Ok(())
        }
        // UNIVERSAL: Remote + Trash — `rclone delete` đệ quy file; backend hỗ trợ trash sẽ trash.
        (Route::Remote, DeleteScope::Trash) => {
            fastlane::fastlane(move || {
                let output = rclone_caller::run_cmd(&["delete", &target])?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                }
                Ok(())
            })
            .await?;
            // Dọn rỗng sau delete Trash thành công (cây rỗng còn lại).
            match empty_dirs {
                EmptyDirs::Keep => Ok(()),
                EmptyDirs::OnlyHere => {
                    fastlane::fastlane(move || {
                        let output = rclone_caller::run_cmd(&["rmdir", &cleanup_target])?;
                        if !output.status.success() {
                            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                        }
                        Ok(())
                    })
                    .await
                }
                EmptyDirs::Recursive => {
                    fastlane::fastlane(move || {
                        let output = rclone_caller::run_cmd(&["rmdirs", &cleanup_target])?;
                        if !output.status.success() {
                            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                        }
                        Ok(())
                    })
                    .await
                }
            }
        }
        // UNIVERSAL: NoTrash — xóa vĩnh viễn (thư mục: purge, file: deletefile, fallback lẫn nhau).
        (_, DeleteScope::NoTrash) => {
            fastlane::fastlane(move || {
                run_delete_notrash(&remote, &real_path, &target, is_dir, crate::actions::perm::Policy::AllowSystem)
            })
            .await
        }
    };

    match &res {
        Ok(()) => {
            crate::core::debug::info(
                None,
                "actions/instant/delete",
                format!("XONG Delete | target='{}' ({:.2?})", plan.target, start.elapsed()),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                None,
                "actions/instant/delete",
                format!("LỖI Delete | target='{}' | err={} ({:.2?})", plan.target, e, start.elapsed()),
            );
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_delete_local_trash() {
        let p = plan_delete("Local::/tmp/file.txt", DeleteScope::Trash, EmptyDirs::Keep, Some(false))
            .expect("plan");
        assert_eq!(p.route, Route::Local);
        assert_eq!(p.target, "/tmp/file.txt");
        assert_eq!(p.scope, DeleteScope::Trash);
        assert!(!p.is_dir);
    }

    #[test]
    fn plan_delete_remote_notrash() {
        let p = plan_delete("GDrive::/folder", DeleteScope::NoTrash, EmptyDirs::Recursive, Some(true))
            .expect("plan");
        assert_eq!(p.route, Route::Remote);
        assert_eq!(p.target, "GDrive:/folder");
        assert_eq!(p.scope, DeleteScope::NoTrash);
        assert!(p.is_dir);
    }

    #[test]
    fn plan_delete_empty_path_error() {
        assert!(plan_delete("", DeleteScope::NoTrash, EmptyDirs::Keep, None).is_err());
        assert_eq!(
            plan_delete("Local::", DeleteScope::NoTrash, EmptyDirs::Keep, None).unwrap_err(),
            "Thiếu đường dẫn cần xóa."
        );
    }
}
