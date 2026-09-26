/*
[INTEGRITY NOTES]
- Mục đích: Tầng API — cửa mỏng cho giao diện (ngôn ngữ / theme / font).
- Trách nhiệm: Chuyển tay xuống `actions::appearance` và `settings::appearance`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::types::{FontInfo, ThemeInfo};
use crate::ipc::{async_command_result, command_result, Empty, IpcErrorCode, IpcResult, Req};
use crate::logic::fastlane::fastlane;
use crate::settings::appearance::AppearanceSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangContentRequest {
    pub lang_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAppearanceRequest {
    pub settings: AppearanceSettings,
}

#[tauri::command]
pub fn get_available_langs(request: Req<Empty>) -> IpcResult<Vec<String>> {
    command_result(request, IpcErrorCode::Io, |_| {
        Ok(crate::actions::appearance::scan_lang_codes())
    })
}

#[tauri::command]
pub fn get_lang_content(request: Req<LangContentRequest>) -> IpcResult<serde_json::Value> {
    command_result(request, IpcErrorCode::Io, |p| {
        crate::actions::appearance::read_lang_content(&p.lang_code)
    })
}

#[tauri::command]
pub fn get_available_themes(request: Req<Empty>) -> IpcResult<Vec<ThemeInfo>> {
    command_result(request, IpcErrorCode::Io, |_| {
        Ok(crate::actions::appearance::scan_themes())
    })
}

#[tauri::command]
pub fn get_available_fonts(request: Req<Empty>) -> IpcResult<Vec<FontInfo>> {
    command_result(request, IpcErrorCode::Io, |_| {
        Ok(crate::actions::appearance::scan_fonts())
    })
}

#[tauri::command]
pub async fn get_appearance(request: Req<Empty>) -> IpcResult<AppearanceSettings> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(crate::settings::appearance::load_appearance).await
    })
    .await
}

#[tauri::command]
pub async fn set_appearance(request: Req<SetAppearanceRequest>) -> IpcResult<AppearanceSettings> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        fastlane(move || {
            crate::settings::appearance::save_appearance(&p.settings)?;
            Ok(p.settings)
        })
        .await
    })
    .await
}
