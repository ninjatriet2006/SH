/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints thao tác File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend (tham số đường dẫn gộp chung kiểu Remote::/Path), gọi tầng `logic` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Gọi `logic::file_ops`, `logic::transfer`.
*/

use serde::{Deserialize, Serialize};

use crate::core::rclone;
use crate::core::task::blocking;
use crate::logic::app_state::AppState;
use crate::logic::file_ops;
use crate::logic::transfer;
use crate::logic::watcher;
use std::process::Command;
use tauri::{Manager, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct ConflictInfo {
    pub relative_path: String,
    pub src_full_path: String,
    pub dest_full_path: String,
}

#[tauri::command]
pub async fn fs_check_conflicts(
    app_handle: tauri::AppHandle,
    srcs: Vec<String>,
    dest_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    file_ops::check_conflicts(app_handle, srcs, dest_path).await
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RcloneFile {
    pub Path: String,
    pub Name: String,
    pub Size: i64,
    pub MimeType: String,
    pub ModTime: String,
    pub IsDir: bool,
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
    size: u64,
    file_count: u64,
    dir_count: u64,
    permissions: u32,
    uid: u32,
    gid: u32,
}

#[derive(serde::Deserialize)]
struct RcloneSizeOutput {
    count: u64,
    bytes: u64,
}

#[derive(serde::Serialize)]
pub struct SearchResultItem {
    item: FileItem,
    path: String,
}

#[tauri::command]
pub async fn list_files(
    app_handle: tauri::AppHandle,
    path: String,
    pane: Option<String>,
) -> Result<Vec<FileItem>, String> {
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let safe_path = if remote == "Local" && real_path.is_empty() {
        "/"
    } else {
        &real_path
    };
    let target = rclone::build_target(&remote, safe_path);

    // Cập nhật inotify watcher theo thư mục pane này đang xem.
    // Chỉ ổ Local mới theo dõi được; remote cloud thì ngừng theo dõi.
    if let Some(pane) = pane.as_deref() {
        let state = app_handle.state::<AppState>();
        let to_watch = if remote == "Local" { Some(safe_path) } else { None };
        watcher::watch_pane(&state, pane, to_watch);
    }

    let files = blocking(move || {
        let output = rclone::run_cmd(&["lsjson", &target, "--max-depth", "1"])?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Lỗi liệt kê file '{}': {}", target, err_msg));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() {
            return Ok(Vec::new());
        }

        let parsed_files: Vec<RcloneFile> =
            serde_json::from_str(&json_str).map_err(|e| format!("Lỗi phân tích JSON rclone_files: {}", e))?;

        Ok(parsed_files)
    })
    .await?;

    let mut files: Vec<FileItem> = files
        .into_iter()
        .map(|f| FileItem {
            uuid: f.Path,
            name: f.Name,
            size: f.Size,
            is_dir: f.IsDir,
            mod_time: f.ModTime,
            file_type: if f.MimeType.is_empty() { None } else { Some(f.MimeType) },
        })
        .collect();

    files.sort_by(|a, b| match (b.is_dir, a.is_dir) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(files)
}

#[tauri::command]
pub async fn fs_mkdir(path: String) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);

        file_ops::run_with_sudo_fallback(&remote, "mkdir", &[real_path.clone()], || {
            let output = rclone::run_cmd(&["mkdir", &target])?;
            if !output.status.success() {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            } else {
                Ok(())
            }
        })
    })
    .await
}

#[tauri::command]
pub async fn fs_delete(path: String) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);

        // Xác định kiểu của target trước, thay vì khớp chuỗi thông điệp lỗi của
        // rclone ("is a file not a directory") — cách đó vỡ nếu rclone đổi wording
        // hoặc chạy dưới locale khác.
        let is_dir = rclone::is_dir(&target).unwrap_or(true);

        file_ops::run_with_sudo_fallback(&remote, "rm", &[real_path.clone()], || {
            // `purge` xoá đệ quy thư mục; `deletefile` xoá đúng một file.
            let cmd = if is_dir { "purge" } else { "deletefile" };
            let output = rclone::run_cmd(&[cmd, &target])?;
            if output.status.success() {
                return Ok(());
            }

            // Phòng trường hợp phán đoán kiểu sai (ví dụ remote trả metadata lạ):
            // thử lệnh còn lại một lần nữa trước khi báo lỗi.
            let fallback = if is_dir { "deletefile" } else { "purge" };
            let retry = rclone::run_cmd(&[fallback, &target])?;
            if retry.status.success() {
                return Ok(());
            }

            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        })
    })
    .await
}

#[tauri::command]
pub async fn fs_touch(path: String) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);

        if remote == "Local" {
            std::fs::File::create(&target).map_err(|e| e.to_string())?;
        } else {
            rclone::spawn_cmd(&["touch", &target])?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn fs_rename(old_path: String, new_path: String) -> Result<(), String> {
    blocking(move || {
        let (remote, old_real) = file_ops::parse_remote_path(&old_path);
        let (_, new_real) = file_ops::parse_remote_path(&new_path);

        let src = rclone::build_target(&remote, &old_real);
        let dst = rclone::build_target(&remote, &new_real);

        file_ops::run_with_sudo_fallback(&remote, "mv", &[old_real.clone(), new_real.clone()], || {
            let output = rclone::run_cmd(&["moveto", &src, &dst])?;
            if !output.status.success() {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            } else {
                Ok(())
            }
        })
    })
    .await
}

#[tauri::command]
pub async fn fs_copy(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    src: String,
    dst: String,
    task_id: Option<u32>,
) -> Result<(), String> {
    let (src_remote, src_real) = file_ops::parse_remote_path(&src);
    let (dst_remote, dst_real) = file_ops::parse_remote_path(&dst);

    let src_target = rclone::build_target(&src_remote, &src_real);
    let dst_target = rclone::build_target(&dst_remote, &dst_real);

    // Chạy tiến trình copy chính (có báo tiến độ). Nếu thất bại do thiếu quyền
    // và cả hai đầu đều là Local, thử lại một lần qua pkexec (`cp -r`).
    let result = transfer::run_transfer_task(app_handle, state, "copyto", src_target, dst_target, task_id).await;

    match result {
        Ok(()) => Ok(()),
        Err(e) if src_remote == "Local" && dst_remote == "Local" => {
            file_ops::run_with_sudo_fallback("Local", "cp", &[src_real.clone(), dst_real.clone()], || Err(e))
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn fs_move(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    src: String,
    dst: String,
    task_id: Option<u32>,
) -> Result<(), String> {
    let (src_remote, src_real) = file_ops::parse_remote_path(&src);
    let (dst_remote, dst_real) = file_ops::parse_remote_path(&dst);

    let src_target = rclone::build_target(&src_remote, &src_real);
    let dst_target = rclone::build_target(&dst_remote, &dst_real);

    let result = transfer::run_transfer_task(app_handle, state, "moveto", src_target, dst_target, task_id).await;

    match result {
        Ok(()) => Ok(()),
        Err(e) if src_remote == "Local" && dst_remote == "Local" => {
            file_ops::run_with_sudo_fallback("Local", "mv", &[src_real.clone(), dst_real.clone()], || Err(e))
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn fs_cancel(state: State<'_, AppState>, task_id: u32) -> Result<(), String> {
    transfer::cancel_transfer(state, task_id)
}

#[tauri::command]
pub async fn fs_stat_advanced(path: String) -> Result<StatInfo, String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);
        let output = rclone::run_cmd(&["size", &target, "--json"])?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }

        let parsed: RcloneSizeOutput =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("Lỗi phân tích JSON rclone size: {}", e))?;

        // Đếm số thư mục con (đệ quy). Lệnh sẽ lỗi nếu target là file → coi như 0.
        let dir_count = match rclone::run_cmd(&["lsjson", "-R", "--dirs-only", &target]) {
            Ok(out) if out.status.success() => serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
                .map(|v| v.len() as u64)
                .unwrap_or(0),
            _ => 0,
        };

        // Quyền/chủ sở hữu chỉ có ý nghĩa trên ổ Local.
        let (permissions, uid, gid) = read_local_ownership(&remote, &target);

        Ok(StatInfo {
            size: parsed.bytes,
            file_count: parsed.count,
            dir_count,
            permissions,
            uid,
            gid,
        })
    })
    .await
}

/// Đọc mode/uid/gid thật của một đường dẫn Local. Trả về (0, 0, 0) trên các
/// remote đám mây hoặc trên hệ điều hành không hỗ trợ khái niệm này.
fn read_local_ownership(remote: &str, target: &str) -> (u32, u32, u32) {
    if remote != "Local" {
        return (0, 0, 0);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(meta) = std::fs::metadata(target) {
            return (meta.mode(), meta.uid(), meta.gid());
        }
    }

    #[cfg(not(unix))]
    let _ = target;

    (0, 0, 0)
}

#[tauri::command]
pub async fn fs_search(path: String, query: String) -> Result<Vec<SearchResultItem>, String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);
        let filter = format!("*{}*", query);

        let output = rclone::run_cmd(&["lsjson", &target, "-R", "--include", &filter, "--files-only"])?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }

        let items: Vec<serde_json::Value> =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("Lỗi phân tích JSON khi tìm kiếm: {}", e))?;

        let mut files = Vec::new();
        for item in items {
            let name = item["Name"].as_str().unwrap_or("").to_string();
            let rel_path = item["Path"].as_str().unwrap_or("").to_string();

            let file_path = if real_path.ends_with('/') {
                format!("{}{}", real_path, rel_path)
            } else {
                format!("{}/{}", real_path, rel_path)
            };

            let size = item["Size"].as_i64().unwrap_or(0);
            let is_dir = item["IsDir"].as_bool().unwrap_or(false);
            let mod_time = item["ModTime"].as_str().unwrap_or("").to_string();

            let file_info = FileItem {
                uuid: file_path.clone(),
                name,
                is_dir,
                size,
                mod_time,
                file_type: None,
            };

            let ui_path = if remote == "Local" {
                format!("Local::{}", file_path)
            } else {
                format!("{}::{}", remote, file_path)
            };

            files.push(SearchResultItem {
                item: file_info,
                path: ui_path,
            });
        }

        Ok(files)
    })
    .await
}

#[tauri::command]
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
#[tauri::command]
pub async fn get_user_places() -> Result<Vec<UserPlace>, String> {
    blocking(|| {
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

#[tauri::command]
pub async fn open_in_terminal(path: String) -> Result<(), String> {
    blocking(move || {
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
                if let Ok(_) = Command::new(term).current_dir(&path).spawn() {
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

#[tauri::command]
pub async fn fs_get_thumbnail(path: String) -> Result<String, String> {
    blocking(move || {
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

/// Tên hàm: fs_read_text
/// Mô tả: Đọc nội dung văn bản của một file (Local hoặc remote) qua `rclone cat`.
/// Giới hạn kích thước để không kéo cả file lớn vào bộ nhớ / IPC.
#[tauri::command]
pub async fn fs_read_text(path: String, max_bytes: Option<u64>) -> Result<String, String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);

        // Mặc định 1 MiB — đủ cho tệp cấu hình / ghi chú, tránh treo UI với file lớn.
        let limit = max_bytes.unwrap_or(1_048_576);
        let limit_arg = limit.to_string();

        let output = rclone::run_cmd(&["cat", &target, "--count", &limit_arg])?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if err.is_empty() {
                format!("Không đọc được '{}': {}", path, output.status)
            } else {
                err
            });
        }

        // Từ chối nội dung nhị phân thay vì trả về chuỗi lộn xộn.
        String::from_utf8(output.stdout)
            .map_err(|_| "Tệp không phải văn bản UTF-8 nên không thể mở bằng trình xem văn bản.".to_string())
    })
    .await
}

/// Tên hàm: fs_write_text
/// Mô tả: Ghi nội dung văn bản vào một file (Local hoặc remote) qua `rclone rcat`.
/// `rcat` đọc từ stdin nên hoạt động đồng nhất cho mọi backend.
#[tauri::command]
pub async fn fs_write_text(path: String, content: String) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        let target = rclone::build_target(&remote, &real_path);

        file_ops::run_with_sudo_fallback(&remote, "write", &[real_path.clone()], || {
            let output = rclone::run_cmd_with_stdin(&["rcat", &target], content.as_bytes())?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    format!("Ghi tệp thất bại: {}", output.status)
                } else {
                    err
                });
            }
            Ok(())
        })
    })
    .await
}

#[tauri::command]
pub fn fs_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

/// Tên hàm: fs_chmod
/// Mô tả: Đổi quyền (mode) của một file/thư mục trên ổ Local.
/// Chỉ hỗ trợ Unix — remote cloud không có khái niệm mode POSIX.
#[tauri::command]
pub async fn fs_chmod(path: String, mode: u32) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        if remote != "Local" {
            return Err(format!(
                "Không thể đổi quyền trên remote '{}' — chỉ hỗ trợ ổ Local.",
                remote
            ));
        }

        #[cfg(unix)]
        {
            // Chỉ giữ 12 bit quyền (bao gồm setuid/setgid/sticky) để không ghi đè
            // các bit loại file trong st_mode.
            let safe_mode = mode & 0o7777;
            let octal = format!("{:o}", safe_mode);

            file_ops::run_with_sudo_fallback("Local", "chmod", &[octal.clone(), real_path.clone()], || {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&real_path, std::fs::Permissions::from_mode(safe_mode))
                    .map_err(|e| e.to_string())
            })
        }

        #[cfg(not(unix))]
        {
            let _ = (real_path, mode);
            Err("Đổi quyền chỉ được hỗ trợ trên hệ điều hành Unix.".to_string())
        }
    })
    .await
}

/// Tên hàm: fs_chown
/// Mô tả: Đổi chủ sở hữu (uid/gid) của một file/thư mục trên ổ Local.
/// Thao tác này gần như luôn cần quyền root nên đi thẳng qua `pkexec chown`.
#[tauri::command]
pub async fn fs_chown(path: String, uid: u32, gid: u32) -> Result<(), String> {
    blocking(move || {
        let (remote, real_path) = file_ops::parse_remote_path(&path);
        if remote != "Local" {
            return Err(format!(
                "Không thể đổi chủ sở hữu trên remote '{}' — chỉ hỗ trợ ổ Local.",
                remote
            ));
        }

        #[cfg(target_os = "linux")]
        {
            let spec = format!("{}:{}", uid, gid);
            let output = Command::new("pkexec")
                .args(["chown", &spec, &real_path])
                .output()
                .map_err(|e| format!("Lỗi gọi pkexec: {}", e))?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    "Thao tác pkexec bị huỷ hoặc lỗi phân quyền.".to_string()
                } else {
                    err
                });
            }
            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (real_path, uid, gid);
            Err("Đổi chủ sở hữu chỉ được hỗ trợ trên Linux.".to_string())
        }
    })
    .await
}
