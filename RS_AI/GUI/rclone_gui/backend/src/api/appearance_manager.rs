/*
[INTEGRITY NOTES]
- Mục đích: Tầng API — cửa mỏng cho giao diện (ngôn ngữ / theme / font).
- Trách nhiệm: chuyển tay xuống `actions::appearance` (đọc tài nguyên) và
  `settings::appearance` (giữ lựa chọn). KHÔNG tự đọc file.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư. Tên lệnh
  giữ NGUYÊN so với 3 loader cũ để frontend không phải đổi.
*/

use crate::actions::types::{FontInfo, ThemeInfo};
use crate::logic::fastlane::fastlane;
use crate::settings::appearance::AppearanceSettings;

#[tauri::command]
pub fn get_available_langs() -> Result<Vec<String>, String> {
    Ok(crate::actions::appearance::scan_lang_codes())
}

#[tauri::command]
pub fn get_lang_content(lang_code: String) -> Result<serde_json::Value, String> {
    crate::actions::appearance::read_lang_content(&lang_code)
}

#[tauri::command]
pub fn get_available_themes() -> Result<Vec<ThemeInfo>, String> {
    Ok(crate::actions::appearance::scan_themes())
}

#[tauri::command]
pub fn get_available_fonts() -> Result<Vec<FontInfo>, String> {
    Ok(crate::actions::appearance::scan_fonts())
}

#[tauri::command]
pub async fn get_appearance() -> Result<AppearanceSettings, String> {
    fastlane(crate::settings::appearance::load_appearance).await
}

#[tauri::command]
pub async fn set_appearance(settings: AppearanceSettings) -> Result<AppearanceSettings, String> {
    fastlane(move || {
        crate::settings::appearance::save_appearance(&settings)?;
        Ok(settings)
    })
    .await
}
