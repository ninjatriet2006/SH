/*
[INTEGRITY NOTES]
- Mục đích: S2 gom món hệ điều hành (home/tmp/XDG places/terminal).
- Trách nhiệm: logic thuần OS, không đụng rclone/file người dùng.
- Tương tác: `api::files_view` chỉ là command mỏng gọi sang (struct `UserPlace` ở đây).
*/

// UNIVERSAL: món hệ điều hành — đọc $HOME/tmp/XDG, mở terminal; không động rclone/file người dùng.
use crate::logic::fastlane::fastlane;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Một vị trí truy cập nhanh trong sidebar (DTO gốc ở thợ `system`).
#[derive(Serialize)]
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

/// Danh sách thư mục chứa Desktop Entry theo chuẩn XDG, xếp theo thứ tự ưu tiên
/// (mục của người dùng ghi đè mục hệ thống nếu cùng tên file).
pub(crate) fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let home = std::env::var("HOME").unwrap_or_default();
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}/.local/share", home));

    // Thư mục của người dùng đứng trước để ưu tiên ghi đè.
    dirs.push(PathBuf::from(&data_home).join("applications"));
    dirs.push(PathBuf::from(&data_home).join("flatpak/exports/share/applications"));

    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string());
    for d in data_dirs.split(':').filter(|s| !s.is_empty()) {
        dirs.push(PathBuf::from(d).join("applications"));
    }

    // Flatpak/Snap không luôn nằm trong XDG_DATA_DIRS.
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));

    dirs
}

/// UNIVERSAL: DTO "Open With" — 1 nơi làm thật ở thợ `system` (đã dời từ `view`
/// + xóa lớp trung gian cũ; JSON giữ nguyên name/exec/icon).
#[derive(Serialize)]
pub struct DesktopApp {
    /// Tên hiển thị của ứng dụng.
    pub name: String,
    /// Lệnh thực thi của ứng dụng.
    pub exec: String,
    /// Đường dẫn hoặc tên icon của ứng dụng.
    pub icon: String,
}

/// Kết quả phân tích một file `.desktop`.
struct Entry {
    name: String,
    exec: String,
    icon: String,
}

/// UNIVERSAL: phân tích nội dung một file `.desktop`, chỉ đọc section `[Desktop Entry]`.
/// Đọc 1 file `.desktop` → 1 app (name/exec/icon).
/// Trả `None` nếu mục không phù hợp để hiện trong "Open With".
fn read_desktop_app(content: &str) -> Option<Entry> {
    let mut in_main_section = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut entry_type = String::new();
    let mut hidden = false;
    let mut needs_terminal = false;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            // Gặp section mới: chỉ xử lý `[Desktop Entry]`, dừng khi sang section khác.
            if in_main_section {
                break;
            }
            in_main_section = line == "[Desktop Entry]";
            continue;
        }
        if !in_main_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        // Bỏ qua khoá có locale (`Name[vi]`) để lấy đúng bản mặc định.
        match key {
            "Name" if name.is_empty() => name = value.to_string(),
            "Exec" if exec.is_empty() => exec = value.to_string(),
            "Icon" if icon.is_empty() => icon = value.to_string(),
            "Type" => entry_type = value.to_string(),
            "NoDisplay" | "Hidden" => {
                if value.eq_ignore_ascii_case("true") {
                    hidden = true;
                }
            }
            "Terminal" if value.eq_ignore_ascii_case("true") => needs_terminal = true,
            // UNIVERSAL: khóa lạ ngoài whitelist (Comment/Keywords/Name[vi]...) —
            // không phải lỗi nên bỏ qua im lặng, khỏi warn đầy sổ.
            _ => {}
        }
    }

    if hidden || needs_terminal {
        return None;
    }
    // Chỉ nhận Type=Application; Link/Directory không mở được file.
    if !entry_type.is_empty() && entry_type != "Application" {
        return None;
    }
    if name.is_empty() || exec.is_empty() {
        return None;
    }

    Some(Entry { name, exec, icon })
}

/// UNIVERSAL: trả về danh sách ứng dụng mở file được, sắp theo tên.
pub fn get_open_list() -> Vec<DesktopApp> {
    // Khoá theo tên file .desktop để thư mục ưu tiên cao ghi đè thư mục thấp hơn.
    let mut found: HashMap<String, Entry> = HashMap::new();

    for dir in application_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                continue;
            }
            let Some(file_id) = path.file_name().and_then(|s| s.to_str()).map(str::to_string) else {
                continue;
            };
            if found.contains_key(&file_id) {
                continue; // Đã có bản ưu tiên cao hơn
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(parsed) = read_desktop_app(&content) {
                found.insert(file_id, parsed);
            }
        }
    }

    let mut apps: Vec<DesktopApp> = found
        .into_values()
        .map(|e| DesktopApp {
            name: e.name,
            exec: e.exec,
            icon: e.icon,
        })
        .collect();

    apps.sort_by_key(|a| a.name.to_lowercase());

    // Lựa chọn mặc định luôn đứng đầu.
    apps.insert(
        0,
        DesktopApp {
            name: "Mặc định hệ thống (xdg-open)".to_string(),
            exec: "xdg-open".to_string(),
            icon: String::new(),
        },
    );

    apps
}

/// UNIVERSAL: hỏi hệ điều hành ứng dụng mặc định cho một file (`xdg-mime query`).
pub fn which_default(path: &Path) -> Option<String> {
    let mime = std::process::Command::new("xdg-mime")
        .args(["query", "filetype", &path.to_string_lossy()])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())?;

    let desktop_id = std::process::Command::new("xdg-mime")
        .args(["query", "default", &mime])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())?;

    for dir in application_dirs() {
        let candidate = dir.join(&desktop_id);
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            if let Some(entry) = read_desktop_app(&content) {
                return Some(entry.exec);
            }
        }
    }
    None
}

/// UNIVERSAL: quét Desktop Entry thật (chuẩn FreeDesktop.org) thay vì dữ liệu giả.
pub async fn sys_list_apps() -> Result<Vec<DesktopApp>, String> {
    fastlane(|| Ok(get_open_list())).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_main_section_exec() {
        // firefox.desktop có thêm [Desktop Action ...] với Exec riêng — không được nhặt.
        let content = "\
[Desktop Entry]
Name=Firefox Web Browser
Name[vi]=Trình duyệt Firefox
Exec=firefox %u
Icon=firefox
Terminal=false
Type=Application

[Desktop Action new-window]
Name=Open a New Window
Exec=firefox -new-window
";
        let e = read_desktop_app(content).expect("phải phân tích được");
        assert_eq!(e.name, "Firefox Web Browser");
        assert_eq!(e.exec, "firefox %u");
        assert_eq!(e.icon, "firefox");
    }

    #[test]
    fn skips_hidden_and_terminal_apps() {
        let hidden = "[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n";
        assert!(read_desktop_app(hidden).is_none());

        let hidden2 = "[Desktop Entry]\nName=X\nExec=x\nHidden=TRUE\n";
        assert!(read_desktop_app(hidden2).is_none());

        let term = "[Desktop Entry]\nName=btop++\nExec=btop\nTerminal=true\n";
        assert!(read_desktop_app(term).is_none());
    }

    #[test]
    fn skips_non_application_and_incomplete() {
        let link = "[Desktop Entry]\nName=X\nExec=x\nType=Link\n";
        assert!(read_desktop_app(link).is_none());

        let no_exec = "[Desktop Entry]\nName=X\nType=Application\n";
        assert!(read_desktop_app(no_exec).is_none());

        let no_name = "[Desktop Entry]\nExec=x\nType=Application\n";
        assert!(read_desktop_app(no_name).is_none());
    }

    #[test]
    fn ignores_comments_and_localized_name() {
        let content = "\
# comment
[Desktop Entry]
Name[ja]=ローカライズ
Name=Real Name
Exec=app %f
Type=Application
";
        let e = read_desktop_app(content).unwrap();
        assert_eq!(e.name, "Real Name");
    }

    #[test]
    fn list_always_includes_xdg_open_first() {
        let apps = get_open_list();
        assert!(!apps.is_empty());
        assert_eq!(apps[0].exec, "xdg-open");
    }
}
