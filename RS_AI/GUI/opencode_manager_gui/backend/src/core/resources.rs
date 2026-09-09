/*
[INTEGRITY NOTES]
 - Mục đích: Tìm thư mục tài nguyên chạy kèm binary (`langs/`, `themes/`, `fonts/`).
- Trách nhiệm: Dò MỘT LẦN gốc tài nguyên rồi cache, để mọi module dùng chung
  một đường dẫn thay vì mỗi nơi tự đoán.
- Tương tác: `api::lang`, `api::theme`.

Bài học đã trả giá ở subscription_manager: mỗi API tự dò riêng thì `cargo tauri
dev` (CWD = backend/, binary ở target/debug/) miss hết → từ điển rỗng → toàn bộ
UI hiện raw key. Ở đây làm đúng ngay từ đầu.
*/

use std::path::PathBuf;
use std::sync::OnceLock;

static RESOURCE_BASE: OnceLock<PathBuf> = OnceLock::new();

/// `langs/` làm mốc neo để nhận ra thư mục gốc app: đây là tài nguyên bắt buộc
/// (thiếu là UI hiện raw key) và luôn được builder copy kèm binary.
/// KHÔNG neo bằng `themes/` vì thư mục đó có thể được tự tạo rỗng.
pub const RESOURCE_ANCHOR: &str = "langs";

fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &std::path::Path| p.join(RESOURCE_ANCHOR).is_dir();

    // Bản debug/test trong workspace dùng chung `target/`; nếu dò từ binary
    // trước, một `langs/` ở workspace root có thể bị nhận nhầm là của app này.
    #[cfg(debug_assertions)]
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    // 1. CWD — user chạy binary từ thư mục app.
    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    // 2. Thư mục chứa binary rồi lần lượt các cấp cha (layout `release/<app>/`).
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            if has_anchor(dir) {
                return dir.to_path_buf();
            }
            cur = dir.parent();
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn configured_resource_base() -> Option<PathBuf> {
    std::env::var_os("OPENCODE_MANAGER_RESOURCE_DIR")
        .map(PathBuf::from)
        .filter(|path| path.join(RESOURCE_ANCHOR).is_dir())
}

/// Gốc tài nguyên (đã cache).
pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(|| configured_resource_base().unwrap_or_else(detect_resource_base))
}

/// Đường dẫn tới một thư mục tài nguyên cụ thể (`langs`, `themes`…).
pub fn resource_dir(name: &str) -> PathBuf {
    resource_base().join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_base_chua_anchor() {
        let base = resource_base();
        assert!(
            base.join(RESOURCE_ANCHOR).is_dir(),
            "resource_base() = {} không chứa {}/ — UI sẽ hiện raw key",
            base.display(),
            RESOURCE_ANCHOR
        );
    }

    #[test]
    fn resource_dir_dung_chung_goc() {
        let base = resource_base();
        for name in ["langs", "themes", "fonts"] {
            assert_eq!(resource_dir(name), base.join(name));
        }
    }
}
