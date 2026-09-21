/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints thao tác File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend (tham số đường dẫn gộp chung kiểu Remote::/Path), gọi tầng `logic`/`actions` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Copy/move/cancel chạy qua `logic::jobs`.
*/
// UNIVERSAL: fs_copy/fs_move/fs_cancel cũ đã gộp về jobs — xóa wrapper đường cũ.

use serde::{Deserialize, Serialize};

use crate::actions::perm::Policy;

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
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::system::get_home_dir`.
    crate::actions::system::get_home_dir().await
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

/// Tên hàm: get_user_places
/// Mô tả: Danh sách thư mục người dùng chuẩn XDG để dựng mục "Truy cập nhanh".
/// Chỉ trả về thư mục thực sự tồn tại, nên không hiện mục dẫn tới đường dẫn rỗng.
pub async fn get_user_places() -> Result<Vec<UserPlace>, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::system::get_user_places`.
    crate::actions::system::get_user_places().await
}

pub async fn open_in_terminal(path: String) -> Result<(), String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::system::open_in_terminal`.
    crate::actions::system::open_in_terminal(path).await
}

pub async fn fs_get_thumbnail(path: String) -> Result<String, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::view::execute_thumbnail`.
    crate::actions::view::execute_thumbnail(path).await
}

pub fn fs_temp_dir() -> String {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::system::fs_temp_dir`.
    crate::actions::system::fs_temp_dir()
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
