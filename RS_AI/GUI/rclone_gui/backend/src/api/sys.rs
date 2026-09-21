/*
[INTEGRITY NOTES]
- Mục đích: API hệ điều hành / ứng dụng ngoài (Open With, clipboard, custom actions).
- Trách nhiệm: Tầng API mỏng — gọi `actions::view` + `logic::{clipboard,custom_action}`.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

#[tauri::command]
pub async fn sys_open_with(
    path: String,
    exec_cmd: Option<String>,
    app: Option<String>,
) -> Result<(), String> {
    crate::actions::view::sys_open_with(path, exec_cmd, app).await
}

#[tauri::command]
pub async fn sys_list_apps() -> Result<Vec<crate::actions::system::DesktopApp>, String> {
    crate::actions::system::sys_list_apps().await
}

#[tauri::command]
pub async fn os_clipboard_set(
    items: Vec<crate::logic::clipboard::OSClipboardItem>,
    is_cut: bool,
) -> Result<(), String> {
    crate::logic::clipboard::os_clipboard_set(items, is_cut).await
}

#[tauri::command]
pub async fn os_clipboard_get() -> Result<Option<crate::logic::clipboard::OSClipboardData>, String> {
    crate::logic::clipboard::os_clipboard_get().await
}

#[tauri::command]
pub async fn sys_get_custom_actions() -> Result<Vec<crate::logic::custom_action::CustomAction>, String>
{
    crate::logic::custom_action::sys_get_custom_actions().await
}

#[tauri::command]
pub async fn sys_get_valid_actions(
    files: Vec<crate::logic::custom_action::SimpleFileItem>,
) -> Result<Vec<crate::logic::custom_action::CustomAction>, String> {
    crate::logic::custom_action::sys_get_valid_actions(files).await
}

#[tauri::command]
pub async fn sys_execute_custom_action(
    exec_template: String,
    base_path: String,
    file_names: Vec<String>,
) -> Result<(), String> {
    crate::logic::custom_action::sys_execute_custom_action(exec_template, base_path, file_names).await
}
