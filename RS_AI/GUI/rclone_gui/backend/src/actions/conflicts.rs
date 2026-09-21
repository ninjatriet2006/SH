/*
[INTEGRITY NOTES]
- Mục đích: Kiểm tra xung đột hash giữa source và dest trước copy/move.
- Trách nhiệm: `lsjson` đích cấp 1 + `lsjson -R` đệ quy khi cả 2 là thư mục.
- Tương tác: Gọi `core::{path, rclone_caller, task}`, `actions::types::is_dir`.
  `api::files_view::fs_check_conflicts` là command mỏng gọi sang. Không đụng frontend.
*/
// UNIVERSAL: bê nguyên văn logic `logic::file_ops::check_conflicts` (giữ hành vi).
// TODO(S2): dùng chung manifest/máy quét `logic::queue::manifest` khi rẻ.

use crate::core::path::cut_remote_path;
use serde::{Deserialize, Serialize};

/// Một xung đột src/dest (DTO gốc ở thợ `conflicts`).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct ConflictInfo {
    pub relative_path: String,
    pub src_full_path: String,
    pub dest_full_path: String,
}

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
pub fn direct_conflict(base_name: &str, src_target: &str, dest_item_target: &str) -> ConflictInfo {
    ConflictInfo {
        relative_path: base_name.to_string(),
        src_full_path: src_target.to_string(),
        dest_full_path: dest_item_target.to_string(),
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
/// Mô tả: Kích hoạt `rclone check` ngầm để đệ quy kiểm tra xung đột hash giữa source và dest.
/// Trả về mảng các đường dẫn file con bị khác hash.
pub async fn check_conflicts(
    _app_handle: tauri::AppHandle,
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
        // UNIVERSAL: JSON hỏng -> giữ hành vi cũ (nuốt lỗi, trả rỗng).
        let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) else {
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
                    // UNIVERSAL: lỗi spawn -> nuốt như cũ.
                    let (Ok(s_out), Ok(d_out)) = (src_files_out, dest_files_out) else {
                        continue;
                    };
                    // UNIVERSAL: lệnh lỗi -> nuốt như cũ.
                    if !(s_out.status.success() && d_out.status.success()) {
                        continue;
                    }
                    let s_json = String::from_utf8_lossy(&s_out.stdout);
                    let d_json = String::from_utf8_lossy(&d_out.stdout);
                    // UNIVERSAL: JSON hỏng -> nuốt như cũ.
                    let (Ok(s_items), Ok(d_items)) = (
                        serde_json::from_str::<Vec<serde_json::Value>>(&s_json),
                        serde_json::from_str::<Vec<serde_json::Value>>(&d_json),
                    ) else {
                        continue;
                    };
                    let d_names = dest_names(d_items);
                    conflicts.extend(child_conflicts(base_name, &src_target, &dest_item_target, s_items, &d_names));
                }
                // UNIVERSAL: trực tiếp (file/file, file/dir, dir/file).
                _ => conflicts.push(direct_conflict(base_name, &src_target, &dest_item_target)),
            }
        }

        Ok(conflicts)
    })
    .await
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
        let c = direct_conflict("a", "S:/a", "D:/a");
        assert_eq!(c.relative_path, "a");
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
    }
}
