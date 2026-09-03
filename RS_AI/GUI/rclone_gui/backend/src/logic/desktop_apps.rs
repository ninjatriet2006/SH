/*
[INTEGRITY NOTES]
- Mục đích: Đọc danh sách ứng dụng đã cài từ Desktop Entry (chuẩn FreeDesktop.org).
- Trách nhiệm: Quét các thư mục `applications/`, phân tích `.desktop`, lọc mục bị ẩn
  và mục cần terminal, sắp xếp theo tên để hiển thị trong hộp thoại "Open With".
- Tương tác: Gọi từ `core::sys::sys_list_apps`.

Điểm cần lưu ý khi phân tích `.desktop` (đã kiểm chứng trên máy thật):
  * File có nhiều section: `[Desktop Entry]` rồi tới các `[Desktop Action ...]`.
    Chỉ được lấy `Exec`/`Name` trong `[Desktop Entry]` — nếu đọc cả file sẽ nhặt
    sai `Exec` của action (ví dụ firefox.desktop có 3 dòng `Exec=`).
  * `Name` có bản dịch (`Name[vi]=`, `Name[ja]=`...). Chỉ khớp đúng khoá `Name`.
  * `NoDisplay=true` / `Hidden=true` là mục không nên hiện cho người dùng.
  * `Terminal=true` cần chạy trong terminal — không dùng để mở file từ GUI.
*/

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::sys::DesktopApp;

/// Danh sách thư mục chứa Desktop Entry theo chuẩn XDG, xếp theo thứ tự ưu tiên
/// (mục của người dùng ghi đè mục hệ thống nếu cùng tên file).
fn application_dirs() -> Vec<PathBuf> {
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

/// Kết quả phân tích một file `.desktop`.
struct Entry {
    name: String,
    exec: String,
    icon: String,
}

/// Phân tích nội dung một file `.desktop`, chỉ đọc section `[Desktop Entry]`.
/// Trả `None` nếu mục không phù hợp để hiện trong "Open With".
fn parse_desktop_entry(content: &str) -> Option<Entry> {
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
            "Terminal" => {
                if value.eq_ignore_ascii_case("true") {
                    needs_terminal = true;
                }
            }
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

/// Tên hàm: list
/// Mô tả: Trả về danh sách ứng dụng có thể dùng để mở file, sắp theo tên.
/// Luôn có `xdg-open` ở đầu làm lựa chọn mặc định của hệ điều hành.
pub fn list() -> Vec<DesktopApp> {
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
            if let Some(parsed) = parse_desktop_entry(&content) {
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

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

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

/// Tên hàm: default_for_file
/// Mô tả: Hỏi hệ điều hành ứng dụng mặc định cho một file (`xdg-mime query`).
/// Trả về `Exec` của ứng dụng đó nếu tra được.
pub fn default_for_file(path: &Path) -> Option<String> {
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
            if let Some(entry) = parse_desktop_entry(&content) {
                return Some(entry.exec);
            }
        }
    }
    None
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
        let e = parse_desktop_entry(content).expect("phải phân tích được");
        assert_eq!(e.name, "Firefox Web Browser");
        assert_eq!(e.exec, "firefox %u");
        assert_eq!(e.icon, "firefox");
    }

    #[test]
    fn skips_hidden_and_terminal_apps() {
        let hidden = "[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n";
        assert!(parse_desktop_entry(hidden).is_none());

        let hidden2 = "[Desktop Entry]\nName=X\nExec=x\nHidden=TRUE\n";
        assert!(parse_desktop_entry(hidden2).is_none());

        let term = "[Desktop Entry]\nName=btop++\nExec=btop\nTerminal=true\n";
        assert!(parse_desktop_entry(term).is_none());
    }

    #[test]
    fn skips_non_application_and_incomplete() {
        let link = "[Desktop Entry]\nName=X\nExec=x\nType=Link\n";
        assert!(parse_desktop_entry(link).is_none());

        let no_exec = "[Desktop Entry]\nName=X\nType=Application\n";
        assert!(parse_desktop_entry(no_exec).is_none());

        let no_name = "[Desktop Entry]\nExec=x\nType=Application\n";
        assert!(parse_desktop_entry(no_name).is_none());
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
        let e = parse_desktop_entry(content).unwrap();
        assert_eq!(e.name, "Real Name");
    }

    #[test]
    fn list_always_includes_xdg_open_first() {
        let apps = list();
        assert!(!apps.is_empty());
        assert_eq!(apps[0].exec, "xdg-open");
    }
}
