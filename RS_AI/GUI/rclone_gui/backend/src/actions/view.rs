/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả view-only (`view_download`) + `fs_get_thumbnail` thành plan thuần.
- Trách nhiệm: parse → build_target → chọn `copyto`/decode ảnh; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `core::path::cut_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone_caller::build_target;
use crate::core::task::blocking;
use crate::core::path::cut_remote_path;
use serde::Serialize;
use std::process::Command;

/// Đặc tả thuần cho `thumbnail`: đường dẫn local thực + nhóm định dạng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailPlan {
    pub local_path: String,
    pub group: ThumbnailGroup,
}

/// Đặc tả thuần cho `view` (view-only): đường dẫn mở đọc + lệnh tải về temp nếu cần.
/// - Không write-back, không gắn watcher; file temp dọn khi thoát app.
/// - Mở ngoài ở chế độ đọc (read-only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewPlan {
    pub temp_path: String,
    pub rclone_args: Vec<String>,
}

/// Nhóm định dạng thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailGroup {
    Video,
    Pdf,
    Image,
}

/// Dựng plan `thumbnail`: chỉ hỗ trợ đường dẫn local thực; remote → lỗi.
pub fn plan_thumbnail(path: &str) -> Result<ThumbnailPlan, String> {
    let (remote, real) = cut_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let local_path = match kind {
        // UNIVERSAL: thumbnail đọc file trực tiếp nên chỉ chạy trên ổ Local.
        RemoteKind::Local => real,
        // UNIVERSAL: remote phải tải về trước, plan này chưa hỗ trợ.
        RemoteKind::Remote => return Err("Thumbnail chỉ hỗ trợ tệp trên ổ Local.".to_string()),
    };
    if local_path.is_empty() {
        return Err("Thiếu đường dẫn tệp cần tạo thumbnail.".to_string());
    }
    let ext = std::path::Path::new(&local_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let group = match ext.as_str() {
        // UNIVERSAL: video qua `ffmpegthumbnailer`, pdf qua `pdftoppm`.
        "mp4" | "mkv" | "avi" | "mov" | "webm" => ThumbnailGroup::Video,
        "pdf" => ThumbnailGroup::Pdf,
        // UNIVERSAL: còn lại thử decode ảnh trực tiếp (`image` crate).
        _ => ThumbnailGroup::Image,
    };
    Ok(ThumbnailPlan { local_path, group })
}

/// Dựng plan `view`: Local trả đường dẫn gốc (không tải); remote tải `copyto` về temp.
pub fn plan_view_download(src: &str) -> Result<ViewPlan, String> {
    let (remote, real) = cut_remote_path(src);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    match kind {
        // UNIVERSAL: Local mở trực tiếp đường dẫn gốc ở chế độ đọc, không tải, không watcher.
        RemoteKind::Local => {
            if real.is_empty() {
                return Err("Thiếu đường dẫn tệp cần xem.".to_string());
            }
            Ok(ViewPlan {
                temp_path: real,
                rclone_args: Vec::new(),
            })
        }
        // UNIVERSAL: remote tải `copyto` src -> temp_dir/basename; view-only nên không write-back.
        RemoteKind::Remote => {
            let target = build_target(&remote, &real);
            let name = std::path::Path::new(&real)
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Thiếu tên tệp cần xem.".to_string())?;
            let temp_path = std::env::temp_dir()
                .join(name)
                .to_string_lossy()
                .to_string();
            Ok(ViewPlan {
                temp_path: temp_path.clone(),
                rclone_args: vec![
                    "copyto".to_string(),
                    target,
                    temp_path,
                ],
            })
        }
    }
}

/// UNIVERSAL: mở file = xem (S2 tách vai, dời từ `core/sys.rs`, hành vi giữ nguyên).
/// Khai báo cấu trúc DesktopApp để trả về cho giao diện khi chọn "Open With"
#[derive(Serialize)]
pub struct DesktopApp {
    // Tên hiển thị của ứng dụng
    pub name: String,
    // Lệnh thực thi của ứng dụng
    pub exec: String,
    // Đường dẫn hoặc tên icon của ứng dụng
    pub icon: String,
}

/// UNIVERSAL: mở file Local bằng app hệ điều hành; exec trực tiếp (không qua
/// `sh -c`) nên tên file chứa ký tự đặc biệt không chèn thêm lệnh được.
pub async fn sys_open_with(path: String, exec_cmd: Option<String>, app: Option<String>) -> Result<(), String> {
    blocking(move || {
        // Frontend gửi đường dẫn dạng "Remote::/path"; chỉ ổ Local mở được bằng app OS.
        let (remote, real_path) = cut_remote_path(&path);
        if remote != "Local" {
            return Err(format!(
                "Không thể mở trực tiếp file trên remote '{}'. Hãy copy về Local trước.",
                remote
            ));
        }
        let path = real_path;

        // Ưu tiên exec_cmd, ngược lại xdg-open mặc định trên Linux.
        let cmd = exec_cmd.or(app).unwrap_or_else(|| "xdg-open".to_string());

        // Lệnh .desktop chứa placeholder (%f, %U...) và tham số sẵn có.
        let mut parts = shell_split(&cmd);
        if parts.is_empty() {
            return Err("Lệnh mở file rỗng.".to_string());
        }

        let program = parts.remove(0);
        let mut args: Vec<String> = Vec::new();
        let mut path_injected = false;

        for part in parts {
            match part.as_str() {
                // Placeholder theo Desktop Entry Spec: thay bằng đường dẫn file.
                "%f" | "%F" | "%u" | "%U" => {
                    args.push(path.clone());
                    path_injected = true;
                }
                // Placeholder không dùng tới (icon, tên app...) thì bỏ qua.
                p if p.len() == 2 && p.starts_with('%') => {}
                other => args.push(other.to_string()),
            }
        }

        if !path_injected {
            args.push(path);
        }

        Command::new(&program)
            .args(&args)
            .spawn()
            .map_err(|e| format!("Lỗi khi chạy '{}': {}", program, e))?;

        Ok(())
    })
    .await
}

/// UNIVERSAL: tách chuỗi lệnh theo cú pháp shell tối giản (nháy đơn/kép, `\`).
fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' && !in_single {
            escaped = true;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        } else if c.is_whitespace() && !in_single && !in_double {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// UNIVERSAL: quét Desktop Entry thật (chuẩn FreeDesktop.org) thay vì dữ liệu giả.
pub async fn sys_list_apps() -> Result<Vec<DesktopApp>, String> {
    blocking(|| Ok(crate::logic::desktop_apps::list())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_remote_rejected() {
        assert!(plan_thumbnail("GDrive::/a.png").is_err());
    }

    #[test]
    fn view_local_passthrough_remote_copyto_temp() {
        let local = plan_view_download("Local::/tmp/a.txt").expect("plan view local");
        assert_eq!(local.temp_path, "/tmp/a.txt");
        assert!(local.rclone_args.is_empty());
        let remote = plan_view_download("GDrive::/docs/a.txt").expect("plan view remote");
        assert_eq!(
            remote.temp_path,
            std::env::temp_dir()
                .join("a.txt")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            remote.rclone_args,
            vec!["copyto", "GDrive:/docs/a.txt", remote.temp_path.as_str()]
        );
    }

    #[test]
    fn shell_split_handles_desktop_exec() {
        assert_eq!(shell_split("xdg-open"), vec!["xdg-open"]);
        assert_eq!(shell_split("code --wait %f"), vec!["code", "--wait", "%f"]);
        assert_eq!(shell_split("\"/opt/My App/run\" -a"), vec!["/opt/My App/run", "-a"]);
    }
}
