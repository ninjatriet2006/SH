/* [INTEGRITY NOTES]
 * UNIVERSAL: tách từ `core/sys.rs` (S2 tách vai) — custom action = logic nghiệp vụ,
 * không phải thao tác OS thô. Stub rỗng, chưa đọc config (giữ nguyên hành vi).
 */

use crate::core::task::blocking;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// UNIVERSAL: item file tối giản do frontend gửi lên để lọc action hợp lệ.
#[derive(Deserialize)]
pub struct SimpleFileItem {
    pub name: String,
    pub is_dir: bool,
}

/// UNIVERSAL: lệnh tự tạo trên menu chuột phải (stub, chưa đọc config).
#[derive(Serialize)]
pub struct CustomAction {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub selection: String,
    pub extensions: Vec<String>,
}

/// UNIVERSAL: stub — trả danh sách rỗng (đọc config JSON làm ở đợt sau).
pub async fn sys_get_custom_actions() -> Result<Vec<CustomAction>, String> {
    Ok(vec![])
}

/// UNIVERSAL: lọc action hợp lệ theo số lượng file chọn + phần mở rộng.
pub async fn sys_get_valid_actions(files: Vec<SimpleFileItem>) -> Result<Vec<CustomAction>, String> {
    let actions = sys_get_custom_actions().await?;
    let sel_count = files.len();

    if sel_count == 0 {
        return Ok(vec![]);
    }

    let valid: Vec<CustomAction> = actions
        .into_iter()
        .filter(|a| {
            if a.selection == "s" && sel_count != 1 {
                return false;
            }
            if a.selection == "m" && sel_count < 2 {
                return false;
            }

            if a.extensions.iter().any(|ext| ext == "any") {
                return true;
            }

            files.iter().all(|f| {
                if f.is_dir && a.extensions.iter().any(|ext| ext == "dir") {
                    return true;
                }
                let ext = f.name.split('.').next_back().unwrap_or("").to_lowercase();
                a.extensions.iter().any(|e| e.to_lowercase() == ext)
            })
        })
        .collect();

    Ok(valid)
}

/// UNIVERSAL: exec_template do người dùng định nghĩa nên chạy qua shell;
/// tên file bọc nháy đơn an toàn để chống chèn lệnh.
pub async fn sys_execute_custom_action(
    exec_template: String,
    base_path: String,
    file_names: Vec<String>,
) -> Result<(), String> {
    let paths_str = file_names
        .iter()
        .map(|name| {
            let p = if base_path.starts_with("trash://") || base_path == "/" {
                format!("{}/{}", base_path.trim_end_matches('/'), name)
            } else {
                format!("{}/{}", base_path, name)
            };
            shell_quote(&p)
        })
        .collect::<Vec<String>>()
        .join(" ");

    let cmd = exec_template.replace("%f", &paths_str);

    blocking(move || {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
}

/// UNIVERSAL: bọc chuỗi thành literal shell POSIX bằng nháy đơn.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_escapes_injection() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("/tmp/$(id)"), "'/tmp/$(id)'");
        assert_eq!(shell_quote("/tmp/it's"), r"'/tmp/it'\''s'");
    }
}
