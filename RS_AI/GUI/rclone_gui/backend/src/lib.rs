/*
[INTEGRITY NOTES]
Mục đích: Library chính của rcloneGUI Tauri Backend.
Trách nhiệm: Khởi tạo Builder, quản lý plugin, định tuyến các invoke handlers (hiện tại trống).
Các module tương tác: frontend qua Tauri IPC.
*/

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use tauri::Emitter;

pub mod remote;
pub mod mount;
pub mod config;
// Removed RemoteInfo struct because we will use serde_json::Value to pass all fields

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

#[derive(Serialize, Clone, Debug)]
pub struct FileItem {
    pub uuid: String, // Sử dụng Path của rclone làm uuid
    pub name: String,
    pub size: i64,
    pub is_dir: bool,
    pub mod_time: String,
    pub file_type: Option<String>,
}

#[tauri::command]
async fn list_remotes() -> Result<Vec<Value>, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("dump")
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse rclone output: {}", e))?;

    let mut remotes = Vec::new();
    
    // Thêm Local giả lập lên đầu
    let mut local_remote = serde_json::Map::new();
    local_remote.insert("name".to_string(), Value::String("Local".to_string()));
    local_remote.insert("type".to_string(), Value::String("local".to_string()));
    remotes.push(Value::Object(local_remote));

    if let Value::Object(map) = parsed {
        for (name, mut config) in map {
            if let Some(config_obj) = config.as_object_mut() {
                // Ensure type exists, then add name
                if config_obj.contains_key("type") {
                    config_obj.insert("name".to_string(), Value::String(name));
                    remotes.push(Value::Object(config_obj.clone()));
                }
            }
        }
    }

    Ok(remotes)
}

#[tauri::command]
async fn list_files(remote: String, path: String) -> Result<Vec<FileItem>, String> {
    let target = if remote == "Local" {
        if path.is_empty() { "/".to_string() } else { path.clone() }
    } else {
        format!("{}:{}", remote, path)
    };

    let output = Command::new("rclone")
        .arg("lsjson")
        .arg(&target)
        .arg("--max-depth")
        .arg("1")
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone lsjson error for {}: {}", target, err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    
    // Nếu output rỗng thì trả về mảng rỗng thay vì lỗi parse
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed_files: Vec<RcloneFile> = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse rclone output: {}", e))?;

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

    // Sort: Thư mục lên trước, rồi tới file theo alphabet
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
async fn fs_mkdir(remote: String, path: String) -> Result<(), String> {
    let target = if remote == "Local" { path } else { format!("{}:{}", remote, path) };
    let output = Command::new("rclone")
        .args(["mkdir", &target])
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(())
}

#[tauri::command]
async fn fs_delete(remote: String, path: String) -> Result<(), String> {
    let target = if remote == "Local" { path } else { format!("{}:{}", remote, path) };
    // purge will delete directory and all its contents, delete only deletes files.
    // Using purge as a universal delete for both files and dirs.
    let output = Command::new("rclone")
        .args(["purge", &target])
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(())
}

#[tauri::command]
async fn fs_rename(remote: String, old_path: String, new_path: String) -> Result<(), String> {
    let src = if remote == "Local" { old_path } else { format!("{}:{}", remote, old_path) };
    let dst = if remote == "Local" { new_path } else { format!("{}:{}", remote, new_path) };
    let output = Command::new("rclone")
        .args(["moveto", &src, &dst])
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(())
}

#[tauri::command]
async fn fs_copy(
    app_handle: tauri::AppHandle,
    src_remote: String,
    src_path: String,
    dest_remote: String,
    dest_path: String,
    task_id: Option<u32>,
) -> Result<(), String> {
    let src = if src_remote == "Local" { src_path } else { format!("{}:{}", src_remote, src_path) };
    let dst = if dest_remote == "Local" { dest_path } else { format!("{}:{}", dest_remote, dest_path) };
    
    let mut child = Command::new("rclone")
        .args(["copyto", &src, &dst, "--use-json-log", "--stats", "0.5s", "-v"])
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone: {}", e))?;

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line_str) {
                    if let Some(stats) = json.get("stats") {
                        if let Some(id) = task_id {
                            let payload = serde_json::json!({
                                "id": id,
                                "stats": stats
                            });
                            let _ = app_handle.emit("transfer_progress", payload);
                        }
                    }
                }
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for rclone: {}", e))?;
    if !status.success() {
        return Err(format!("Command failed with status: {}", status));
    }
    Ok(())
}

#[tauri::command]
async fn fs_move(
    app_handle: tauri::AppHandle,
    src_remote: String,
    src_path: String,
    dest_remote: String,
    dest_path: String,
    task_id: Option<u32>,
) -> Result<(), String> {
    let src = if src_remote == "Local" { src_path } else { format!("{}:{}", src_remote, src_path) };
    let dst = if dest_remote == "Local" { dest_path } else { format!("{}:{}", dest_remote, dest_path) };
    
    let mut child = Command::new("rclone")
        .args(["moveto", &src, &dst, "--use-json-log", "--stats", "0.5s", "-v"])
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rclone: {}", e))?;

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line_str) {
                    if let Some(stats) = json.get("stats") {
                        if let Some(id) = task_id {
                            let payload = serde_json::json!({
                                "id": id,
                                "stats": stats
                            });
                            let _ = app_handle.emit("transfer_progress", payload);
                        }
                    }
                }
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for rclone: {}", e))?;
    if !status.success() {
        return Err(format!("Command failed with status: {}", status));
    }
    Ok(())
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

#[tauri::command]
async fn fs_stat_advanced(remote: String, path: String) -> Result<StatInfo, String> {
    let target = if remote == "Local" { path } else { format!("{}:{}", remote, path) };
    let output = Command::new("rclone")
        .args(["size", &target, "--json"])
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    
    let parsed: RcloneSizeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
    Ok(StatInfo {
        size: parsed.bytes,
        file_count: parsed.count,
        dir_count: 0,
        permissions: 0,
        uid: 0,
        gid: 0,
    })
}

#[derive(serde::Serialize)]
pub struct SearchResultItem {
    item: FileItem,
    path: String,
}

#[tauri::command]
async fn fs_search(remote: String, path: String, query: String) -> Result<Vec<SearchResultItem>, String> {
    let target = if remote == "Local" { path.clone() } else { format!("{}:{}", remote, path.clone()) };
    let filter = format!("*{}*", query);
    let output = Command::new("rclone")
        .args(["lsjson", &target, "-R", "--include", &filter, "--files-only"])
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let items: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let mut files = Vec::new();
    for item in items {
        let name = item["Name"].as_str().unwrap_or("").to_string();
        let rel_path = item["Path"].as_str().unwrap_or("").to_string();
        
        let file_path = if path.ends_with('/') {
            format!("{}{}", path, rel_path)
        } else {
            format!("{}/{}", path, rel_path)
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
        
        // Trả về fullpath dạng Remote::/path
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
async fn get_home_dir() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    // Mặc định trả về Desktop
    let desktop = format!("{}/Desktop", home);
    // Kiểm tra xem thư mục Desktop có tồn tại không, nếu không thì trả về home
    if std::path::Path::new(&desktop).exists() {
        Ok(desktop)
    } else {
        Ok(home)
    }
}

#[tauri::command]
async fn open_in_terminal(path: String) -> Result<(), String> {
    use std::process::Command;
    
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/c")
            .arg("start")
            .arg("cmd")
            .current_dir(&path)
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let terms = ["gnome-terminal", "konsole", "xfce4-terminal", "xterm", "alacritty", "kitty"];
        let mut success = false;
        for term in terms {
            // For linux, some terminals might need --working-directory, but current_dir usually works.
            if let Ok(_) = Command::new(term).current_dir(&path).spawn() {
                success = true;
                break;
            }
        }
        if !success {
            return Err("No supported terminal found (tried gnome-terminal, konsole, xfce4-terminal, xterm).".into());
        }
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn fs_get_thumbnail(path: String) -> Result<String, String> {
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
                let output = std::process::Command::new("ffmpegthumbnailer")
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
                let output = std::process::Command::new("pdftoppm")
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
async fn fs_sudo_exec(action: String, args: Vec<String>) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    {
        let mut cmd_args = Vec::new();
        match action.as_str() {
            "rm" => {
                cmd_args.push("rm".to_string());
                cmd_args.push("-rf".to_string());
                for arg in args { cmd_args.push(arg); }
            },
            "mkdir" => {
                cmd_args.push("mkdir".to_string());
                cmd_args.push("-p".to_string());
                for arg in args { cmd_args.push(arg); }
            },
            "mv" => {
                cmd_args.push("mv".to_string());
                for arg in args { cmd_args.push(arg); }
            },
            "cp" => {
                cmd_args.push("cp".to_string());
                cmd_args.push("-r".to_string());
                for arg in args { cmd_args.push(arg); }
            },
            _ => return Err("Unsupported action".into()),
        }
        let output = Command::new("pkexec")
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("Failed to execute pkexec: {}", e))?;
            
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).into_owned();
            if err.is_empty() {
                return Err("Thao tác bị huỷ hoặc lỗi phân quyền.".into());
            }
            return Err(err);
        }
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        return Err("Tính năng Sudo hiện tại chỉ hỗ trợ trên Linux (qua pkexec).".into());
    }
}


#[tauri::command]
async fn rclone_about(remote: String) -> Result<serde_json::Value, String> {
    let output = Command::new("rclone")
        .arg("about")
        .arg(&remote)
        .arg("--json")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));
        Ok(parsed)
    } else {
        // Many remotes don't support about, just return empty object instead of error
        Ok(serde_json::json!({}))
    }
}

#[tauri::command]
async fn rclone_size(remote: String) -> Result<serde_json::Value, String> {
    let output = Command::new("rclone")
        .arg("size")
        .arg(&remote)
        .arg("--json")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));
        Ok(parsed)
    } else {
        Ok(serde_json::json!({}))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_remotes,
            list_files,
            fs_mkdir,
            fs_delete,
            fs_rename,
            fs_copy,
            fs_move,
            fs_stat_advanced,
            fs_search,
            get_home_dir,
            open_in_terminal,
            fs_get_thumbnail,
            fs_sudo_exec,
            remote::get_providers,
            remote::create_remote,
            remote::update_remote,
            remote::delete_remote,
            remote::get_backend_features,
            mount::check_fuse_installed,
            mount::create_mount_service,
            mount::delete_mount_service,
            mount::manage_mount_service,
            mount::list_mount_services,
            mount::get_mount_service_config,
            config::get_config_content,
            config::set_config_content,
            rclone_about,
            rclone_size,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rclone_about_empty() {
        // Test with invalid remote, should fallback to {} instead of failing
        let result = rclone_about("InvalidRemoteTest123:".to_string()).await;
        assert!(result.is_ok());
        let val = result.unwrap();
        // Since invalid remote will fail `rclone about`, it should return {}
        assert_eq!(val, serde_json::json!({}));
    }

    #[tokio::test]
    async fn test_rclone_size_empty() {
        // Test with invalid remote, should fallback to {} instead of failing
        let result = rclone_size("InvalidRemoteTest123:".to_string()).await;
        assert!(result.is_ok());
        let val = result.unwrap();
        assert_eq!(val, serde_json::json!({}));
    }
}
