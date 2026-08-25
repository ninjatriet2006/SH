//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến thao tác hệ thống tệp (File System - FS).
//! Trách nhiệm: Xử lý cloud fs (liệt kê, tạo thư mục, xóa, copy, move...), local fs và các thao tác đặc thù (tìm kiếm, chmod, thùng rác).
//! Tương tác: Giao tiếp qua `filen_gui::cloud_fs` và `filen_gui::local_fs`. Giao diện `DualPaneExplorer` sử dụng các alias như `fs_rename_terminal`.

use crate::state::AppState;
use filen_gui::models::{FileItem, TrashItemLocal};

/// Liệt kê danh sách file/thư mục trên Cloud (phương thức thông thường).
#[tauri::command]
pub async fn fs_list_remote_terminal(
    account: Option<String>,
    path: String,
) -> Result<Vec<FileItem>, String> {
    filen_gui::cloud_fs::list_remote_terminal(&account, &path).await
}

/// Liệt kê danh sách file/thư mục trên Cloud theo dạng luồng (stream),
/// giúp UI cập nhật dần khi có nhiều file thay vì đợi toàn bộ.
#[tauri::command]
pub async fn fs_list_remote_stream_terminal(
    account: Option<String>,
    path: String,
    on_chunk: tauri::ipc::Channel<Vec<FileItem>>,
) -> Result<(), String> {
    filen_gui::cloud_fs::list_remote_stream_terminal(&account, &path, move |chunk| {
        let _ = on_chunk.send(chunk);
    }).await
}

/// Lấy ảnh thu nhỏ (thumbnail) của file, sinh ra mã Base64 để hiển thị lên UI.
#[tauri::command]
pub async fn fs_get_thumbnail(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        filen_gui::local_fs::get_thumbnail(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Liệt kê danh sách file/thư mục tại máy tính cục bộ (Local).
/// Hàm này đồng thời cập nhật trình theo dõi tự động (Watcher) để UI phản ứng khi có file mới/bị xóa.
#[tauri::command]
pub async fn fs_list_local(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FileItem>, String> {
    // Cập nhật trình theo dõi (local watcher)
    {
        use notify::Watcher;
        // Lấy khóa truy cập vào biến trạng thái lưu đường dẫn và trình theo dõi
        let mut watched_path = state.watched_path.lock().unwrap();
        let mut watcher_opt = state.local_watcher.lock().unwrap();
        
        if let Some(watcher) = watcher_opt.as_mut() {
            // Hủy theo dõi đường dẫn cũ nếu đường dẫn thay đổi
            if let Some(old_path) = watched_path.as_ref() {
                if old_path != &path {
                    let _ = watcher.unwatch(std::path::Path::new(old_path));
                }
            }
            // Đăng ký theo dõi đường dẫn mới
            if watched_path.as_ref() != Some(&path) {
                // NonRecursive vì chúng ta chỉ quan tâm biến động ở thư mục gốc đang hiển thị
                let _ = watcher.watch(std::path::Path::new(&path), notify::RecursiveMode::NonRecursive);
                *watched_path = Some(path.clone());
            }
        }
    }

    filen_gui::local_fs::list_local(&path)
}

/// Tạo thư mục mới trên Cloud.
#[tauri::command]
pub async fn fs_mkdir_terminal(account: Option<String>, path: String) -> Result<(), String> {
    filen_gui::cloud_fs::mkdir_terminal(&account, &path).await
}

/// Xóa file/thư mục trên Cloud (có hỗ trợ tùy chọn xóa vĩnh viễn không qua thùng rác).
#[tauri::command]
pub async fn fs_rm_terminal(
    account: Option<String>,
    path: String,
    no_trash: bool,
) -> Result<(), String> {
    filen_gui::cloud_fs::rm_terminal(&account, &path, no_trash).await
}

/// Đổi tên / Di chuyển thư mục, file trên Cloud.
#[tauri::command]
pub async fn fs_mv_terminal(
    account: Option<String>,
    from: String,
    to: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::mv_terminal(&account, &from, &to).await
}

/// Sao chép thư mục, file trên Cloud.
#[tauri::command]
pub async fn fs_cp_terminal(
    account: Option<String>,
    from: String,
    to: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::cp_terminal(&account, &from, &to).await
}

/// Sao chép thư mục, file trên Local.
#[tauri::command]
pub async fn fs_cp_local(from: String, to: String, overwrite: bool) -> Result<(), String> {
    filen_gui::local_fs::copy_local(&from, &to, overwrite)
}

/// Đổi tên / Di chuyển thư mục, file trên Local.
#[tauri::command]
pub async fn fs_mv_local(from: String, to: String) -> Result<(), String> {
    filen_gui::local_fs::move_local(&from, &to)
}

/// Xóa thư mục, file trên Local.
#[tauri::command]
pub async fn fs_rm_local(path: String) -> Result<(), String> {
    filen_gui::local_fs::delete_local(&path)
}

/// Tạo thư mục mới trên Local.
#[tauri::command]
pub async fn fs_mkdir_local(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

/// Đổi tên thư mục, file trên Local (giữ nguyên gốc thư mục).
#[tauri::command]
pub async fn fs_rename_local(path: String, new_name: String) -> Result<(), String> {
    let parent = std::path::Path::new(&path).parent().unwrap_or(std::path::Path::new(""));
    let dest = parent.join(new_name);
    std::fs::rename(&path, &dest).map_err(|e| e.to_string())
}

/// Lệnh hỗ trợ chép nhiều file, thư mục cùng một lúc dưới Local.
#[tauri::command]
pub async fn fs_cp_batch(srcs: Vec<String>, dst_dir: String, overwrite: bool) -> Result<(), String> {
    filen_gui::local_fs::copy_local_batch(&srcs, &dst_dir, overwrite)
}

// ---------------------------------------------------------------------------
// Thùng rác (Trash)
// ---------------------------------------------------------------------------

/// Lấy danh sách rác trong hệ điều hành Local.
#[tauri::command]
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    filen_gui::local_fs::list_trash_local()
}

/// Khôi phục file trong thùng rác hệ điều hành.
#[tauri::command]
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    filen_gui::local_fs::trash_restore_local(&item_id)
}

/// Dọn sạch thùng rác cục bộ.
#[tauri::command]
pub async fn fs_trash_empty_local() -> Result<(), String> {
    filen_gui::local_fs::trash_empty_local()
}

/// Lấy danh sách rác trên Cloud.
#[tauri::command]
pub async fn fs_trash_list_remote_terminal(account: Option<String>) -> Result<Vec<FileItem>, String> {
    filen_gui::cloud_fs::list_trash_terminal(&account).await
}

/// Khôi phục file trong thùng rác Cloud dựa vào index (ID).
#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(account: Option<String>, idx: usize) -> Result<(), String> {
    filen_gui::cloud_fs::trash_restore_terminal(&account, idx).await
}

/// Xóa vĩnh viễn 1 file cụ thể trong thùng rác Cloud.
#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(account: Option<String>, idx: usize) -> Result<(), String> {
    filen_gui::cloud_fs::trash_delete_terminal(&account, idx).await
}

/// Dọn sạch thùng rác Cloud.
#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(account: Option<String>) -> Result<(), String> {
    filen_gui::cloud_fs::trash_empty_terminal(&account).await
}

// ---------------------------------------------------------------------------
// Đồng bộ nhanh (Upload / Download terminal không qua hàng đợi)
// ---------------------------------------------------------------------------

/// Tải lên trực tiếp không qua hàng đợi.
#[tauri::command]
pub async fn fs_upload_terminal(
    account: Option<String>,
    local: String,
    remote: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::upload_terminal(&account, &local, &remote).await
}

/// Tải xuống trực tiếp không qua hàng đợi.
#[tauri::command]
pub async fn fs_download_terminal(
    account: Option<String>,
    remote: String,
    local: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::download_terminal(&account, &remote, &local).await
}

/// Đọc trực tiếp nội dung văn bản của file Cloud (ví dụ: mở text editor).
#[tauri::command]
pub async fn fs_cat_terminal(account: Option<String>, path: String) -> Result<String, String> {
    filen_gui::cloud_fs::cat_terminal(&account, &path).await
}

/// Tạo một public link (liên kết chia sẻ) cho file trên Cloud.
#[tauri::command]
pub async fn fs_link_create_terminal(
    account: Option<String>,
    path: String,
) -> Result<String, String> {
    filen_gui::cloud_fs::create_link_terminal(&account, &path).await
}

/// Liệt kê toàn bộ public links đã tạo.
#[tauri::command]
pub async fn fs_links_list_terminal(
    account: Option<String>,
) -> Result<Vec<(String, String)>, String> {
    filen_gui::cloud_fs::list_links_terminal(&account).await
}

/// Ghi nội dung văn bản vào file Cloud.
#[tauri::command]
pub async fn fs_write_terminal(
    account: Option<String>,
    path: String,
    content: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::write_file_terminal(&account, &path, &content).await
}

/// Ghi nội dung văn bản vào file dưới Local.
#[tauri::command]
pub async fn fs_write_local(
    path: String,
    content: String,
) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Các Alias cho DualPaneExplorer (UI gọi tên đồng nhất)
// ---------------------------------------------------------------------------

/// Alias đổi tên trên Cloud: Thực chất là gọi Move với đường dẫn cùng cha.
#[tauri::command]
pub async fn fs_rename_terminal(
    account: Option<String>,
    path: String,
    new_name: String,
) -> Result<(), String> {
    let parent = std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());
    let new_path = if parent == "/" || parent.ends_with('/') {
        format!("{parent}{new_name}")
    } else {
        format!("{parent}/{new_name}")
    };
    filen_gui::cloud_fs::mv_terminal(&account, &path, &new_path).await
}

/// Alias xóa trên Cloud (vào thùng rác thay vì xóa vĩnh viễn).
#[tauri::command]
pub async fn fs_delete_terminal(account: Option<String>, path: String) -> Result<(), String> {
    filen_gui::cloud_fs::rm_terminal(&account, &path, false).await
}

/// Alias sao chép Cloud.
#[tauri::command]
pub async fn fs_copy_terminal(
    account: Option<String>,
    src: String,
    dest: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::cp_terminal(&account, &src, &dest).await
}

/// Alias di chuyển Cloud.
#[tauri::command]
pub async fn fs_move_terminal(
    account: Option<String>,
    src: String,
    dest: String,
) -> Result<(), String> {
    filen_gui::cloud_fs::mv_terminal(&account, &src, &dest).await
}

/// Mở file trong ứng dụng mặc định của hệ điều hành.
#[tauri::command]
pub fn fs_open(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Hệ điều hành chưa được hỗ trợ để mở file ngoài hệ thống".to_string())
    }
}

// ---------------------------------------------------------------------------
// Các chức năng phân tích hệ thống tệp và tìm kiếm (Advanced)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct StatInfo {
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub permissions: u32,
    pub uid: u32,
    pub gid: u32,
}

/// Tính toán thông tin dung lượng mở rộng: đếm tổng dung lượng, số lượng file, thư mục bên trong (đệ quy).
#[tauri::command]
pub fn fs_stat_advanced(path: String) -> Result<StatInfo, String> {
    use std::path::Path;
    let p = Path::new(&path);
    let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    
    let mut size = meta.len();
    let mut file_count = 0;
    let mut dir_count = 0;

    // Nếu là thư mục, đi sâu vào đếm (recursive)
    if meta.is_dir() {
        let walker = walkdir::WalkDir::new(&p).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            if entry.path() != p {
                if entry.file_type().is_file() {
                    file_count += 1;
                    size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                } else if entry.file_type().is_dir() {
                    dir_count += 1;
                }
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::fs::MetadataExt;
        Ok(StatInfo {
            size,
            file_count,
            dir_count,
            permissions: meta.permissions().mode(),
            uid: meta.uid(),
            gid: meta.gid(),
        })
    }
    #[cfg(not(unix))]
    {
        Ok(StatInfo {
            size,
            file_count,
            dir_count,
            permissions: 0,
            uid: 0,
            gid: 0,
        })
    }
}

/// Thay đổi quyền truy cập (chmod).
#[tauri::command]
pub fn fs_chmod(path: String, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Err("Lệnh chmod không được hỗ trợ trên hệ điều hành này".to_string())
    }
}

/// Thay đổi chủ sở hữu (chown).
#[tauri::command]
pub fn fs_chown(path: String, uid: u32, gid: u32) -> Result<(), String> {
    filen_gui::local_fs::chown_local(&path, uid, gid)
}

/// Lấy thông tin dung lượng còn trống của một đường dẫn phân vùng (dành cho Local).
#[tauri::command]
pub fn fs_get_free_space(path: String) -> Result<u64, String> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let c_path = std::ffi::CString::new(path.as_bytes()).map_err(|e| e.to_string())?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
            // Khối lượng khả dụng * kích thước khối
            Ok(stat.f_bavail as u64 * stat.f_frsize as u64)
        } else {
            Err("Không thể lấy dung lượng phân vùng trống".to_string())
        }
    }
    #[cfg(not(unix))]
    {
        Ok(0)
    }
}

/// Dữ liệu đầu vào cấu hình cho tìm kiếm tệp tin.
#[derive(serde::Deserialize, Debug)]
pub struct SearchOptions {
    /// Sử dụng tìm kiếm tương đối (Fuzzy search) hay chính xác.
    pub fuzzy: bool,
    /// Từ khóa nội dung nếu muốn tìm bên trong văn bản (Content search).
    pub content_query: Option<String>,
    /// Dung lượng tệp nhỏ nhất.
    pub min_size: Option<u64>,
    /// Dung lượng tệp lớn nhất.
    pub max_size: Option<u64>,
}

/// Kết quả trả về của một tác vụ tìm kiếm tệp.
#[derive(serde::Serialize)]
pub struct SearchResult {
    /// Thông tin tiêu chuẩn FileItem (size, thời gian).
    pub item: filen_gui::models::FileItem,
    /// Đường dẫn file tìm được.
    pub path: String,
    /// Điểm chấm tìm kiếm (càng cao càng chính xác).
    pub score: i64,
}

/// Tìm kiếm File/Thư mục cục bộ (hỗ trợ lọc file lớn/nhỏ, tên và cả nội dung văn bản).
#[tauri::command]
pub async fn fs_search_local(path: String, query: String, options: Option<SearchOptions>) -> Result<Vec<SearchResult>, String> {
    use std::path::Path;
    use fuzzy_matcher::FuzzyMatcher;
    use fuzzy_matcher::skim::SkimMatcherV2;
    
    let root = Path::new(&path);
    let mut results = Vec::new();
    let lower_query = query.to_lowercase();
    let opts = options.unwrap_or(SearchOptions {
        fuzzy: false,
        content_query: None,
        min_size: None,
        max_size: None,
    });
    
    let matcher = SkimMatcherV2::default();
    let walker = walkdir::WalkDir::new(root).into_iter();
    
    for entry in walker.filter_map(|e| e.ok()) {
        if entry.path() == root {
            continue; // Bỏ qua thư mục gốc
        }
        
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        
        let size = meta.len();
        // Lọc giới hạn dung lượng (min_size, max_size)
        if let Some(min_s) = opts.min_size {
            if size < min_s { continue; }
        }
        if let Some(max_s) = opts.max_size {
            if size > max_s { continue; }
        }
        
        let file_name = entry.file_name().to_string_lossy().to_string();
        let mut score = 0;
        
        // Lọc theo từ khóa ở tên file
        if !query.is_empty() {
            if opts.fuzzy {
                if let Some(s) = matcher.fuzzy_match(&file_name, &query) {
                    score = s;
                } else {
                    continue;
                }
            } else {
                if file_name.to_lowercase().contains(&lower_query) {
                    score = 100;
                } else {
                    continue;
                }
            }
        }
        
        // Lọc theo từ khóa bên trong nội dung văn bản
        if let Some(ref cq) = opts.content_query {
            if cq.trim().len() > 0 {
                if meta.is_dir() { continue; } // Không đọc thư mục
                if size > 10 * 1024 * 1024 { continue; } // Bỏ qua file > 10MB để tránh treo ứng dụng
                
                // Trích xuất text từ file doc/pdf hoặc txt
                if let Some(content) = filen_gui::sys::doc_search::extract_text(entry.path()) {
                    if !content.to_lowercase().contains(&cq.to_lowercase()) {
                        continue;
                    }
                } else {
                    continue; // Không phải định dạng text hoặc lỗi đọc file
                }
            }
        }

        let mod_time = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
            
        let mod_time_str = format!("{}", mod_time);
        
        results.push(SearchResult {
            item: filen_gui::models::FileItem {
                name: file_name,
                is_dir: meta.is_dir(),
                size: size,
                mod_time: mod_time_str,
                ..Default::default()
            },
            path: entry.path().to_string_lossy().to_string(),
            score,
        });

        // Tối đa 100 kết quả để tránh nghẽn giao diện UI
        if results.len() >= 100 {
            break;
        }
    }
    
    Ok(results)
}

#[tauri::command]
pub async fn fs_sudo_exec(action: String, args: Vec<String>) -> Result<(), String> {
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
