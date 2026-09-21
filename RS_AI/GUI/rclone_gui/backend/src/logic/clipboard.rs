/* [INTEGRITY NOTES]
 * UNIVERSAL: tách từ `core/sys.rs` (S2 tách vai) — clipboard giả lập qua file JSON
 * temp là logic nghiệp vụ, không phải syscall OS thô.
 */

use crate::core::task::blocking;
use serde::{Deserialize, Serialize};
use std::env;

/// UNIVERSAL: một mục clipboard (pane nguồn + đường dẫn đầy đủ).
#[derive(Serialize, Deserialize, Clone)]
pub struct OSClipboardItem {
    pub pane: String,
    pub path: String,
}

/// UNIVERSAL: payload clipboard (danh sách item + cờ cắt/dán).
#[derive(Serialize, Deserialize)]
pub struct OSClipboardData {
    pub items: Vec<OSClipboardItem>,
    pub is_cut: bool,
}

/// UNIVERSAL: lưu clipboard vào file JSON trong thư mục tạm hệ điều hành.
pub async fn os_clipboard_set(items: Vec<OSClipboardItem>, is_cut: bool) -> Result<(), String> {
    blocking(move || {
        let data = OSClipboardData { items, is_cut };
        // UNIVERSAL: serialize không bao giờ fail với struct này nhưng vẫn map_err thay vì unwrap.
        let json = serde_json::to_string(&data).map_err(|e| e.to_string())?;
        std::fs::write(env::temp_dir().join("rclone_gui_clipboard.json"), json)
            .map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
}

/// UNIVERSAL: đọc clipboard, chưa copy gì → `None`.
pub async fn os_clipboard_get() -> Result<Option<OSClipboardData>, String> {
    blocking(|| {
        let path = env::temp_dir().join("rclone_gui_clipboard.json");

        if path.exists() {
            let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let data: OSClipboardData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    })
    .await
}
