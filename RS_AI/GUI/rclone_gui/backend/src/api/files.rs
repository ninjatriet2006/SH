/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints thao tác File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend (tham số đường dẫn gộp chung kiểu Remote::/Path), gọi tầng `logic` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Gọi `logic::file_ops`, `logic::transfer`.
*/

use serde::{Deserialize, Serialize};

use crate::logic::app_state::AppState;
use crate::logic::file_ops;
use crate::logic::transfer;
use crate::core::rclone;
use tauri::State;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct ConflictInfo {
    pub relative_path: String,
    pub src_full_path: String,
    pub dest_full_path: String,
}

#[tauri::command]
pub async fn fs_check_conflicts(app_handle: tauri::AppHandle, srcs: Vec<String>, dest_path: String) -> Result<Vec<ConflictInfo>, String> {
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
pub async fn list_files(path: String) -> Result<Vec<FileItem>, String> {
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let safe_path = if remote == "Local" && real_path.is_empty() { "/" } else { &real_path };
    let target = rclone::build_target(&remote, safe_path);

    let output = rclone::run_cmd(&["lsjson", &target, "--max-depth", "1"])?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi liệt kê file '{}': {}", target, err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed_files: Vec<RcloneFile> = serde_json::from_str(&json_str)
        .map_err(|e| format!("Lỗi phân tích JSON rclone_files: {}", e))?;

    let mut files: Vec<FileItem> = parsed_files.into_iter().map(|f| {
        FileItem {
            uuid: f.Path,
            name: f.Name,
            size: f.Size,
            is_dir: f.IsDir,
            mod_time: f.ModTime,
            file_type: if f.MimeType.is_empty() { None } else { Some(f.MimeType) }
        }
    }).collect();

    files.sort_by(|a, b| {
        match (b.is_dir, a.is_dir) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(files)
}

#[tauri::command]
pub async fn fs_mkdir(path: String) -> Result<(), String> {
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
}

#[tauri::command]
pub async fn fs_delete(path: String) -> Result<(), String> {
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let target = rclone::build_target(&remote, &real_path);
    
    file_ops::run_with_sudo_fallback(&remote, "rm", &[real_path.clone()], || {
        let output = rclone::run_cmd(&["purge", &target])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr).into_owned();
            if err_msg.contains("is a file not a directory") || err_msg.contains("not a directory") {
                let out2 = rclone::run_cmd(&["deletefile", &target])?;
                if !out2.status.success() {
                    return Err(String::from_utf8_lossy(&out2.stderr).into_owned());
                }
                return Ok(());
            }
            Err(err_msg)
        } else {
            Ok(())
        }
    })
}

#[tauri::command]
pub async fn fs_touch(path: String) -> Result<(), String> {
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let target = rclone::build_target(&remote, &real_path);
    
    if remote == "Local" {
        std::fs::File::create(&target).map_err(|e| e.to_string())?;
    } else {
        rclone::spawn_cmd(&["touch", &target])?;
    }
    Ok(())
}

#[tauri::command]
pub async fn fs_rename(old_path: String, new_path: String) -> Result<(), String> {
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
    let result = transfer::run_transfer_task(
        app_handle,
        state,
        "copyto",
        src_target,
        dst_target,
        task_id,
    )
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(e) if src_remote == "Local" && dst_remote == "Local" => {
            file_ops::run_with_sudo_fallback(
                "Local",
                "cp",
                &[src_real.clone(), dst_real.clone()],
                || Err(e),
            )
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

    let result = transfer::run_transfer_task(
        app_handle,
        state,
        "moveto",
        src_target,
        dst_target,
        task_id,
    )
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(e) if src_remote == "Local" && dst_remote == "Local" => {
            file_ops::run_with_sudo_fallback(
                "Local",
                "mv",
                &[src_real.clone(), dst_real.clone()],
                || Err(e),
            )
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
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let target = rclone::build_target(&remote, &real_path);
    let output = rclone::run_cmd(&["size", &target, "--json"])?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let parsed: RcloneSizeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Lỗi phân tích JSON rclone size: {}", e))?;

    // Đếm số thư mục con (đệ quy). Lệnh sẽ lỗi nếu target là file → coi như 0.
    let dir_count = match rclone::run_cmd(&["lsjson", "-R", "--dirs-only", &target]) {
        Ok(out) if out.status.success() => {
            serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout)
                .map(|v| v.len() as u64)
                .unwrap_or(0)
        }
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
    let (remote, real_path) = file_ops::parse_remote_path(&path);
    let target = rclone::build_target(&remote, &real_path);
    let filter = format!("*{}*", query);
    
    let output = rclone::run_cmd(&["lsjson", &target, "-R", "--include", &filter, "--files-only"])?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let items: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Lỗi phân tích JSON khi tìm kiếm: {}", e))?;
    
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
}

#[tauri::command]
pub async fn get_home_dir() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let desktop = format!("{}/Desktop", home);
    if std::path::Path::new(&desktop).exists() {
        Ok(desktop)
    } else {
        Ok(home)
    }
}

#[tauri::command]
pub async fn open_in_terminal(path: String) -> Result<(), String> {
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
        let terms = ["gnome-terminal", "konsole", "xfce4-terminal", "xterm", "alacritty", "kitty"];
        let mut success = false;
        for term in terms {
            if let Ok(_) = Command::new(term).current_dir(&path).spawn() {
                success = true;
                break;
            }
        }
        if !success {
            return Err("Không tìm thấy terminal hỗ trợ (đã thử gnome-terminal, konsole, xfce4-terminal, xterm).".into());
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
}

#[tauri::command]
pub async fn fs_get_thumbnail(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        use std::io::Cursor;
        use std::path::Path;

        let actual_path = if path.starts_with("Local::") {
            path.strip_prefix("Local::").unwrap().to_string()
        } else {
            path.clone()
        };

        let ext = Path::new(&actual_path).extension().unwrap_or_default().to_string_lossy().to_lowercase();
        
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
            },
            "pdf" => {
                let output = Command::new("pdftoppm")
                    .args(["-jpeg", "-f", "1", "-l", "1", "-singlefile", "-scale-to", "64", &actual_path])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let base64_str = STANDARD.encode(&out.stdout);
                        return Ok(format!("data:image/jpeg;base64,{}", base64_str));
                    }
                }
            },
            _ => {}
        }

        let img = image::open(&actual_path).map_err(|e| format!("Lỗi mở ảnh: {}", e))?;
        let thumb = img.thumbnail(64, 64);
        
        let mut buffer = Cursor::new(Vec::new());
        thumb.write_to(&mut buffer, image::ImageFormat::Jpeg).map_err(|e| format!("Lỗi tạo thumb: {}", e))?;
        
        let base64_str = STANDARD.encode(buffer.into_inner());
        Ok(format!("data:image/jpeg;base64,{}", base64_str))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn fs_temp_dir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}
