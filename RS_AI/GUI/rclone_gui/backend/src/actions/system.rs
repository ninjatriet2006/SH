/*
[INTEGRITY NOTES]
- Mục đích: S2 gom món hệ điều hành (home/tmp/XDG places/terminal).
- Trách nhiệm: logic thuần OS, không đụng rclone/file người dùng.
- Tương tác: `api::files` chỉ là wrapper mỏng; IPC + struct `UserPlace` giữ ở `api`.
*/

// UNIVERSAL: món hệ điều hành — đọc $HOME/tmp/XDG, mở terminal; không động rclone/file người dùng.
use crate::api::files::UserPlace;
use crate::logic::fastlane::fastlane;
use std::process::Command;

/// UNIVERSAL: trả đúng $HOME (rơi về "/" khi thiếu).
pub async fn get_home_dir() -> Result<String, String> {
    Ok(std::env::var("HOME").unwrap_or_else(|_| "/".to_string()))
}

/// UNIVERSAL: thư mục tmp hệ điều hành.
pub fn fs_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

/// UNIVERSAL: tra một thư mục XDG bằng `xdg-user-dir`; `None` khi thiếu/trùng $HOME.
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

/// UNIVERSAL: danh sách thư mục người dùng chuẩn XDG cho mục "Truy cập nhanh".
/// Chỉ trả thư mục thật sự tồn tại.
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

/// UNIVERSAL: mở terminal hệ điều hành tại `path` (thử danh sách terminal Linux).
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
