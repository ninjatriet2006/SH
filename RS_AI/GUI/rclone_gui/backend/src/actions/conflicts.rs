/*
[INTEGRITY NOTES]
- Mục đích: Kiểm tra xung đột hash giữa source và dest trước copy/move.
- Trách nhiệm: `lsjson` đích cấp 1 + `lsjson -R` đệ quy khi cả 2 là thư mục.
- Tương tác: Gọi `core::{path, rclone_caller, task}`, `actions::types::is_dir`.
  `api::files::fs_check_conflicts` là wrapper mỏng. Không đụng IPC/frontend.
*/
// UNIVERSAL: bê nguyên văn logic `logic::file_ops::check_conflicts` (giữ hành vi).
// TODO(S2): dùng chung manifest/máy quét `logic::queue::manifest` khi rẻ.

use crate::api::files::ConflictInfo;
use crate::core::path::cut_remote_path;

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
            .map(|f| f.fast_list)
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
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
            let existing_items: std::collections::HashMap<String, bool> = items
                .into_iter()
                .filter_map(|item| {
                    let name = item.get("Name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let is_dir = item.get("IsDir").and_then(|v| v.as_bool()).unwrap_or(false);
                    name.map(|n| (n, is_dir))
                })
                .collect();

            for src_item in srcs {
                let (src_remote, src_real_path) = cut_remote_path(&src_item);
                let base_name = if let Some(idx) = src_real_path.rfind('/') {
                    &src_real_path[idx + 1..]
                } else {
                    src_real_path.as_str()
                };

                let src_target = crate::core::rclone_caller::build_target(&src_remote, &src_real_path);

                // Xây dựng đường dẫn đích tuyệt đối cho mục này
                let dest_item_real = if dest_real.is_empty() || dest_real == "/" {
                    base_name.to_string()
                } else {
                    if dest_real.ends_with('/') {
                        format!("{}{}", dest_real, base_name)
                    } else {
                        format!("{}/{}", dest_real, base_name)
                    }
                };
                let dest_item_target = crate::core::rclone_caller::build_target(&dest_remote, &dest_item_real);

                if let Some(&is_dest_dir) = existing_items.get(base_name) {
                    let src_is_dir = crate::actions::types::is_dir(&src_target).unwrap_or(false);

                    if src_is_dir && is_dest_dir {
                        // Cả 2 đều là thư mục -> Quét đệ quy các file con
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

                        if let (Ok(s_out), Ok(d_out)) = (src_files_out, dest_files_out) {
                            if s_out.status.success() && d_out.status.success() {
                                let s_json = String::from_utf8_lossy(&s_out.stdout);
                                let d_json = String::from_utf8_lossy(&d_out.stdout);

                                if let (Ok(s_items), Ok(d_items)) = (
                                    serde_json::from_str::<Vec<serde_json::Value>>(&s_json),
                                    serde_json::from_str::<Vec<serde_json::Value>>(&d_json),
                                ) {
                                    let d_names: std::collections::HashSet<String> = d_items
                                        .into_iter()
                                        .filter_map(|i| i.get("Path").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                        .collect();

                                    for s_item in s_items {
                                        if let Some(s_path) = s_item.get("Path").and_then(|v| v.as_str()) {
                                            if d_names.contains(s_path) {
                                                // Xung đột file con!
                                                conflicts.push(ConflictInfo {
                                                    relative_path: format!("{}/{}", base_name, s_path),
                                                    src_full_path: format!("{}/{}", src_target, s_path),
                                                    dest_full_path: format!("{}/{}", dest_item_target, s_path),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Xung đột trực tiếp (file vs file, file vs dir, hoặc dir vs file)
                        conflicts.push(ConflictInfo {
                            relative_path: base_name.to_string(),
                            src_full_path: src_target,
                            dest_full_path: dest_item_target,
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    })
    .await
}
