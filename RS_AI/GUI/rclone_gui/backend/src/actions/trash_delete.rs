/*
[INTEGRITY NOTES]
- Mục đích: Xoá vĩnh viễn từng mục + dọn sạch thùng rác Local/Remote (S1 unify).
- Trách nhiệm: Phân tuyến (Route) → chọn nhánh Local/Remote bằng `match` + UNIVERSAL;
  xoá vĩnh viễn dùng chung [`DeleteScope`], dọn sạch dùng chung [`EmptyDirs`].
- Tương tác: Tầng `api::trash` bọc mỏng qua `core::task::blocking`. Không đụng IPC/frontend.
  Xoá từng mục remote cần cờ `--<backend>-trashed-only`; dọn sạch qua `rclone cleanup`
  khi backend có tính năng `CleanUp`.
*/

pub use super::trash_list::Route;
pub use crate::actions::types::{DeleteScope, EmptyDirs};
use super::trash_list::{list_local_inner, remote_type, trash_dir, trashed_only_flag};
use crate::core::rclone_caller;
use serde_json::Value;

/// Xoá vĩnh viễn một mục local khỏi thùng rác (bỏ cả nội dung và metadata).
fn delete_local_inner(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Thiếu định danh mục cần xoá.".to_string());
    }
    // Chặn path traversal: `id` phải là một tên đơn, không chứa phân cách.
    if id.contains('/') || id.contains('\\') || id == "." || id == ".." {
        return Err(format!("Định danh không hợp lệ: '{}'", id));
    }

    let dir = trash_dir()?;
    let target = dir.join("files").join(id);
    let info = dir.join("info").join(format!("{}.trashinfo", id));

    if !target.exists() && !info.exists() {
        return Err(format!("Không tìm thấy '{}' trong thùng rác.", id));
    }

    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("Lỗi xoá thư mục: {}", e))?;
    } else if target.exists() {
        std::fs::remove_file(&target).map_err(|e| format!("Lỗi xoá tệp: {}", e))?;
    }

    // Metadata mồ côi sẽ làm `list()` bỏ qua mục đó, nhưng vẫn nên dọn sạch.
    let _ = std::fs::remove_file(&info);
    Ok(())
}

/// Xoá vĩnh viễn một mục remote đang ở trong thùng rác.
///
/// Cần cờ `--<backend>-trashed-only` để rclone nhắm vào bản trong thùng rác chứ
/// không phải file cùng tên đang ở ngoài — thiếu cờ này sẽ xoá nhầm file đang dùng.
fn delete_remote_inner(remote: &str, path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần xoá.".to_string());
    }

    let backend = remote_type(remote)?;
    let flag = trashed_only_flag(&backend).ok_or_else(|| {
        format!(
            "rclone không hỗ trợ xoá từng mục trong thùng rác cho loại '{}'. Hãy dùng 'Dọn sạch thùng rác'.",
            backend
        )
    })?;

    let target = format!("{}:{}", remote, path);
    let is_dir = crate::actions::types::is_dir(&target).unwrap_or(false);
    let cmd = if is_dir { "purge" } else { "deletefile" };

    let output = rclone_caller::run_cmd(&[cmd, &target, flag])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Xoá vĩnh viễn '{}' thất bại: {}", path, output.status)
        } else {
            err
        });
    }
    Ok(())
}

/// Xoá vĩnh viễn một mục local. `scope` phải là [`DeleteScope::NoTrash`]
/// (mục đã nằm trong thùng rác nên `Trash` không còn ý nghĩa).
pub fn delete_local(id: &str, scope: DeleteScope) -> Result<(), String> {
    match scope {
        // UNIVERSAL: mục đã ở trong thùng rác — chuyển vào trash lần nữa là vô nghĩa.
        DeleteScope::Trash => Err("Mục đã nằm trong thùng rác, chỉ xoá vĩnh viễn được.".to_string()),
        // UNIVERSAL: NoTrash — xoá cả nội dung `Trash/files` lẫn metadata `.trashinfo`.
        DeleteScope::NoTrash => match Route::Local {
            // UNIVERSAL: Local xoá file/thư mục thẳng qua syscall.
            Route::Local => delete_local_inner(id),
            // UNIVERSAL: nhánh Remote không xảy ra ở hàm local — giữ để `match` đủ đầy.
            Route::Remote => Err("Tuyến Remote phải dùng `delete_remote`.".to_string()),
        },
    }
}

/// Xoá vĩnh viễn một mục remote. `scope` phải là [`DeleteScope::NoTrash`].
pub fn delete_remote(remote: &str, path: &str, scope: DeleteScope) -> Result<(), String> {
    match scope {
        // UNIVERSAL: mục đã ở trong thùng rác — chuyển vào trash lần nữa là vô nghĩa.
        DeleteScope::Trash => Err("Mục đã nằm trong thùng rác, chỉ xoá vĩnh viễn được.".to_string()),
        // UNIVERSAL: NoTrash — `purge` cho thư mục, `deletefile` cho file, kèm cờ trashed-only.
        DeleteScope::NoTrash => match Route::classify(remote) {
            // UNIVERSAL: classifier đã loại `"Local"` ở tầng api nên nhánh này là lỗi lập trình.
            Route::Local => Err("Tuyến Local phải dùng `delete_local`.".to_string()),
            // UNIVERSAL: remote nhắm đúng bản trong thùng rác nhờ cờ `--<backend>-trashed-only`.
            Route::Remote => delete_remote_inner(remote, path),
        },
    }
}

/// Xoá vĩnh viễn toàn bộ mục local trong thùng rác.
fn empty_local_inner() -> Result<(), String> {
    let items = list_local_inner()?;
    let mut errors = Vec::new();
    for item in &items {
        if let Err(e) = delete_local_inner(&item.id) {
            errors.push(format!("{}: {}", item.name, e));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Không xoá được {} mục:\n{}", errors.len(), errors.join("\n")))
    }
}

/// Dọn sạch toàn bộ thùng rác remote bằng `rclone cleanup`.
/// Kiểm tra trước tính năng `CleanUp` để báo lỗi rõ ràng thay vì thất bại mơ hồ.
fn empty_remote_inner(remote: &str) -> Result<(), String> {
    let target = format!("{}:", remote);

    if let Ok(output) = rclone_caller::run_cmd(&["backend", "features", &target]) {
        if output.status.success() {
            if let Ok(v) = serde_json::from_slice::<Value>(&output.stdout) {
                let can_cleanup = v
                    .get("Features")
                    .and_then(|f| f.get("CleanUp"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(true);
                if !can_cleanup {
                    return Err(format!(
                        "Remote '{}' không hỗ trợ dọn sạch thùng rác (CleanUp).",
                        remote
                    ));
                }
            }
        }
    }

    let output = rclone_caller::run_cmd(&["cleanup", &target])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Dọn sạch thùng rác '{}' thất bại: {}", remote, output.status)
        } else {
            err
        });
    }
    Ok(())
}

/// Dọn sạch thùng rác local. `cleanup` đã xoá toàn bộ nên mọi [`EmptyDirs`]
/// đều không cần dọn rỗng bổ sung (tương tự nhánh `purge` của delete).
pub fn empty_local(empty: EmptyDirs) -> Result<(), String> {
    match Route::Local {
        // UNIVERSAL: nhánh Remote không xảy ra ở hàm local — giữ để `match` đủ đầy.
        Route::Remote => Err("Tuyến Remote phải dùng `empty_remote`.".to_string()),
        // UNIVERSAL: Local dọn sạch bằng cách xoá từng mục (files + info).
        Route::Local => {
            empty_local_inner()?;
            match empty {
                // UNIVERSAL: Keep — giữ hành vi cũ, cleanup đã sạch nên không dọn thêm.
                EmptyDirs::Keep => Ok(()),
                // UNIVERSAL: đã xoá cả thư mục lẫn file nên skip `rmdir`.
                EmptyDirs::OnlyHere => Ok(()),
                // UNIVERSAL: đã xoá cả thư mục lẫn file nên skip `rmdirs`.
                EmptyDirs::Recursive => Ok(()),
            }
        }
    }
}

/// Dọn sạch thùng rác remote. `cleanup` đã xoá toàn bộ nên mọi [`EmptyDirs`]
/// đều không cần dọn rỗng bổ sung.
pub fn empty_remote(remote: &str, empty: EmptyDirs) -> Result<(), String> {
    match Route::classify(remote) {
        // UNIVERSAL: classifier đã loại `"Local"` ở tầng api nên nhánh này là lỗi lập trình.
        Route::Local => Err("Tuyến Local phải dùng `empty_local`.".to_string()),
        // UNIVERSAL: remote dọn sạch qua `rclone cleanup` (cần backend có `CleanUp`).
        Route::Remote => {
            empty_remote_inner(remote)?;
            match empty {
                // UNIVERSAL: Keep — giữ hành vi cũ, cleanup đã sạch nên không dọn thêm.
                EmptyDirs::Keep => Ok(()),
                // UNIVERSAL: cleanup đã sạch nên skip `rmdir`.
                EmptyDirs::OnlyHere => Ok(()),
                // UNIVERSAL: cleanup đã sạch nên skip `rmdirs`.
                EmptyDirs::Recursive => Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_local_rejects_bad_id_and_trash_scope() {
        assert!(delete_local("../../etc/passwd", DeleteScope::NoTrash).is_err());
        assert!(delete_local("sub/file", DeleteScope::NoTrash).is_err());
        assert!(delete_local("..", DeleteScope::NoTrash).is_err());
        assert!(delete_local("", DeleteScope::NoTrash).is_err());
        // Mục đã trong thùng rác thì scope Trash vô nghĩa.
        assert!(delete_local("some.txt", DeleteScope::Trash).is_err());
        assert!(delete_remote("GDrive", "a.txt", DeleteScope::Trash).is_err());
    }
}
