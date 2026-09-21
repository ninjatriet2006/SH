/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc logic `fs_delete` (`purge`/`deletefile` + sudo fallback) thành
  `execute_delete`, cộng thêm nhánh Trash; `fs_delete` cũ giữ nguyên hành vi.
- Trách nhiệm: Phân tuyến (Route) + loại target (IsDir) + phạm vi xóa (DeleteScope) → chọn lệnh.
- Tương tác: Gọi `core::path::cut_remote_path` + `actions::perm::escalate`,
  `core::rclone_caller + logic::fastlane::fastlane`. `DeleteScope` dùng chung cho copy/move/delete (S2).
  Không đụng move/copy/ipc/frontend.
*/

pub use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::core::rclone_caller;
use crate::logic::fastlane;
use crate::core::path::cut_remote_path;

/// Tuyến xóa, suy từ remote chứa target (`"Local"` = ổ máy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Local,
    Remote,
}

impl Route {
    /// Phân tuyến từ tên remote đã parse.
    pub fn classify(remote: &str) -> Self {
        // UNIVERSAL: ổ máy xóa qua syscall/`gio`; remote xóa qua lệnh rclone.
        if remote == "Local" {
            Self::Local
        } else {
            Self::Remote
        }
    }
}

/// UNIVERSAL: bản ĐỒNG BỘ cho worker job/queue (sync, không AppHandle/State) —
/// cùng ngữ nghĩa nhánh `NoTrash` của `execute_delete` (xóa vĩnh viễn như
/// `fs_delete`): phán đoán kiểu trước qua `is_dir` rồi `purge`/`deletefile` +
/// thử lệnh còn lại, qua `perm::escalate` với policy đã đóng dấu lúc dispatch.
pub fn execute_delete_sync(path: &str, policy: crate::actions::perm::Policy) -> Result<(), String> {
    let (remote, real_path) = cut_remote_path(path);
    let target = rclone_caller::build_target(&remote, &real_path);
    // UNIVERSAL: chạm Tier route để giữ một nguồn sự thật về tuyến xóa.
    let _ = Route::classify(&remote);
    let is_dir = crate::actions::types::is_dir(&target).unwrap_or(true);
    // UNIVERSAL: thư mục `purge` trước, file `deletefile` trước; rớt qua lệnh
    // còn lại khi phán đoán kiểu sai (giữ đúng thứ tự `fs_delete` cũ).
    let (first, second) = if is_dir {
        ("purge", "deletefile")
    } else {
        ("deletefile", "purge")
    };
    crate::actions::perm::escalate(policy, &remote, "rm", std::slice::from_ref(&real_path), || {
        let output = rclone_caller::run_cmd(&[first, &target])?;
        if output.status.success() {
            return Ok(());
        }
        let retry = rclone_caller::run_cmd(&[second, &target])?;
        if retry.status.success() {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    })
}
/// Thực thi xóa theo phạm vi (nhánh `NoTrash` = xóa vĩnh viễn như IPC `fs_delete` cũ).
///
/// Không đổi cờ rclone; chọn lệnh bằng `match` trên ([`Route`], IsDir, [`DeleteScope`]).
/// Giữ tương thích cũ: mặc định [`EmptyDirs::Keep`] (không dọn thư mục rỗng).
pub async fn execute_delete(path: String, scope: DeleteScope) -> Result<(), String> {
    execute_delete_with_empty_dirs(path, scope, EmptyDirs::Keep).await
}

/// S1 bổ sung cờ: như [`execute_delete`], cộng dọn thư mục rỗng theo [`EmptyDirs`].
///
/// Không đổi cờ rclone; dọn rỗng bằng `match` trên [`EmptyDirs`].
pub async fn execute_delete_with_empty_dirs(
    path: String,
    scope: DeleteScope,
    empty_dirs: EmptyDirs,
) -> Result<(), String> {
    let (remote, real_path) = cut_remote_path(&path);
    let route = Route::classify(&remote);
    let target = rclone_caller::build_target(&remote, &real_path);

    // Xác định kiểu target trước (như `fs_delete`), thay vì khớp chuỗi lỗi rclone.
    let is_dir = crate::actions::types::is_dir(&target).unwrap_or(true);
    // Clone cho bước dọn rỗng sau delete chính (closure `move` đã chiếm `target`).
    let cleanup_target = target.clone();

    match (route, scope) {
        // UNIVERSAL: Local + Trash — `gio trash` đưa vào thùng rác FreeDesktop, khôi phục được.
        (Route::Local, DeleteScope::Trash) => {
            fastlane::fastlane(move || {
                let output = std::process::Command::new("gio")
                    .args(["trash", &real_path])
                    .output()
                    .map_err(|e| format!("Lỗi khi gọi gio trash: {}", e))?;
                // UNIVERSAL: guard sớm, phẳng else lồng.
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                }
                Ok(())
            })
            .await?;
            // UNIVERSAL: Local + Trash via `gio` đã dời cả thư mục nên skip dọn rỗng.
            match empty_dirs {
                // UNIVERSAL: Keep — giữ hành vi cũ, không dọn gì thêm.
                EmptyDirs::Keep => Ok(()),
                // UNIVERSAL: Local + Trash đã dời cả thư mục qua `gio` nên skip `rmdir`.
                EmptyDirs::OnlyHere => Ok(()),
                // UNIVERSAL: Local + Trash đã dời cả thư mục qua `gio` nên skip `rmdirs`.
                EmptyDirs::Recursive => Ok(()),
            }
        }
        // UNIVERSAL: Remote + Trash — `rclone delete` đệ quy file; backend hỗ trợ
        // trash (vd. Drive) sẽ trash thay vì xóa hẳn. Cây thư mục rỗng có thể còn lại.
        (Route::Remote, DeleteScope::Trash) => {
            fastlane::fastlane(move || {
                let output = rclone_caller::run_cmd(&["delete", &target])?;
                // UNIVERSAL: guard sớm, phẳng else lồng.
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                }
                Ok(())
            })
            .await?;
            // UNIVERSAL: dọn rỗng sau `delete` Trash thành công (cây rỗng còn lại).
            match empty_dirs {
                // UNIVERSAL: Keep — giữ hành vi cũ, không dọn gì thêm.
                EmptyDirs::Keep => Ok(()),
                // UNIVERSAL: chỉ dọn đúng target rỗng sau Trash (`rmdir` không đệ quy).
                EmptyDirs::OnlyHere => {
                    fastlane::fastlane(move || {
                        let output = rclone_caller::run_cmd(&["rmdir", &cleanup_target])?;
                        // UNIVERSAL: guard sớm, phẳng else lồng.
                        if !output.status.success() {
                            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                        }
                        Ok(())
                    })
                    .await
                }
                // UNIVERSAL: dọn đệ quy cây rỗng sau Trash (`rmdirs` leo lên cha).
                EmptyDirs::Recursive => {
                    fastlane::fastlane(move || {
                        let output = rclone_caller::run_cmd(&["rmdirs", &cleanup_target])?;
                        // UNIVERSAL: guard sớm, phẳng else lồng.
                        if !output.status.success() {
                            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
                        }
                        Ok(())
                    })
                    .await
                }
            }
        }
        // UNIVERSAL: NoTrash — xóa vĩnh viễn như `fs_delete`: thư mục qua `purge`,
        // file qua `deletefile`, thử lệnh còn lại khi phán đoán kiểu sai; Local có sudo fallback.
        (route, DeleteScope::NoTrash) => {
            let _ = route;
            match is_dir {
                // UNIVERSAL: thư mục — `purge` xóa đệ quy; rớt qua `deletefile` nếu đoán sai kiểu.
                true => {
                    fastlane::fastlane(move || {
                        // UNIVERSAL: hành vi cũ = AllowSystem (tự pkexec khi lỗi quyền Local).
                        crate::actions::perm::escalate(
                            crate::actions::perm::Policy::AllowSystem,
                            &remote,
                            "rm",
                            std::slice::from_ref(&real_path),
                            || {
                            let output = rclone_caller::run_cmd(&["purge", &target])?;
                            if output.status.success() {
                                return Ok(());
                            }
                            let retry = rclone_caller::run_cmd(&["deletefile", &target])?;
                            if retry.status.success() {
                                return Ok(());
                            }
                            Err(String::from_utf8_lossy(&output.stderr).into_owned())
                        })
                    })
                    .await?;
                    // UNIVERSAL: NoTrash + `purge` đã xóa sạch cả thư mục nên skip `rmdir`/`rmdirs`.
                    match empty_dirs {
                        // UNIVERSAL: Keep — giữ hành vi cũ, không dọn gì thêm.
                        EmptyDirs::Keep => Ok(()),
                        // UNIVERSAL: `purge` đã sạch nên skip `rmdir`.
                        EmptyDirs::OnlyHere => Ok(()),
                        // UNIVERSAL: `purge` đã sạch nên skip `rmdirs`.
                        EmptyDirs::Recursive => Ok(()),
                    }
                }
                // UNIVERSAL: file — `deletefile` xóa đúng một file; rớt qua `purge` nếu đoán sai kiểu.
                false => {
                    fastlane::fastlane(move || {
                        // UNIVERSAL: hành vi cũ = AllowSystem (tự pkexec khi lỗi quyền Local).
                        crate::actions::perm::escalate(
                            crate::actions::perm::Policy::AllowSystem,
                            &remote,
                            "rm",
                            std::slice::from_ref(&real_path),
                            || {
                            let output = rclone_caller::run_cmd(&["deletefile", &target])?;
                            if output.status.success() {
                                return Ok(());
                            }
                            let retry = rclone_caller::run_cmd(&["purge", &target])?;
                            if retry.status.success() {
                                return Ok(());
                            }
                            Err(String::from_utf8_lossy(&output.stderr).into_owned())
                        })
                    })
                    .await?;
                    // UNIVERSAL: NoTrash xóa file đơn không để lại cây rỗng của `delete` nên skip.
                    let _ = cleanup_target;
                    match empty_dirs {
                        // UNIVERSAL: Keep — giữ hành vi cũ, không dọn gì thêm.
                        EmptyDirs::Keep => Ok(()),
                        // UNIVERSAL: NoTrash file đơn nên skip `rmdir` (cha có thể không rỗng).
                        EmptyDirs::OnlyHere => Ok(()),
                        // UNIVERSAL: NoTrash file đơn nên skip `rmdirs` (cha có thể không rỗng).
                        EmptyDirs::Recursive => Ok(()),
                    }
                }
            }
        }
    }
}
