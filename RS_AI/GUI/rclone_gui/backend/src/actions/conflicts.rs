/*
[INTEGRITY NOTES]
- Mục đích: Tiền kiểm tra xung đột va chạm đường dẫn (PreJobConflicts: Name/Path Collision)
  trước khi tạo Job (Transfer/Rename).
- Trách nhiệm: Quét `lsjson` đích cấp 1 + `lsjson -R` đệ quy khi cả 2 là thư mục;
  kiểm tra va chạm tồn tại tệp đích cho Rename.
- Tương tác: Gọi `core::{path, rclone_caller}`, `logic::fastlane`,
  `actions::types::is_dir`. `api::files_view::fs_check_conflicts` là command
  mỏng gọi sang. Không đụng frontend.
*/
// UNIVERSAL: KHÔNG gộp vào `logic::queue::manifest` dù quét giống nhau —
// manifest `filter_map` nuốt mục thiếu tên IM LẶNG, còn đây phải warn từng mục
// lạ theo luật default-debug (thấy ở `child_conflicts`). Gộp là mất chẩn đoán.

use crate::core::path::cut_remote_path;
use serde::{Deserialize, Serialize};

/// Một xung đột src/dest (DTO gốc ở thợ `PreJobConflicts`).
/// `src_is_dir`/`dest_is_dir` cho biết ca xung đột (file/file = ghi đè,
/// lệch loại = chép sẽ hỏng tùy backend) — dir/dir không có mục riêng.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConflictInfo {
    pub relative_path: String,
    pub src_full_path: String,
    pub dest_full_path: String,
    pub src_is_dir: bool,
    pub dest_is_dir: bool,
}

/// Alias ngữ nghĩa mới cho hệ thống Job:
pub type PreJobConflictInfo = ConflictInfo;

/// UNIVERSAL: map tên cấp 1 → IsDir từ `lsjson` (thuần, dễ test).
pub fn existing_map(items: Vec<serde_json::Value>) -> std::collections::HashMap<String, bool> {
    items
        .into_iter()
        .filter_map(|item| {
            let name = item.get("Name").and_then(|v| v.as_str()).map(|s| s.to_string())?;
            let is_dir = item.get("IsDir").and_then(|v| v.as_bool()).unwrap_or(false);
            Some((name, is_dir))
        })
        .collect()
}

/// UNIVERSAL: tên file cuối từ `src_real_path` (sau `/` cuối, thuần).
pub fn base_name_of(src_real_path: &str) -> &str {
    match src_real_path.rfind('/') {
        Some(idx) => &src_real_path[idx + 1..],
        None => src_real_path,
    }
}

/// UNIVERSAL: nối `dest_real + base` (giữ `/`, thuần).
pub fn join_dest_item(dest_real: &str, base_name: &str) -> String {
    match dest_real {
        // UNIVERSAL: gốc đích rỗng/`/` → chỉ tên base.
        "" | "/" => base_name.to_string(),
        // UNIVERSAL: đích đã có `/` cuối → nối trực tiếp.
        d if d.ends_with('/') => format!("{}{}", d, base_name),
        // UNIVERSAL: còn lại chèn `/` phân cách.
        d => format!("{}/{}", d, base_name),
    }
}

/// UNIVERSAL: 1 xung đột trực tiếp (file/file, file/dir, dir/file — thuần).
/// `src_is_dir`/`dest_is_dir` ghi rõ ca nào (UI/backend-aware phân biệt
/// "sẽ hỏng" với "sẽ ghi đè"); dir/dir không qua đây (merge, chỉ báo file con).
pub fn direct_conflict(
    base_name: &str,
    src_target: &str,
    dest_item_target: &str,
    src_is_dir: bool,
    dest_is_dir: bool,
) -> ConflictInfo {
    ConflictInfo {
        relative_path: base_name.to_string(),
        src_full_path: src_target.to_string(),
        dest_full_path: dest_item_target.to_string(),
        src_is_dir,
        dest_is_dir,
    }
}

/// UNIVERSAL: xung đột file con (giao `Path` src ∩ dest, thuần).
pub fn child_conflicts(
    base_name: &str,
    src_target: &str,
    dest_item_target: &str,
    s_items: Vec<serde_json::Value>,
    d_names: &std::collections::HashSet<String>,
) -> Vec<ConflictInfo> {
    let mut out = Vec::new();
    for s_item in s_items {
        // UNIVERSAL: thiếu `Path` (JSON rclone lạ) → warn rồi bỏ qua như cũ.
        let s_path = match s_item.get("Path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                crate::core::debug::warn(None, "conflicts/child_conflicts", "mục lsjson thiếu Path, bỏ qua");
                continue;
            }
        };
        // UNIVERSAL: chỉ giao nhau mới là xung đột.
        if !d_names.contains(s_path) {
            continue;
        }
        out.push(ConflictInfo {
            relative_path: format!("{}/{}", base_name, s_path),
            src_full_path: format!("{}/{}", src_target, s_path),
            dest_full_path: format!("{}/{}", dest_item_target, s_path),
            // UNIVERSAL: hai scan đệ quy đều `--files-only` nên mục giao nhau
            // luôn là file/file — không cần tra loại, đóng dấu thẳng.
            src_is_dir: false,
            dest_is_dir: false,
        });
    }
    out
}

/// UNIVERSAL: tên `Path` đệ quy từ `lsjson -R` (thuần, nuốt mục thiếu `Path`).
pub fn dest_names(d_items: Vec<serde_json::Value>) -> std::collections::HashSet<String> {
    d_items
        .into_iter()
        .filter_map(|i| i.get("Path").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}
/// Tên hàm: check_conflicts
/// Mô tả: Quét va chạm tên và đường dẫn (Name/Path Collision) giữa nguồn và đích trước khi tạo Job Transfer.
/// Trả về mảng các xung đột đường dẫn tệp/thư mục.
pub async fn check_conflicts(
    srcs: Vec<String>,
    dest_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    crate::logic::fastlane::fastlane(move || {
        // UNIVERSAL: đệ quy + fast-list khi bật cờ engine; tắt thì giữ args cũ.
        let fast_list = crate::settings::engine::load_engine_flags()
            .map(|f| f.switches.fast_list)
            .unwrap_or(false);
        let mut conflicts = Vec::new();

        let (dest_remote, dest_real) = cut_remote_path(&dest_path);
        let dest_target = crate::core::rclone_caller::build_target(&dest_remote, &dest_real);

        // Lấy danh sách các file/thư mục hiện có ở cấp 1 của thư mục đích
        let output = crate::core::rclone_caller::run_cmd(&["lsjson", &dest_target])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            if err_msg.contains("directory not found") || err_msg.contains("failed to read directory") {
                return Ok(conflicts);
            }
            return Err(format!("Lỗi kiểm tra trùng lặp: {}", err_msg));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        // UNIVERSAL: JSON rclone hỏng là bất thường (không phải vỏ rỗng) → warn
        // rồi rớt về rỗng để UI không treo (giữ hành vi cũ, thêm dấu vết).
        let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) else {
            crate::core::debug::warn(None, "conflicts/check_conflicts", "lsjson đích JSON hỏng, rớt về rỗng");
            return Ok(conflicts);
        };
        let existing_items = existing_map(items);

        for src_item in srcs {
            let (src_remote, src_real_path) = cut_remote_path(&src_item);
            let base_name = base_name_of(&src_real_path);
            let src_target = crate::core::rclone_caller::build_target(&src_remote, &src_real_path);
            // UNIVERSAL: đường dẫn đích tuyệt đối cho mục này.
            let dest_item_real = join_dest_item(&dest_real, base_name);
            let dest_item_target = crate::core::rclone_caller::build_target(&dest_remote, &dest_item_real);
            // UNIVERSAL: không trùng tên cấp 1 -> không xung đột, qua mục sau.
            let Some(&is_dest_dir) = existing_items.get(base_name) else {
                continue;
            };
            let src_is_dir = crate::actions::types::is_dir(&src_target).unwrap_or(false);
            match (src_is_dir, is_dest_dir) {
                // UNIVERSAL: cả 2 là thư mục -> quét đệ quy file con.
                (true, true) => {
                    // UNIVERSAL: bật fast-list thì quét đệ quy nhanh hơn.
                    let src_args: Vec<&str> = if fast_list {
                        vec!["lsjson", "-R", "--files-only", "--fast-list", &src_target]
                    } else {
                        vec!["lsjson", "-R", "--files-only", &src_target]
                    };
                    let dest_args: Vec<&str> = if fast_list {
                        vec!["lsjson", "-R", "--files-only", "--fast-list", &dest_item_target]
                    } else {
                        vec!["lsjson", "-R", "--files-only", &dest_item_target]
                    };
                    let src_files_out = crate::core::rclone_caller::run_cmd(&src_args);
                    let dest_files_out = crate::core::rclone_caller::run_cmd(&dest_args);
                    // UNIVERSAL: spawn hỏng giữa chừng là bất thường → warn rồi bỏ
                    // mục này (giữ hành vi cũ là qua mục sau, thêm dấu vết).
                    let (Ok(s_out), Ok(d_out)) = (src_files_out, dest_files_out) else {
                        crate::core::debug::warn(None, "conflicts/check_conflicts", format!("spawn lsjson lỗi cho '{base_name}', bỏ qua"));
                        continue;
                    };
                    // UNIVERSAL: rclone báo lỗi giữa chừng → warn rồi bỏ mục này.
                    if !(s_out.status.success() && d_out.status.success()) {
                        crate::core::debug::warn(None, "conflicts/check_conflicts", format!("lsjson lỗi cho '{base_name}', bỏ qua"));
                        continue;
                    }
                    let s_json = String::from_utf8_lossy(&s_out.stdout);
                    let d_json = String::from_utf8_lossy(&d_out.stdout);
                    // UNIVERSAL: JSON hỏng giữa chừng → warn rồi bỏ mục này.
                    let (Ok(s_items), Ok(d_items)) = (
                        serde_json::from_str::<Vec<serde_json::Value>>(&s_json),
                        serde_json::from_str::<Vec<serde_json::Value>>(&d_json),
                    ) else {
                        crate::core::debug::warn(None, "conflicts/check_conflicts", format!("JSON hỏng cho '{base_name}', bỏ qua"));
                        continue;
                    };
                    let d_names = dest_names(d_items);
                    conflicts.extend(child_conflicts(base_name, &src_target, &dest_item_target, s_items, &d_names));
                }
                // UNIVERSAL: trực tiếp (file/file, file/dir, dir/file) — ghi rõ
                // ca nào để UI phân biệt "sẽ hỏng" với "sẽ ghi đè".
                _ => conflicts.push(direct_conflict(
                    base_name,
                    &src_target,
                    &dest_item_target,
                    src_is_dir,
                    is_dest_dir,
                )),
            }
        }

        Ok(conflicts)
    })
    .await
}

/// Alias ngữ nghĩa cho tiền kiểm tra chuyển file (PreJob Transfer Conflicts):
pub use check_conflicts as check_transfer_conflicts;

/// Tiền kiểm tra xung đột cho thao tác Đổi tên (PreJob Rename Conflict Check).
/// Trả về Some(ConflictInfo) nếu đích đã tồn tại (xung đột), hoặc None nếu đích an toàn.
pub fn check_rename_conflict(old_path: &str, new_path: &str) -> Result<Option<ConflictInfo>, String> {
    if old_path == new_path {
        return Ok(None);
    }
    let (src_remote, src_real) = cut_remote_path(old_path);
    let (dst_remote, dst_real) = cut_remote_path(new_path);
    if src_real.is_empty() || dst_real.is_empty() {
        return Err("Thiếu đường dẫn nguồn hoặc đích khi kiểm tra đổi tên.".to_string());
    }
    let src_target = crate::core::rclone_caller::build_target(&src_remote, &src_real);
    let dst_target = crate::core::rclone_caller::build_target(&dst_remote, &dst_real);

    let (dst_exists, dst_is_dir) = if dst_remote == "Local" {
        let p = std::path::Path::new(&dst_real);
        (p.exists(), p.is_dir())
    } else {
        match crate::actions::types::is_dir(&dst_target) {
            Some(is_dir) => (true, is_dir),
            None => (false, false),
        }
    };

    if !dst_exists {
        return Ok(None);
    }

    let src_is_dir = if src_remote == "Local" {
        std::path::Path::new(&src_real).is_dir()
    } else {
        crate::actions::types::is_dir(&src_target).unwrap_or(false)
    };

    let base_name = base_name_of(&dst_real);
    Ok(Some(direct_conflict(base_name, &src_target, &dst_target, src_is_dir, dst_is_dir)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_pure_shape() {
        // UNIVERSAL: map + join + direct giữ nguyên hình JSON/hành vi.
        let items = vec![
            serde_json::json!({"Name": "a.txt", "IsDir": false}),
            serde_json::json!({"Name": "d", "IsDir": true}),
            serde_json::json!({"IsDir": false}),
        ];
        let m = existing_map(items);
        assert_eq!(m.get("a.txt"), Some(&false));
        assert_eq!(m.get("d"), Some(&true));
        assert_eq!(base_name_of("/x/y.txt"), "y.txt");
        assert_eq!(base_name_of("y.txt"), "y.txt");
        assert_eq!(join_dest_item("", "a"), "a");
        assert_eq!(join_dest_item("/", "a"), "a");
        assert_eq!(join_dest_item("/d/", "a"), "/d/a");
        assert_eq!(join_dest_item("/d", "a"), "/d/a");
        let c = direct_conflict("a", "S:/a", "D:/a", false, false);
        assert_eq!(c.relative_path, "a");
        assert!(!c.src_is_dir && !c.dest_is_dir);
        // UNIVERSAL: ca lệch loại ghi đúng cờ (UI/backend-aware phân biệt).
        let df = direct_conflict("d", "S:/d", "D:/d", false, true);
        assert!(!df.src_is_dir && df.dest_is_dir);
        let dset: std::collections::HashSet<String> = ["x".to_string()].into_iter().collect();
        let kids = child_conflicts(
            "d",
            "S:/d",
            "D:/d",
            vec![serde_json::json!({"Path": "x"}), serde_json::json!({"Path": "y"}), serde_json::json!({})],
            &dset,
        );
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].relative_path, "d/x");
        // UNIVERSAL: dest_names bóc đúng tập Path, bỏ mục thiếu tên.
        let names = dest_names(vec![
            serde_json::json!({"Path": "a/b.txt"}),
            serde_json::json!({"Name": "lonely"}),
            serde_json::json!({}),
        ]);
        assert!(names.contains("a/b.txt"));
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn test_check_rename_conflict() {
        let temp_dir = std::env::temp_dir();
        let src = temp_dir.join("test_conflict_src.txt");
        let dst = temp_dir.join("test_conflict_dst.txt");
        let non_exist = temp_dir.join("test_conflict_non_exist.txt");
        let _ = std::fs::write(&src, b"source");
        let _ = std::fs::write(&dst, b"dest");

        let src_str = format!("Local::{}", src.to_string_lossy());
        let dst_str = format!("Local::{}", dst.to_string_lossy());
        let non_exist_str = format!("Local::{}", non_exist.to_string_lossy());

        // Same path -> None
        let same = check_rename_conflict(&src_str, &src_str).expect("check");
        assert!(same.is_none());

        // Target does not exist -> None
        let safe = check_rename_conflict(&src_str, &non_exist_str).expect("check");
        assert!(safe.is_none());

        // Target exists -> Some(conflict)
        let conflict = check_rename_conflict(&src_str, &dst_str).expect("check");
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.relative_path, "test_conflict_dst.txt");
        assert!(!c.src_is_dir);
        assert!(!c.dest_is_dir);

        // Dọn dẹp
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dst);
    }
}

