/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả view-only (`view_download`) + `fs_get_thumbnail` thành plan thuần.
- Trách nhiệm: parse → build_target → chọn `copyto`/decode ảnh; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone_caller::build_target;
use crate::logic::file_ops::parse_remote_path;

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
    let (remote, real) = parse_remote_path(path);
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
    let (remote, real) = parse_remote_path(src);
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
}
