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
use crate::logic::fastlane::fastlane;
use crate::core::path::cut_remote_path;
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
    /// UNIVERSAL: đuôi ngoài whitelist — không phải lỗi, chỉ là không có gì để xem.
    /// Trả `Ok(None)` im lặng để UI hiện icon chung, khỏi báo lỗi suốt ngày.
    Unsupported,
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
        // UNIVERSAL: chỉ đuôi ảnh đã biết mới decode (`image` crate).
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "webp" => ThumbnailGroup::Image,
        // UNIVERSAL: đuôi ngoài whitelist (zip, txt...) — không phải lỗi nên
        // không warn (kẻo sổ đầy): đánh dấu Unsupported, execute trả Ok(None).
        // Muốn hỗ trợ đuôi mới thì thêm vào 2 nhánh trên (whitelist là danh sách).
        _ => ThumbnailGroup::Unsupported,
    };
    Ok(ThumbnailPlan { local_path, group })
}

/// UNIVERSAL: thực thi thumbnail từ `plan_thumbnail` (video→ffmpegthumbnailer,
/// pdf→pdftoppm, ảnh→image crate; trả data URI base64).
/// Đuôi ngoài whitelist → `Ok(None)` im lặng (UI hiện icon chung), KHÔNG phải lỗi.
pub async fn execute_thumbnail(path: String) -> Result<Option<String>, String> {
    fastlane(move || {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use std::io::Cursor;

        let plan = plan_thumbnail(&path)?;
        if plan.group == ThumbnailGroup::Unsupported {
            return Ok(None);
        }
        match plan.group {
            // UNIVERSAL: video qua `ffmpegthumbnailer`, hỏng thì rơi xuống decode ảnh.
            ThumbnailGroup::Video => {
                let output = Command::new("ffmpegthumbnailer")
                    .args(["-i", &plan.local_path, "-o", "-", "-s", "64", "-c", "jpeg", "-f"])
                    .output();
                // UNIVERSAL: match phẳng, hỏng → rơi xuống decode ảnh như cũ.
                match output {
                    Ok(out) if out.status.success() => {
                        let base64_str = STANDARD.encode(&out.stdout);
                        return Ok(Some(format!("data:image/jpeg;base64,{}", base64_str)));
                    }
                    // UNIVERSAL: hỏng ffmpegthumbnailer → warn rồi rơi xuống decode ảnh như cũ.
                    other => {
                        crate::core::debug::warn(None, "view/execute_thumbnail", format!("ffmpegthumbnailer hỏng: ok={}", other.as_ref().map(|o| o.status.success()).unwrap_or(false)));
                    }
                }
            }
            // UNIVERSAL: pdf qua `pdftoppm`, hỏng thì rơi xuống decode ảnh.
            ThumbnailGroup::Pdf => {
                let output = Command::new("pdftoppm")
                    .args([
                        "-jpeg", "-f", "1", "-l", "1", "-singlefile", "-scale-to", "64",
                        &plan.local_path,
                    ])
                    .output();
                // UNIVERSAL: match phẳng, hỏng → rơi xuống decode ảnh như cũ.
                match output {
                    Ok(out) if out.status.success() => {
                        let base64_str = STANDARD.encode(&out.stdout);
                        return Ok(Some(format!("data:image/jpeg;base64,{}", base64_str)));
                    }
                    // UNIVERSAL: hỏng pdftoppm → warn rồi rơi xuống decode ảnh như cũ.
                    other => {
                        crate::core::debug::warn(None, "view/execute_thumbnail", format!("pdftoppm hỏng: ok={}", other.as_ref().map(|o| o.status.success()).unwrap_or(false)));
                    }
                }
            }
            // UNIVERSAL: còn lại thử decode ảnh trực tiếp (`image` crate).
            ThumbnailGroup::Image => {}
            // UNIVERSAL: đã chặn Unsupported ở trên — nhánh này không tới được.
            ThumbnailGroup::Unsupported => return Ok(None),
        }

        let img = image::open(&plan.local_path).map_err(|e| format!("Lỗi mở ảnh: {}", e))?;
        let thumb = img.thumbnail(64, 64);
        let mut buffer = Cursor::new(Vec::new());
        thumb
            .write_to(&mut buffer, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Lỗi tạo thumb: {}", e))?;
        let base64_str = STANDARD.encode(buffer.get_ref());
        Ok(Some(format!("data:image/jpeg;base64,{}", base64_str)))
    })
    .await
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
/// UNIVERSAL: DTO "Open With" (`DesktopApp`) + `sys_list_apps` đã dời về 1 nơi
/// làm thật `actions::system` — xóa vỏ chuyển tay, `api::sys` gọi thẳng system.
pub async fn sys_open_with(path: String, exec_cmd: Option<String>, app: Option<String>) -> Result<(), String> {
    fastlane(move || {
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
        // UNIVERSAL: phẳng else-if ký tự → match trên (escaped,in_single,in_double,c).
        match (escaped, in_single, in_double, c) {
            // UNIVERSAL: ký tự sau `\` luôn literal.
            (true, _, _, ch) => {
                current.push(ch);
                escaped = false;
            }
            // UNIVERSAL: `\` ngoài nháy đơn thì escape ký tự sau.
            (false, false, _, '\\') => escaped = true,
            (false, true, _, '\\') => current.push('\\'),
            // UNIVERSAL: nháy đơn/kép toggle khi ngoài quote đối ứng.
            (false, _, false, '\'') => in_single = !in_single,
            (false, _, _, '\'') => current.push('\''),
            (false, false, _, '"') => in_double = !in_double,
            (false, _, _, '"') => current.push('"'),
            // UNIVERSAL: trắng ngoài quote → cắt token; trong quote là literal.
            (false, false, false, ch) if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            // UNIVERSAL: còn lại là ký tự thường/literal trong quote.
            (_, _, _, ch) => current.push(ch),
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_remote_rejected() {
        assert!(plan_thumbnail("GDrive::/a.png").is_err());
    }

    #[test]
    fn thumbnail_known_image_ok_and_unknown_ext_unsupported() {
        assert!(plan_thumbnail("Local::/tmp/a.png").is_ok());
        assert!(plan_thumbnail("Local::/tmp/a.JPG").is_ok());
        // UNIVERSAL: đuôi ngoài whitelist không phải lỗi — plan vẫn Ok nhóm Unsupported.
        let plan = plan_thumbnail("Local::/tmp/a.zip").expect("zip plan");
        assert_eq!(plan.group, ThumbnailGroup::Unsupported);
    }

    #[tokio::test]
    async fn thumbnail_unsupported_returns_none_silently() {
        // UNIVERSAL: không xem được ≠ lỗi — UI hiện icon chung, khỏi báo suốt ngày.
        let out = execute_thumbnail("Local::/tmp/a.zip".to_string()).await.expect("no error");
        assert!(out.is_none());
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
