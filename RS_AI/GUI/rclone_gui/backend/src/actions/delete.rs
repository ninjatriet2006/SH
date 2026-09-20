/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc logic `fs_delete` (`purge`/`deletefile` + sudo fallback) thành
  `execute_delete`, cộng thêm nhánh Trash; `fs_delete` cũ giữ nguyên hành vi.
- Trách nhiệm: Phân tuyến (Route) + loại target (IsDir) + phạm vi xóa (DeleteScope) → chọn lệnh.
- Tương tác: Gọi `logic::file_ops::{parse_remote_path, run_with_sudo_fallback}`,
  `core::{rclone, task::blocking}`. `DeleteScope` dùng chung cho copy/move/delete (S2).
  Không đụng move/copy/ipc/frontend.
*/

pub use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::core::{rclone, task};
use crate::logic::file_ops;

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

/// Trial S1: thực thi xóa theo phạm vi, cùng hành vi `api::files::fs_delete` ở nhánh `NoTrash`.
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
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let route = Route::classify(&remote);
    let target = rclone::build_target(&remote, &real_path);

    // Xác định kiểu target trước (như `fs_delete`), thay vì khớp chuỗi lỗi rclone.
    let is_dir = rclone::is_dir(&target).unwrap_or(true);
    // Clone cho bước dọn rỗng sau delete chính (closure `move` đã chiếm `target`).
    let cleanup_target = target.clone();

    match (route, scope) {
        // UNIVERSAL: Local + Trash — `gio trash` đưa vào thùng rác FreeDesktop, khôi phục được.
        (Route::Local, DeleteScope::Trash) => {
            task::blocking(move || {
                let output = std::process::Command::new("gio")
                    .args(["trash", &real_path])
                    .output()
                    .map_err(|e| format!("Lỗi khi gọi gio trash: {}", e))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                }
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
            task::blocking(move || {
                let output = rclone::run_cmd(&["delete", &target])?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                }
            })
            .await?;
            // UNIVERSAL: dọn rỗng sau `delete` Trash thành công (cây rỗng còn lại).
            match empty_dirs {
                // UNIVERSAL: Keep — giữ hành vi cũ, không dọn gì thêm.
                EmptyDirs::Keep => Ok(()),
                // UNIVERSAL: chỉ dọn đúng target rỗng sau Trash (`rmdir` không đệ quy).
                EmptyDirs::OnlyHere => {
                    task::blocking(move || {
                        let output = rclone::run_cmd(&["rmdir", &cleanup_target])?;
                        if output.status.success() {
                            Ok(())
                        } else {
                            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                        }
                    })
                    .await
                }
                // UNIVERSAL: dọn đệ quy cây rỗng sau Trash (`rmdirs` leo lên cha).
                EmptyDirs::Recursive => {
                    task::blocking(move || {
                        let output = rclone::run_cmd(&["rmdirs", &cleanup_target])?;
                        if output.status.success() {
                            Ok(())
                        } else {
                            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
                        }
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
                    task::blocking(move || {
                        file_ops::run_with_sudo_fallback(&remote, "rm", std::slice::from_ref(&real_path), || {
                            let output = rclone::run_cmd(&["purge", &target])?;
                            if output.status.success() {
                                return Ok(());
                            }
                            let retry = rclone::run_cmd(&["deletefile", &target])?;
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
                    task::blocking(move || {
                        file_ops::run_with_sudo_fallback(&remote, "rm", std::slice::from_ref(&real_path), || {
                            let output = rclone::run_cmd(&["deletefile", &target])?;
                            if output.status.success() {
                                return Ok(());
                            }
                            let retry = rclone::run_cmd(&["purge", &target])?;
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
