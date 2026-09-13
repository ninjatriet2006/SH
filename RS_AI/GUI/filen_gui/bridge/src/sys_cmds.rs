//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến thao tác hệ thống (System).
//! Trách nhiệm: Quản lý clipboard OS, mở file bằng app ngoài, lấy danh sách app và chạy custom actions.
//! Tương tác: Thao tác thông qua OS bindings (gtk/winapi) hoặc thư viện chuẩn `std::process::Command`.

use crate::{
    auth_cmds::Empty,
    contract::{backend, success, validate, IpcResult, Req},
    state::{AppState, OSClipboardData},
};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct ClipboardSetReq {
    paths: Vec<String>,
    is_cut: bool,
}

#[derive(Deserialize)]
pub struct OpenWithReq {
    path: String,
    exec_cmd: String,
}

#[derive(Deserialize)]
pub struct CustomActionReq {
    exec_template: String,
    file_paths: Vec<String>,
}

#[derive(Deserialize)]
pub struct PathReq {
    path: String,
}

/// Lấy dữ liệu từ Clipboard của hệ điều hành. Chỉ khả dụng cho Linux.
#[cfg(target_os = "linux")]
#[tauri::command]
pub fn os_clipboard_get(request: Req<Empty>, app: tauri::AppHandle) -> IpcResult<Option<OSClipboardData>> {
    let (request_id, _) = validate(request)?;
    let (tx, rx) = std::sync::mpsc::channel();

    // Yêu cầu chạy trên main thread của UI (bắt buộc đối với GTK)
    app.run_on_main_thread(move || {
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        let mut result = None;

        // Thử lấy kiểu dữ liệu copy-paste chuyên biệt của GNOME
        if let Some(selection) = clipboard.wait_for_contents(&gdk::Atom::intern("x-special/gnome-copied-files")) {
            let data = selection.data();
            let s = String::from_utf8_lossy(&data).to_string();
            result = Some(s);
        // Fallback lấy định dạng danh sách URI phổ quát
        } else if let Some(selection) = clipboard.wait_for_contents(&gdk::Atom::intern("text/uri-list")) {
            let data = selection.data();
            let s = String::from_utf8_lossy(&data).to_string();
            // URI list mặc định không chứa cờ copy/cut, ta ngầm định là copy
            result = Some(format!("copy\n{}", s));
        }
        let _ = tx.send(result);
    })
    .map_err(|e| backend(e.to_string()))?;

    // Chờ nhận kết quả từ main thread
    if let Ok(Some(s)) = rx.recv() {
        let mut lines = s.lines();
        // Dòng đầu tiên định nghĩa chế độ (cut hay copy)
        let mode = match lines.next() {
            Some("cut") => "cut".to_string(),
            _ => "copy".to_string(),
        };
        let mut paths = Vec::new();
        // Xử lý các dòng còn lại (là các đường dẫn file)
        for line in lines {
            let line = line.trim();
            if line.starts_with("file://") {
                let p = line.strip_prefix("file://").unwrap();
                // Giải mã các ký tự đặc biệt trong URI (ví dụ %20 thành khoảng trắng)
                if let Ok(decoded) = urlencoding::decode(p) {
                    paths.push(decoded.into_owned());
                } else {
                    paths.push(p.to_string());
                }
            } else if !line.is_empty() {
                paths.push(line.to_string());
            }
        }

        if paths.is_empty() {
            Ok(success(request_id, None))
        } else {
            Ok(success(request_id, Some(OSClipboardData { mode, paths })))
        }
    } else {
        Ok(success(request_id, None)) // Không có file nào trong clipboard
    }
}

/// Fallback trả về rỗng cho các hệ điều hành không phải Linux.
#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn os_clipboard_get(request: Req<Empty>) -> IpcResult<Option<OSClipboardData>> {
    let (request_id, _) = validate(request)?;
    Ok(success(request_id, None))
}

/// Thiết lập dữ liệu vào Clipboard của hệ điều hành. Chỉ khả dụng cho Linux.
#[cfg(target_os = "linux")]
#[tauri::command]
pub fn os_clipboard_set(request: Req<ClipboardSetReq>, app: tauri::AppHandle) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    let action = if request.is_cut { "cut" } else { "copy" };
    let paths = request.paths;
    // Dựng nội dung payload chuẩn của GNOME clipboard
    let mut payload = format!("{}\n", action);
    for path in &paths {
        payload.push_str(&format!("file://{}\n", path));
    }
    let payload = payload.trim_end().to_string(); // Loại bỏ ký tự xuống dòng dư thừa ở cuối

    app.run_on_main_thread(move || {
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        let targets = vec![
            gtk::TargetEntry::new("x-special/gnome-copied-files", gtk::TargetFlags::empty(), 0),
            gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::empty(), 1),
        ];

        // Thiết lập callback để Hệ điều hành gọi khi ứng dụng khác muốn dán (paste) dữ liệu này
        clipboard.set_with_data(&targets, move |_cb, sel_data, info| {
            if info == 0 {
                let data = payload.clone().into_bytes();
                sel_data.set(&gdk::Atom::intern("x-special/gnome-copied-files"), 8, &data);
            } else if info == 1 {
                let mut uri_list = String::new();
                for path in &paths {
                    uri_list.push_str(&format!("file://{}\r\n", path));
                }
                let data = uri_list.into_bytes();
                sel_data.set(&gdk::Atom::intern("text/uri-list"), 8, &data);
            }
        });
    })
    .map_err(|e| backend(e.to_string()))?;
    Ok(success(request_id, ()))
}

/// Bỏ qua thiết lập clipboard cho hệ điều hành không phải Linux (chưa hỗ trợ).
#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn os_clipboard_set(request: Req<ClipboardSetReq>) -> IpcResult<()> {
    let (request_id, _) = validate(request)?;
    Ok(success(request_id, ()))
}

/// Trả về danh sách các ứng dụng (app) đã cài đặt trên máy.
#[tauri::command]
pub fn sys_list_apps(request: Req<Empty>) -> IpcResult<Vec<filen_gui::sys::DesktopApp>> {
    let (request_id, _) = validate(request)?;
    #[cfg(target_os = "linux")]
    {
        Ok(success(request_id, filen_gui::sys::get_desktop_apps()))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(success(request_id, vec![]))
    }
}

/// Mở file cụ thể với lệnh thực thi (app ngoài) do người dùng chỉ định.
#[tauri::command]
pub fn sys_open_with(
    request: Req<OpenWithReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    let path = state.picker_roots.existing(window.label(), Path::new(&request.path))?;
    // exec_cmd truyền vào đã được làm sạch các ký tự giữ chỗ (placeholder) như "%f", "%F"
    let mut parts = request.exec_cmd.split_whitespace();
    if let Some(bin) = parts.next() {
        let mut cmd = std::process::Command::new(bin);
        // Nạp các tham số thừa
        for arg in parts {
            cmd.arg(arg);
        }
        // Nạp tham số cuối cùng là đường dẫn file
        cmd.arg(&path);
        // Gọi ứng dụng ngoài
        cmd.spawn().map_err(|e| backend(e.to_string()))?;
    }
    Ok(success(request_id, ()))
}

/// Lấy danh sách các thao tác ngữ cảnh tùy chỉnh (Custom Context Menu Actions).
#[tauri::command]
pub fn sys_get_custom_actions(request: Req<Empty>) -> IpcResult<Vec<filen_gui::sys::CustomAction>> {
    let (request_id, _) = validate(request)?;
    #[cfg(target_os = "linux")]
    {
        Ok(success(request_id, filen_gui::sys::get_custom_actions()))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(success(request_id, vec![]))
    }
}

/// Khởi chạy một thao tác tùy chỉnh (custom action) với các tham số file hiện tại.
#[tauri::command]
pub fn sys_execute_custom_action(
    request: Req<CustomActionReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    let paths = request
        .file_paths
        .iter()
        .map(|path| state.picker_roots.existing(window.label(), Path::new(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let paths_str = paths
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    // Điền danh sách đường dẫn vào mẫu lệnh thay cho %F và %f
    let exec_cmd = request
        .exec_template
        .replace("%F", &paths_str)
        .replace("%f", &paths_str);

    // Tách lệnh khởi chạy và tham số
    let mut parts = exec_cmd.split_whitespace();
    if let Some(bin) = parts.next() {
        let mut cmd = std::process::Command::new(bin);
        for arg in parts {
            cmd.arg(arg);
        }
        // Khởi chạy tiến trình dưới nền
        cmd.spawn().map_err(|e| backend(e.to_string()))?;
    }
    Ok(success(request_id, ()))
}

#[tauri::command]
pub async fn open_in_terminal(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (request_id, request) = validate(request)?;
    let path = state.picker_roots.existing(window.label(), Path::new(&request.path))?;
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        Command::new("wt.exe")
            .arg("-d")
            .arg(&path)
            .spawn()
            .map_err(|e| backend(format!("Failed to open terminal: {}", e)))?;
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
            return Err(crate::contract::unavailable(
                "No supported terminal found (tried gnome-terminal, konsole, xfce4-terminal, xterm).",
            ));
        }
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&path)
            .spawn()
            .map_err(|e| backend(format!("Failed to open terminal: {}", e)))?;
    }

    Ok(success(request_id, ()))
}
