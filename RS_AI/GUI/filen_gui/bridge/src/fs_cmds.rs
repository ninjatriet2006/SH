//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến thao tác hệ thống tệp (File System - FS).
//! Trách nhiệm: Xử lý cloud fs (liệt kê, tạo thư mục, xóa, copy, move...), local fs và các thao tác đặc thù (tìm kiếm, chmod, thùng rác).
//! Tương tác: Giao tiếp qua `filen_gui::cloud_fs` và `filen_gui::local_fs`. Giao diện `DualPaneExplorer` sử dụng các alias như `fs_rename_terminal`.

use crate::state::AppState;
use crate::{
    auth_cmds::Empty,
    contract::{backend, success, validate, IpcResult, Req},
    security::{require_confirmation, validate_sudo_argv},
};
use filen_gui::models::{FileItem, TrashItemLocal};
use serde::Deserialize;
use std::path::Path;

macro_rules! payload {
    ($name:ident { $($(#[$meta:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Deserialize)] pub struct $name { $($(#[$meta])* pub $field: $ty),* }
    };
}
payload!(AccountPath { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String });
payload!(StreamPath { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String });
payload!(PathReq { path: String });
payload!(RemoteRm { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String, recursive: bool, confirmed: bool });
payload!(RemoteFromTo { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, from: String, to: String });
payload!(LocalFromTo { src: String, dest: String, #[serde(deserialize_with = "crate::contract::present_nullable")] overwrite: Option<bool> });
payload!(RenameReq {
    path: String,
    new_name: String
});
payload!(BatchReq { srcs: Vec<String>, dst_dir: String, overwrite: bool });
payload!(UploadReq { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, local: String, remote: String });
payload!(DownloadReq { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, remote: String, local: String });
payload!(WriteReq { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String, content: String });
payload!(WriteLocalReq {
    path: String,
    content: String
});
payload!(RenameRemote { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String, new_name: String });
payload!(DeleteRemote { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, path: String, confirmed: bool });
payload!(CopyRemote { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, src: String, dest: String });
payload!(ItemReq { item_id: String });
payload!(IndexReq { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String>, idx: usize, #[serde(deserialize_with = "crate::contract::present_nullable")] confirmed: Option<bool> });
payload!(AccountReq { #[serde(deserialize_with = "crate::contract::present_nullable")] account: Option<String> });
payload!(ModeReq {
    path: String,
    mode: u32,
    confirmed: bool
});
payload!(OwnerReq {
    path: String,
    uid: u32,
    gid: u32,
    confirmed: bool
});
payload!(SearchReq { path: String, query: String, #[serde(deserialize_with = "crate::contract::present_nullable")] options: Option<SearchOptions> });
payload!(SudoReq { action: String, args: Vec<String>, confirmed: bool });
payload!(PickerReq {});

#[cfg(target_os = "windows")]
fn open_path(path: &str) -> Result<(), crate::contract::IpcError> {
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", path])
        .spawn()
        .map(|_| ())
        .map_err(|error| backend(error.to_string()))
}

fn contained_existing(state: &AppState, window: &str, path: &str) -> Result<String, crate::contract::IpcError> {
    Ok(state
        .picker_roots
        .existing(window, Path::new(path))?
        .to_string_lossy()
        .into_owned())
}
fn contained_create(state: &AppState, window: &str, path: &str) -> Result<String, crate::contract::IpcError> {
    Ok(state
        .picker_roots
        .create(window, Path::new(path))?
        .to_string_lossy()
        .into_owned())
}

/// Liệt kê danh sách file/thư mục trên Cloud (phương thức thông thường).
#[tauri::command]
pub async fn fs_list_remote_terminal(request: Req<AccountPath>) -> IpcResult<Vec<FileItem>> {
    let (id, p) = validate(request)?;
    let data = filen_gui::cloud_fs::list_remote_terminal(&p.account, &p.path)
        .await
        .map_err(backend)?;
    Ok(success(id, data))
}

/// Liệt kê danh sách file/thư mục trên Cloud theo dạng luồng (stream),
/// giúp UI cập nhật dần khi có nhiều file thay vì đợi toàn bộ.
#[tauri::command]
pub async fn fs_list_remote_stream_terminal(
    request: Req<StreamPath>,
    on_chunk: tauri::ipc::Channel<Vec<FileItem>>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::list_remote_stream_terminal(&p.account, &p.path, move |chunk| {
        let _ = on_chunk.send(chunk);
    })
    .await
    .map_err(backend)?;
    Ok(success(id, ()))
}

/// Lấy ảnh thu nhỏ (thumbnail) của file, sinh ra mã Base64 để hiển thị lên UI.
#[tauri::command]
pub async fn fs_get_thumbnail(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<String> {
    let (id, p) = validate(request)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    let data = tauri::async_runtime::spawn_blocking(move || filen_gui::local_fs::get_thumbnail(&path))
        .await
        .map_err(|e| backend(e.to_string()))?
        .map_err(backend)?;
    Ok(success(id, data))
}

/// Liệt kê danh sách file/thư mục tại máy tính cục bộ (Local).
/// Hàm này đồng thời cập nhật trình theo dõi tự động (Watcher) để UI phản ứng khi có file mới/bị xóa.
#[tauri::command]
pub async fn fs_list_local(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<Vec<FileItem>> {
    let (id, p) = validate(request)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
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

    let data = filen_gui::local_fs::list_local(&path).map_err(backend)?;
    Ok(success(id, data))
}

/// Tạo thư mục mới trên Cloud.
#[tauri::command]
pub async fn fs_mkdir_terminal(request: Req<AccountPath>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::mkdir_terminal(&p.account, &p.path)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Xóa file/thư mục trên Cloud (có hỗ trợ tùy chọn xóa vĩnh viễn không qua thùng rác).
#[tauri::command]
pub async fn fs_rm_terminal(request: Req<RemoteRm>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    filen_gui::cloud_fs::rm_terminal(&p.account, &p.path, p.recursive)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Đổi tên / Di chuyển thư mục, file trên Cloud.
#[tauri::command]
pub async fn fs_mv_terminal(request: Req<RemoteFromTo>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::mv_terminal(&p.account, &p.from, &p.to)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Sao chép thư mục, file trên Cloud.
#[tauri::command]
pub async fn fs_cp_terminal(request: Req<RemoteFromTo>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::cp_terminal(&p.account, &p.from, &p.to)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Sao chép thư mục, file trên Local.
#[tauri::command]
pub async fn fs_cp_local(
    request: Req<LocalFromTo>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let src = contained_existing(&state, window.label(), &p.src)?;
    let dest = contained_create(&state, window.label(), &p.dest)?;
    filen_gui::local_fs::copy_local(&src, &dest, p.overwrite.unwrap_or(false)).map_err(backend)?;
    Ok(success(id, ()))
}

/// Đổi tên / Di chuyển thư mục, file trên Local.
#[tauri::command]
pub async fn fs_mv_local(
    request: Req<LocalFromTo>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let src = contained_existing(&state, window.label(), &p.src)?;
    let dest = contained_create(&state, window.label(), &p.dest)?;
    let _ = p.overwrite;
    filen_gui::local_fs::move_local(&src, &dest).map_err(backend)?;
    Ok(success(id, ()))
}

/// Xóa thư mục, file trên Local.
#[tauri::command]
pub async fn fs_rm_local(
    request: Req<DeleteRemote>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    filen_gui::local_fs::delete_local(&path).map_err(backend)?;
    Ok(success(id, ()))
}

/// Tạo thư mục mới trên Local.
#[tauri::command]
pub async fn fs_mkdir_local(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let path = contained_create(&state, window.label(), &p.path)?;
    std::fs::create_dir_all(path).map_err(|e| backend(e.to_string()))?;
    Ok(success(id, ()))
}

/// Đổi tên thư mục, file trên Local (giữ nguyên gốc thư mục).
#[tauri::command]
pub async fn fs_rename_local(
    request: Req<RenameReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    let dest_path = Path::new(&path).with_file_name(p.new_name);
    let dest = contained_create(&state, window.label(), &dest_path.to_string_lossy())?;
    std::fs::rename(path, dest).map_err(|e| backend(e.to_string()))?;
    Ok(success(id, ()))
}

/// Lệnh hỗ trợ chép nhiều file, thư mục cùng một lúc dưới Local.
#[tauri::command]
pub async fn fs_cp_batch(
    request: Req<BatchReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let srcs = p
        .srcs
        .iter()
        .map(|v| contained_existing(&state, window.label(), v))
        .collect::<Result<Vec<_>, _>>()?;
    let dst = contained_existing(&state, window.label(), &p.dst_dir)?;
    filen_gui::local_fs::copy_local_batch(&srcs, &dst, p.overwrite).map_err(backend)?;
    Ok(success(id, ()))
}

// ---------------------------------------------------------------------------
// Thùng rác (Trash)
// ---------------------------------------------------------------------------

/// Lấy danh sách rác trong hệ điều hành Local.
#[tauri::command]
pub async fn fs_trash_list_local(request: Req<Empty>) -> IpcResult<Vec<TrashItemLocal>> {
    let (id, _) = validate(request)?;
    Ok(success(id, filen_gui::local_fs::list_trash_local().map_err(backend)?))
}

/// Khôi phục file trong thùng rác hệ điều hành.
#[tauri::command]
pub async fn fs_trash_restore_local(request: Req<ItemReq>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::local_fs::trash_restore_local(&p.item_id).map_err(backend)?;
    Ok(success(id, ()))
}

/// Dọn sạch thùng rác cục bộ.
#[tauri::command]
pub async fn fs_trash_empty_local(request: Req<RemoteRm>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    filen_gui::local_fs::trash_empty_local().map_err(backend)?;
    Ok(success(id, ()))
}

/// Lấy danh sách rác trên Cloud.
#[tauri::command]
pub async fn fs_trash_list_remote_terminal(request: Req<AccountReq>) -> IpcResult<Vec<FileItem>> {
    let (id, p) = validate(request)?;
    Ok(success(
        id,
        filen_gui::cloud_fs::list_trash_terminal(&p.account)
            .await
            .map_err(backend)?,
    ))
}

/// Khôi phục file trong thùng rác Cloud dựa vào index (ID).
#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(request: Req<IndexReq>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::trash_restore_terminal(&p.account, p.idx)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Xóa vĩnh viễn 1 file cụ thể trong thùng rác Cloud.
#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(request: Req<IndexReq>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed.unwrap_or(false))?;
    filen_gui::cloud_fs::trash_delete_terminal(&p.account, p.idx)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Dọn sạch thùng rác Cloud.
#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(request: Req<RemoteRm>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    filen_gui::cloud_fs::trash_empty_terminal(&p.account)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

// ---------------------------------------------------------------------------
// Đồng bộ nhanh (Upload / Download terminal không qua hàng đợi)
// ---------------------------------------------------------------------------

/// Tải lên trực tiếp không qua hàng đợi.
#[tauri::command]
pub async fn fs_upload_terminal(
    request: Req<UploadReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let local = contained_existing(&state, window.label(), &p.local)?;
    filen_gui::cloud_fs::upload_terminal(&p.account, &local, &p.remote)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Tải xuống trực tiếp không qua hàng đợi.
#[tauri::command]
pub async fn fs_download_terminal(
    request: Req<DownloadReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let local = contained_create(&state, window.label(), &p.local)?;
    filen_gui::cloud_fs::download_terminal(&p.account, &p.remote, &local)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Đọc trực tiếp nội dung văn bản của file Cloud (ví dụ: mở text editor).
#[tauri::command]
pub async fn fs_cat_terminal(request: Req<AccountPath>) -> IpcResult<String> {
    let (id, p) = validate(request)?;
    Ok(success(
        id,
        filen_gui::cloud_fs::cat_terminal(&p.account, &p.path)
            .await
            .map_err(backend)?,
    ))
}

/// Tạo một public link (liên kết chia sẻ) cho file trên Cloud.
#[tauri::command]
pub async fn fs_link_create_terminal(request: Req<AccountPath>) -> IpcResult<String> {
    let (id, p) = validate(request)?;
    Ok(success(
        id,
        filen_gui::cloud_fs::create_link_terminal(&p.account, &p.path)
            .await
            .map_err(backend)?,
    ))
}

/// Liệt kê toàn bộ public links đã tạo.
#[tauri::command]
pub async fn fs_links_list_terminal(request: Req<AccountReq>) -> IpcResult<Vec<(String, String)>> {
    let (id, p) = validate(request)?;
    Ok(success(
        id,
        filen_gui::cloud_fs::list_links_terminal(&p.account)
            .await
            .map_err(backend)?,
    ))
}

/// Ghi nội dung văn bản vào file Cloud.
#[tauri::command]
pub async fn fs_write_terminal(request: Req<WriteReq>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::write_file_terminal(&p.account, &p.path, &p.content)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Ghi nội dung văn bản vào file dưới Local.
#[tauri::command]
pub async fn fs_write_local(
    request: Req<WriteLocalReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let path = contained_create(&state, window.label(), &p.path)?;
    std::fs::write(path, p.content).map_err(|e| backend(e.to_string()))?;
    Ok(success(id, ()))
}

// ---------------------------------------------------------------------------
// Các Alias cho DualPaneExplorer (UI gọi tên đồng nhất)
// ---------------------------------------------------------------------------

/// Alias đổi tên trên Cloud: Thực chất là gọi Move với đường dẫn cùng cha.
#[tauri::command]
pub async fn fs_rename_terminal(request: Req<RenameRemote>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let parent = std::path::Path::new(&p.path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());
    let new_path = if parent == "/" || parent.ends_with('/') {
        format!("{parent}{}", p.new_name)
    } else {
        format!("{parent}/{}", p.new_name)
    };
    filen_gui::cloud_fs::mv_terminal(&p.account, &p.path, &new_path)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Alias xóa trên Cloud (vào thùng rác thay vì xóa vĩnh viễn).
#[tauri::command]
pub async fn fs_delete_terminal(request: Req<DeleteRemote>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    filen_gui::cloud_fs::rm_terminal(&p.account, &p.path, false)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Alias sao chép Cloud.
#[tauri::command]
pub async fn fs_copy_terminal(request: Req<CopyRemote>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::cp_terminal(&p.account, &p.src, &p.dest)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Alias di chuyển Cloud.
#[tauri::command]
pub async fn fs_move_terminal(request: Req<CopyRemote>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    filen_gui::cloud_fs::mv_terminal(&p.account, &p.src, &p.dest)
        .await
        .map_err(backend)?;
    Ok(success(id, ()))
}

/// Mở file trong ứng dụng mặc định của hệ điều hành.
#[tauri::command]
pub fn fs_open(request: Req<PathReq>, window: tauri::Window, state: tauri::State<'_, AppState>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    #[cfg(target_os = "windows")]
    {
        open_path(&path)?;
        Ok(success(id, ()))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| backend(e.to_string()))?;
        return Ok(success(id, ()));
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| backend(e.to_string()))?;
        Ok(success(id, ()))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(crate::contract::unavailable(
            "Hệ điều hành chưa được hỗ trợ để mở file ngoài hệ thống",
        ))
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
pub fn fs_stat_advanced(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<StatInfo> {
    let (id, request) = validate(request)?;
    let path = contained_existing(&state, window.label(), &request.path)?;
    use std::path::Path;
    let p = Path::new(&path);
    let meta = std::fs::metadata(p).map_err(|e| backend(e.to_string()))?;

    let mut size = meta.len();
    let mut file_count = 0;
    let mut dir_count = 0;

    // Nếu là thư mục, đi sâu vào đếm (recursive)
    if meta.is_dir() {
        let walker = walkdir::WalkDir::new(p).into_iter();
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
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;
        Ok(success(
            id,
            StatInfo {
                size,
                file_count,
                dir_count,
                permissions: meta.permissions().mode(),
                uid: meta.uid(),
                gid: meta.gid(),
            },
        ))
    }
    #[cfg(not(unix))]
    {
        Ok(success(
            id,
            StatInfo {
                size,
                file_count,
                dir_count,
                permissions: 0,
                uid: 0,
                gid: 0,
            },
        ))
    }
}

/// Thay đổi quyền truy cập (chmod).
#[tauri::command]
pub fn fs_chmod(request: Req<ModeReq>, window: tauri::Window, state: tauri::State<'_, AppState>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(p.mode)).map_err(|e| backend(e.to_string()))?;
        Ok(success(id, ()))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, p.mode);
        Err(crate::contract::unavailable(
            "Lệnh chmod không được hỗ trợ trên hệ điều hành này",
        ))
    }
}

/// Thay đổi chủ sở hữu (chown).
#[tauri::command]
pub fn fs_chown(request: Req<OwnerReq>, window: tauri::Window, state: tauri::State<'_, AppState>) -> IpcResult<()> {
    let (id, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    filen_gui::local_fs::chown_local(&path, p.uid, p.gid).map_err(backend)?;
    Ok(success(id, ()))
}

/// Lấy thông tin dung lượng còn trống của một đường dẫn phân vùng (dành cho Local).
#[tauri::command]
pub fn fs_get_free_space(
    request: Req<PathReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<u64> {
    let (id, p) = validate(request)?;
    let path = contained_existing(&state, window.label(), &p.path)?;
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(path.as_bytes()).map_err(|e| backend(e.to_string()))?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
            // Khối lượng khả dụng * kích thước khối
            Ok(success(id, stat.f_bavail as u64 * stat.f_frsize as u64))
        } else {
            Err(backend("Không thể lấy dung lượng phân vùng trống".to_string()))
        }
    }
    #[cfg(not(unix))]
    {
        Ok(success(id, 0))
    }
}

/// Dữ liệu đầu vào cấu hình cho tìm kiếm tệp tin.
#[derive(serde::Deserialize, Debug)]
pub struct SearchOptions {
    /// Sử dụng tìm kiếm tương đối (Fuzzy search) hay chính xác.
    pub fuzzy: bool,
    /// Từ khóa nội dung nếu muốn tìm bên trong văn bản (Content search).
    #[serde(deserialize_with = "crate::contract::present_nullable")]
    pub content_query: Option<String>,
    /// Dung lượng tệp nhỏ nhất.
    #[serde(deserialize_with = "crate::contract::present_nullable")]
    pub min_size: Option<u64>,
    /// Dung lượng tệp lớn nhất.
    #[serde(deserialize_with = "crate::contract::present_nullable")]
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
pub async fn fs_search_local(
    request: Req<SearchReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<Vec<SearchResult>> {
    let (id, request) = validate(request)?;
    let path = contained_existing(&state, window.label(), &request.path)?;
    let query = request.query;
    let options = request.options;
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    use std::path::Path;

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
            if size < min_s {
                continue;
            }
        }
        if let Some(max_s) = opts.max_size {
            if size > max_s {
                continue;
            }
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
            if !cq.trim().is_empty() {
                if meta.is_dir() {
                    continue;
                } // Không đọc thư mục
                if size > 10 * 1024 * 1024 {
                    continue;
                } // Bỏ qua file > 10MB để tránh treo ứng dụng

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

        let mod_time = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mod_time_str = format!("{}", mod_time);

        results.push(SearchResult {
            item: filen_gui::models::FileItem {
                name: file_name,
                is_dir: meta.is_dir(),
                size,
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

    Ok(success(id, results))
}

#[tauri::command]
pub async fn fs_sudo_exec(
    request: Req<SudoReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<()> {
    let (_, p) = validate(request)?;
    require_confirmation(p.confirmed)?;
    validate_sudo_argv(&p.action, &p.args)?;
    let _ = (window, state);
    unreachable!("privileged actions are rejected by validate_sudo_argv")
}

#[tauri::command]
pub async fn fs_picker_select(
    request: Req<PickerReq>,
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
) -> IpcResult<String> {
    let (id, _) = validate(request)?;
    let dialog = rfd::FileDialog::new().set_parent(&window);
    let selected = tauri::async_runtime::spawn_blocking(move || dialog.pick_folder())
        .await
        .map_err(|error| backend(error.to_string()))?
        .ok_or_else(|| crate::contract::invalid("directory selection was cancelled"))?;
    let root = state.picker_roots.replace(window.label(), &selected)?;
    Ok(success(id, root.to_string_lossy().into_owned()))
}
