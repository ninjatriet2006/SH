/*
[INTEGRITY NOTES]
- Mục đích: Tìm thư mục tài nguyên chạy kèm binary (`langs/`, `themes/`).
- Trách nhiệm: Dò MỘT LẦN gốc tài nguyên rồi cache, để mọi module dùng chung
  một đường dẫn thay vì mỗi nơi tự đoán.
- Tương tác: `actions::appearance` đọc các thư mục tài nguyên (`langs/`/`themes/`/`fonts/`) qua đây; `api::appearance_manager` là cửa mỏng bên trên.
*/

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::core::debug;

/// Gốc tài nguyên đã dò (cache) — mọi resource phải cùng một base, nếu không sẽ
/// xảy ra trạng thái nửa vời (thư mục này thấy, thư mục kia không).
static RESOURCE_BASE: OnceLock<PathBuf> = OnceLock::new();

/// Tauri biết chính xác resource directory của app đã đóng gói. Gọi hàm này
/// trong setup trước command đầu tiên để không phụ thuộc layout từng nền tảng.
pub fn init_resource_base(path: PathBuf) {
    if path.join(RESOURCE_ANCHOR).is_dir() {
        let _ = RESOURCE_BASE.set(path);
    } else {
        // Warn only: giữ nguyên (không set base) vì thiếu langs/ → UI hiện raw ID.
        debug::warn(None, "resources", format!("init_resource_base thiếu {}/ trong {}", RESOURCE_ANCHOR, path.display()));
    }
}

/// `langs/` làm mốc neo để nhận ra thư mục gốc app: đây là tài nguyên bắt buộc
/// (thiếu là UI hiện raw ID) và luôn được builder copy kèm binary.
pub const RESOURCE_ANCHOR: &str = "langs";

/// Dò gốc tài nguyên theo thứ tự:
///   1. CWD — khi người dùng chạy binary từ thư mục app;
///   2. Thư mục chứa binary rồi lần lượt các cấp cha — layout `release/<app>/`
///      mà GUI Builder xuất ra;
///   3. Chỉ ở bản debug: `CARGO_MANIFEST_DIR/..` — lúc `cargo tauri dev`, CWD là
///      `backend/` và binary ở `target/debug/`, cả hai đều KHÔNG có `langs/`
///      nên không có nhánh này thì dev mode luôn mất từ điển.
fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &std::path::Path| p.join(RESOURCE_ANCHOR).is_dir();

    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().filter(|dir| has_anchor(dir)) {
            return dir.to_path_buf();
        }
    }

    #[cfg(debug_assertions)]
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    // Không tìm được: warn rồi trả CWD để hành vi vẫn xác định (không panic).
    // Thiếu langs/ → UI hiện raw ID.
    let fallback = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    debug::warn(None, "resources", format!("không tìm thấy {}/, dùng CWD {}", RESOURCE_ANCHOR, fallback.display()));
    fallback
}

/// Gốc tài nguyên (đã cache).
pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(detect_resource_base)
}

/// Đường dẫn tới một thư mục tài nguyên cụ thể (`langs`, `themes`…).
pub fn resource_dir(name: &str) -> PathBuf {
    resource_base().join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gốc dò được phải chứa anchor, nếu không thì `langs/` rỗng và toàn bộ UI
    /// hiện raw ID (`nav_explorer` thay vì "Explorer").
    #[test]
    fn resource_base_chua_anchor() {
        let base = resource_base();
        assert!(
            base.join(RESOURCE_ANCHOR).is_dir(),
            "resource_base() = {} không chứa {}/",
            base.display(),
            RESOURCE_ANCHOR
        );
    }

    /// Mọi resource lấy từ cùng một gốc.
    #[test]
    fn resource_dir_dung_chung_goc() {
        let base = resource_base();
        for name in ["langs", "themes"] {
            assert_eq!(resource_dir(name), base.join(name));
        }
    }
}
