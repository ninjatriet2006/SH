/*
[INTEGRITY NOTES]
- Mục đích: API quản lý Theme — đọc các file JSON trong `themes/`.
- Trách nhiệm: Liệt kê theme có thật, sắp xếp theo id để "theme đầu tiên" ổn
  định giữa các máy. KHÔNG tự sinh `default.json`: theme nào cũng như nhau,
  thiếu hết thì frontend giữ màu gốc trong CSS.
- Tương tác: frontend `store/useThemeStore.ts`.
*/

use crate::core::resources::resource_dir;
use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    pub colors: HashMap<String, String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_available_themes(request: Req<Empty>) -> IpcResult<Vec<Theme>> {
    let (request_id, _) = request.validate()?;
    let dir = resource_dir("themes");
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match fs::read_to_string(&path) {
                // Báo rõ file nào hỏng thay vì im lặng bỏ qua — theme lỗi biến
                // mất khỏi danh sách mà không để lại dấu vết thì rất khó lần.
                Ok(content) => match serde_json::from_str::<Theme>(&content) {
                    Ok(theme) => themes.push(theme),
                    Err(e) => eprintln!("[theme] bỏ qua file hỏng {}: {e}", path.display()),
                },
                Err(e) => eprintln!("[theme] không đọc được {}: {e}", path.display()),
            }
        }
    }

    themes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(respond(request_id, themes))
}
