/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints thao tác File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend (tham số đường dẫn gộp chung kiểu Remote::/Path), gọi tầng `logic`/`actions` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Copy/move/cancel chạy qua `logic::jobs`.
*/
// UNIVERSAL: fs_copy/fs_move/fs_cancel cũ đã gộp về jobs — xóa wrapper đường cũ.

use serde::{Deserialize, Serialize};

use crate::actions::perm::Policy;
use crate::logic::fastlane::fastlane;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct ConflictInfo {
    pub relative_path: String,
    pub src_full_path: String,
    pub dest_full_path: String,
}

pub async fn fs_check_conflicts(
    app_handle: tauri::AppHandle,
    srcs: Vec<String>,
    dest_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::conflicts::check_conflicts`.
    crate::actions::conflicts::check_conflicts(app_handle, srcs, dest_path).await
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileItem {
    pub uuid: String,
    pub name: String,
    pub size: i64,
    pub is_dir: bool,
    pub mod_time: String,
    pub file_type: Option<String>,
}

#[derive(serde::Serialize)]
pub struct StatInfo {
    pub(crate) size: u64,
    pub(crate) file_count: u64,
    pub(crate) dir_count: u64,
    pub(crate) permissions: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
}

#[derive(serde::Serialize)]
pub struct SearchResultItem {
    pub(crate) item: FileItem,
    pub(crate) path: String,
}

pub async fn list_files(
    app_handle: tauri::AppHandle,
    path: String,
    pane: Option<String>,
) -> Result<Vec<FileItem>, String> {
    crate::actions::list::execute_list(app_handle, path, pane).await
}

pub async fn fs_mkdir(path: String) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::create::execute_mkdir` (AllowSystem = hành vi cũ).
    crate::actions::create::execute_mkdir(path, Policy::AllowSystem).await
}

pub async fn fs_delete(path: String) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — xóa vĩnh viễn (`NoTrash`) như bản inline cũ.
    crate::actions::delete_op::execute_delete(path, crate::actions::delete_op::DeleteScope::NoTrash).await
}

pub async fn fs_touch(path: String) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::create::execute_touch` (AllowSystem = hành vi cũ).
    crate::actions::create::execute_touch(path, Policy::AllowSystem).await
}

pub async fn fs_rename(old_path: String, new_path: String) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::rename::execute_rename` (AllowSystem = hành vi cũ).
    crate::actions::rename::execute_rename(old_path, new_path, Policy::AllowSystem).await
}

pub async fn fs_stat_advanced(path: String) -> Result<StatInfo, String> {
    crate::actions::stat::execute_stat(path).await
}

pub async fn fs_search(path: String, query: String) -> Result<Vec<SearchResultItem>, String> {
    crate::actions::search::execute_search(path, query).await
}

pub async fn get_home_dir() -> Result<String, String> {
    // Trả về đúng $HOME. (Trước đây hàm này trả về ~/Desktop nhưng vẫn được gọi
    // dưới nhãn "Local", gây nhầm lẫn về vị trí thực tế đang mở.)
    Ok(std::env::var("HOME").unwrap_or_else(|_| "/".to_string()))
}

/// Một vị trí truy cập nhanh trong sidebar.
#[derive(serde::Serialize)]
pub struct UserPlace {
    /// Nhãn hiển thị (đã theo ngôn ngữ hệ thống nếu XDG cung cấp).
    pub name: String,
    /// Đường dẫn tuyệt đối trên ổ Local.
    pub path: String,
    /// Emoji gợi ý cho UI.
    pub icon: String,
    /// Khoá XDG (`HOME`, `DESKTOP`, ...) để Frontend nhận diện.
    pub kind: String,
}

/// Tra một thư mục XDG bằng `xdg-user-dir`. Trả `None` nếu không có hoặc trùng $HOME
/// (khi thư mục chưa được tạo, `xdg-user-dir` trả về chính $HOME).
fn xdg_user_dir(key: &str, home: &str) -> Option<String> {
    let out = Command::new("xdg-user-dir").arg(key).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() || path == home {
        return None;
    }
    Some(path)
}

/// Tên hàm: get_user_places
/// Mô tả: Danh sách thư mục người dùng chuẩn XDG để dựng mục "Truy cập nhanh".
/// Chỉ trả về thư mục thực sự tồn tại, nên không hiện mục dẫn tới đường dẫn rỗng.
pub async fn get_user_places() -> Result<Vec<UserPlace>, String> {
    fastlane(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

        let mut places = vec![UserPlace {
            name: "Home".to_string(),
            path: home.clone(),
            icon: "🏠".to_string(),
            kind: "HOME".to_string(),
        }];

        // Thứ tự giống trình quản lý tệp thông dụng (Nemo/Nautilus).
        let candidates = [
            ("DESKTOP", "🖥️"),
            ("DOWNLOAD", "⬇️"),
            ("DOCUMENTS", "📄"),
            ("PICTURES", "🖼️"),
            ("MUSIC", "🎵"),
            ("VIDEOS", "🎬"),
        ];

        for (key, icon) in candidates {
            if let Some(path) = xdg_user_dir(key, &home) {
                if std::path::Path::new(&path).is_dir() {
                    let name = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(key)
                        .to_string();
                    places.push(UserPlace {
                        name,
                        path,
                        icon: icon.to_string(),
                        kind: key.to_string(),
                    });
                }
            }
        }

        Ok(places)
    })
    .await
}

pub async fn open_in_terminal(path: String) -> Result<(), String> {
    fastlane(move || {
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("cmd")
                .current_dir(&path)
                .spawn()
                .map_err(|e| format!("Lỗi khi mở terminal: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            let terms = [
                "gnome-terminal",
                "konsole",
                "xfce4-terminal",
                "xterm",
                "alacritty",
                "kitty",
            ];
            let mut success = false;
            for term in terms {
                if Command::new(term).current_dir(&path).spawn().is_ok() {
                    success = true;
                    break;
                }
            }
            if !success {
                return Err(
                    "Không tìm thấy terminal hỗ trợ (đã thử gnome-terminal, konsole, xfce4-terminal, xterm).".into(),
                );
            }
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(&path)
                .spawn()
                .map_err(|e| format!("Lỗi khi mở terminal: {}", e))?;
        }

        Ok(())
    })
    .await
}

pub async fn fs_get_thumbnail(path: String) -> Result<String, String> {
    fastlane(move || {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use std::io::Cursor;
        use std::path::Path;

        let actual_path = if path.starts_with("Local::") {
            path.strip_prefix("Local::").unwrap().to_string()
        } else {
            path.clone()
        };

        let ext = Path::new(&actual_path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        match ext.as_str() {
            "mp4" | "mkv" | "avi" | "mov" | "webm" => {
                let output = Command::new("ffmpegthumbnailer")
                    .args(["-i", &actual_path, "-o", "-", "-s", "64", "-c", "jpeg", "-f"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let base64_str = STANDARD.encode(&out.stdout);
                        return Ok(format!("data:image/jpeg;base64,{}", base64_str));
                    }
                }
            }
            "pdf" => {
                let output = Command::new("pdftoppm")
                    .args([
                        "-jpeg",
                        "-f",
                        "1",
                        "-l",
                        "1",
                        "-singlefile",
                        "-scale-to",
                        "64",
                        &actual_path,
                    ])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let base64_str = STANDARD.encode(&out.stdout);
                        return Ok(format!("data:image/jpeg;base64,{}", base64_str));
                    }
                }
            }
            _ => {}
        }

        let img = image::open(&actual_path).map_err(|e| format!("Lỗi mở ảnh: {}", e))?;
        let thumb = img.thumbnail(64, 64);

        let mut buffer = Cursor::new(Vec::new());
        thumb
            .write_to(&mut buffer, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Lỗi tạo thumb: {}", e))?;

        let base64_str = STANDARD.encode(buffer.into_inner());
        Ok(format!("data:image/jpeg;base64,{}", base64_str))
    })
    .await
}

pub fn fs_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

/// Tên hàm: fs_chmod
/// Mô tả: Đổi quyền (mode) của một file/thư mục trên ổ Local.
/// Chỉ hỗ trợ Unix — remote cloud không có khái niệm mode POSIX.
/// IPC cũ giữ: wrapper mặc định `AllowSystem` (hành vi sudo tự động cũ).
pub async fn fs_chmod(path: String, mode: u32) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::perm::execute_chmod`.
    crate::actions::perm::execute_chmod(path, mode, Policy::AllowSystem).await
}

/// S2: bản tôn trọng policy — `Deny`/`AskOnce` trả `PERMISSION_CONSENT` để park.
pub async fn fs_chmod_with_policy(path: String, mode: u32, policy: Policy) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::perm::execute_chmod`.
    crate::actions::perm::execute_chmod(path, mode, policy).await
}

/// Tên hàm: fs_chown
/// Mô tả: Đổi chủ sở hữu (uid/gid) của một file/thư mục trên ổ Local.
/// Thao tác này gần như luôn cần quyền root nên đi thẳng qua `pkexec chown`.
/// IPC cũ giữ: wrapper mặc định `AllowSystem` (hành vi pkexec cũ).
pub async fn fs_chown(path: String, uid: u32, gid: u32) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::perm::execute_chown`.
    crate::actions::perm::execute_chown(path, uid, gid, Policy::AllowSystem).await
}

/// S2: bản tôn trọng policy — `Deny`/`AskOnce` trả `PERMISSION_CONSENT` để park.
pub async fn fs_chown_with_policy(path: String, uid: u32, gid: u32, policy: Policy) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::perm::execute_chown`.
    crate::actions::perm::execute_chown(path, uid, gid, policy).await
}
