/*
[INTEGRITY NOTES]
- Mục đích: Xoá vĩnh viễn từng mục + dọn sạch thùng rác Local/Remote (S1 unify).
- Trách nhiệm: Phân tuyến (Route) → chọn nhánh Local/Remote bằng `match` + UNIVERSAL;
  xoá vĩnh viễn dùng chung [`DeleteScope`], dọn sạch dùng chung [`EmptyDirs`].
- Tương tác: Tầng `api::trash_manager` bọc mỏng qua `logic::fastlane::fastlane`. Không đụng IPC/frontend.
  Xoá từng mục remote cần cờ `--<backend>-trashed-only`; dọn sạch qua `rclone cleanup`
  khi backend có tính năng `CleanUp`.
*/

use crate::actions::information::trash_list::{self, Route, remote_type, trash_dir};
pub use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::core::rclone_caller;

/// Xoá vĩnh viễn một mục local khỏi thùng rác (bỏ cả nội dung và metadata).
/// (đồng bộ; tầng api bọc `fastlane`).
pub fn delete_local(id: &str, scope: DeleteScope) -> Result<(), String> {
    match scope {
        // UNIVERSAL: mục đã ở trong thùng rác — chuyển vào trash lần nữa là vô nghĩa.
        DeleteScope::Trash => Err("Mục đã nằm trong thùng rác, chỉ xoá vĩnh viễn được.".to_string()),
        // UNIVERSAL: NoTrash — xoá cả nội dung `Trash/files` lẫn metadata `.trashinfo`.
        // UNIVERSAL: thụt lề phẳng theo nhánh (sửa từ bản lệch cũ).
        DeleteScope::NoTrash => {
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
    }
}

/// Xoá vĩnh viễn một mục remote đang ở trong thùng rác.
///
/// Cần cờ `--<backend>-trashed-only` để rclone nhắm vào bản trong thùng rác chứ
/// không phải file cùng tên đang ở ngoài — thiếu cờ này sẽ xoá nhầm file đang dùng.
/// (đồng bộ; tầng api bọc `fastlane`).
pub fn delete_remote(remote: &str, path: &str, scope: DeleteScope) -> Result<(), String> {
    match scope {
        // UNIVERSAL: mục đã ở trong thùng rác — chuyển vào trash lần nữa là vô nghĩa.
        DeleteScope::Trash => Err("Mục đã nằm trong thùng rác, chỉ xoá vĩnh viễn được.".to_string()),
        // UNIVERSAL: NoTrash — `purge` cho thư mục, `deletefile` cho file, kèm cờ trashed-only.
        DeleteScope::NoTrash => {
            if Route::classify(remote) == Route::Local {
                return Err("Tuyến Local phải dùng `delete_local`.".to_string());
            }
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần xoá.".to_string());
    }

    let backend = remote_type(remote)?;
    // UNIVERSAL không-bịa: cùng quy ước `--{type}-trashed-only` như list (xem
    // `trash_list::trashed_only_flag`) — rclone tự xác nhận lúc chạy.
    let flag = trash_list::trashed_only_flag(&backend)?;

    let target = format!("{}:{}", remote, path);
    let is_dir = crate::actions::types::is_dir(&target).unwrap_or(false);
    let cmd = if is_dir { "purge" } else { "deletefile" };

    let output = rclone_caller::run_cmd(&[cmd, &target, &flag])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // UNIVERSAL: flag lạ với backend này = không có khái niệm xóa-trash
        // (đã xác minh chữ `unknown flag` với Box thật) — mapping dùng chung.
        if let Some(clean) =
            trash_list::map_unknown_flag(&backend, &err, "xoá từng mục trong")
        {
            return Err(format!("{clean} Hãy dùng 'Dọn sạch thùng rác'."));
        }
        return Err(if err.is_empty() {
            format!("Xoá vĩnh viễn '{}' thất bại: {}", path, output.status)
        } else {
            err
        });
    }
    Ok(())
        }
    }
}

/// Xoá vĩnh viễn toàn bộ mục local trong thùng rác.
pub fn empty_local(empty: EmptyDirs) -> Result<(), String> {
    // UNIVERSAL: Local dọn sạch bằng cách xoá từng mục (files + info).
    let items = trash_list::list_local()?;
    let mut errors = Vec::new();
    for item in &items {
        if let Err(e) = delete_local(&item.id, DeleteScope::NoTrash) {
            errors.push(format!("{}: {}", item.name, e));
        }
    }
    if !errors.is_empty() {
        return Err(format!("Không xoá được {} mục:\n{}", errors.len(), errors.join("\n")));
    }
    match empty {
        // UNIVERSAL: Keep — giữ hành vi cũ, cleanup đã sạch nên không dọn thêm.
        EmptyDirs::Keep => Ok(()),
        // UNIVERSAL: đã xoá cả thư mục lẫn file nên skip `rmdir`.
        EmptyDirs::OnlyHere => Ok(()),
        // UNIVERSAL: đã xoá cả thư mục lẫn file nên skip `rmdirs`.
        EmptyDirs::Recursive => Ok(()),
    }
}

/// Dọn sạch thùng rác remote. `cleanup` đã xoá toàn bộ nên mọi [`EmptyDirs`]
/// đều không cần dọn rỗng bổ sung.
pub fn empty_remote(remote: &str, empty: EmptyDirs) -> Result<(), String> {
    if Route::classify(remote) == Route::Local {
        return Err("Tuyến Local phải dùng `empty_local`.".to_string());
    }
    let target = format!("{}:", remote);

    // UNIVERSAL không-bịa: gate bằng cờ `CleanUp` THẬT qua cache (không hỏi
    // được → false → từ chối như bảng cũ). Bỏ double-check đọc trực tiếp vì
    // cùng 1 cờ — trùng lặp; lệnh `cleanup` cuối cùng vẫn là trọng tài thật.
    let can_cleanup = crate::actions::feature::checkcap::backend_features_cached(remote)
        .map(|f| f.clean_up)
        .unwrap_or(false);
    if !can_cleanup {
        return Err(format!(
            "Remote '{}' không hỗ trợ dọn sạch thùng rác (CleanUp).",
            remote
        ));
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
    match empty {
        // UNIVERSAL: Keep — giữ hành vi cũ, cleanup đã sạch nên không dọn thêm.
        EmptyDirs::Keep => Ok(()),
        // UNIVERSAL: cleanup đã sạch nên skip `rmdir`.
        EmptyDirs::OnlyHere => Ok(()),
        // UNIVERSAL: cleanup đã sạch nên skip `rmdirs`.
        EmptyDirs::Recursive => Ok(()),
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
