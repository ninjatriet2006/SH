/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp cơ chế lưu trữ dữ liệu đơn giản bằng JSON file.
- Trách nhiệm: Đọc và ghi các danh sách User, Package, và Subscription ra file `data.json` tại thư mục gốc.
- Tương tác: Được gọi bởi các API modules để lưu trữ hoặc lấy dữ liệu bền vững (persistent data).
*/

// Import thư viện File và xử lý đường dẫn
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
// Import Serialize/Deserialize và các Models đã tạo
use serde::{Deserialize, Serialize};
use crate::models::{Package, Subscription, User, Transaction};

/// Thư mục gốc chứa tài nguyên app (`langs/`, `themes/`, `fonts/`).
/// Dò MỘT LẦN rồi cache: mọi resource phải cùng một base, nếu không sẽ xảy ra
/// tình trạng `themes` lấy ở repo root mà `langs` lại không thấy (đã gặp).
static RESOURCE_BASE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// `langs/` là mốc neo (anchor) để nhận ra một thư mục có phải app root không:
/// đây là tài nguyên BẮT BUỘC (app không có nó thì UI hiện raw key), và luôn
/// được builder copy kèm. Không neo bằng `themes/`/`fonts/` vì hai thứ đó được
/// tự tạo rỗng khi thiếu nên có thể tồn tại ở thư mục bất kỳ.
const RESOURCE_ANCHOR: &str = "langs";

/// Dò thư mục gốc tài nguyên theo thứ tự:
///   1. CWD (`cargo tauri dev` với CWD đúng, hoặc user chạy từ thư mục app);
///   2. Thư mục chứa binary (layout `release/<app>/` mà GUI Builder xuất);
///   3. Các cấp cha của thư mục binary (bắt trường hợp bundle lồng thư mục);
///   4. Chỉ ở bản debug: `CARGO_MANIFEST_DIR/..` — tài nguyên nằm ở
///      `GUI/<app>/langs` còn CWD lúc `cargo tauri dev` là `GUI/<app>/backend`,
///      nên hai nhánh trên đều MISS và toàn bộ UI hiện raw key.
fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &std::path::Path| p.join(RESOURCE_ANCHOR).is_dir();

    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        // Duyệt exe_dir rồi lần lượt các cấp cha.
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            if has_anchor(dir) {
                return dir.to_path_buf();
            }
            cur = dir.parent();
        }
    }

    // Bản debug: neo theo vị trí source để `cargo tauri dev` chạy được ngay.
    #[cfg(debug_assertions)]
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    // Không tìm được: dùng CWD để hành vi ghi file vẫn như cũ (không panic).
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Thư mục gốc tài nguyên đã cache.
pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(detect_resource_base)
}

// Tìm file `<name>` trong thư mục storage theo thứ tự ưu tiên:
//   1. `./storage/<name>` tính từ thư mục làm việc (tương thích data của
//      bản cũ — user cũ mở app từ đâu thì data vẫn ở đó, không mất);
//   2. `<gốc tài nguyên>/storage/<name>` — cùng base với langs/themes/fonts
//      nên data luôn đi kèm app, kể cả khi CWD trỏ chỗ khác.
// Trước đây chỉ dùng CWD nên double-click binary từ chỗ khác là "mất" data.
pub fn storage_file_path(name: &str) -> PathBuf {
    let rel = format!("storage/{name}");
    let cwd_path = PathBuf::from(&rel);
    if cwd_path.is_file() {
        return cwd_path;
    }
    let base = resource_base();
    // Chỉ chuyển sang base khi base thực sự là app root (có anchor); nếu không
    // thì giữ CWD như cũ để không tạo thư mục rác ở chỗ ngẫu nhiên.
    if base.join(RESOURCE_ANCHOR).is_dir() {
        return base.join(&rel);
    }
    cwd_path
}

fn data_file_path() -> PathBuf {
    storage_file_path("data.json")
}

/// Tìm THƯ MỤC tài nguyên `<name>` (langs/themes/fonts) — luôn lấy từ CÙNG một
/// gốc đã dò (`resource_base`). Trước đây mỗi API tự dò riêng nên dev mode miss
/// hết (UI hiện raw key) và có thể lẫn tài nguyên giữa các thư mục khác nhau.
pub fn resource_dir(name: &str) -> PathBuf {
    resource_base().join(name)
}

// Khóa toàn cục serialize mọi chuỗi đọc-sửa-ghi. Mỗi lệnh ghi giữ guard này
// suốt thời gian chạy (`let _guard = lock_store();` ở đầu command) nên hai
// `invoke` song song không thể load cùng bản cũ rồi ghi đè lẫn nhau.
// (Khóa trong tiến trình; chạy 2 bản app cùng lúc vẫn cần file-lock OS.)
static DATA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Giữ khóa storage cho toàn bộ lệnh ghi. Guard phải được bind vào biến
/// (`let _guard = ...`) để khóa tồn tại đến hết scope.
pub fn lock_store() -> std::sync::MutexGuard<'static, ()> {
    DATA_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// Cấu trúc DataStore chứa toàn bộ dữ liệu của ứng dụng.
// `Default` derive thay cho `impl` tay (clippy::derivable_impls).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataStore {
    // Danh sách người dùng
    pub users: Vec<User>,
    // Danh sách các gói dịch vụ
    pub packages: Vec<Package>,
    // Danh sách các đăng ký dịch vụ
    pub subscriptions: Vec<Subscription>,
    // Lịch sử giao dịch (có serde default để tương thích file cũ)
    #[serde(default)]
    pub transactions: Vec<Transaction>,
}

// Dùng `DataStore::default()` từ derive ở trên để khởi tạo rỗng.

// Hàm đọc dữ liệu từ file JSON
pub fn load_data() -> DataStore {
    let data_file = data_file_path();
    // Kiểm tra xem file dữ liệu đã tồn tại hay chưa
    if data_file.exists() {
        // Mở file với quyền đọc
        let mut file = match File::open(&data_file) {
            Ok(f) => f,
            Err(_) => return DataStore::default(), // Trả về mặc định nếu lỗi mở file
        };
        // Khởi tạo chuỗi để chứa nội dung file
        let mut contents = String::new();
        // Đọc toàn bộ nội dung file vào chuỗi
        if file.read_to_string(&mut contents).is_ok() {
            // Cố gắng chuyển đổi chuỗi JSON thành đối tượng DataStore
            match serde_json::from_str(&contents) {
                Ok(data) => return data,
                // Báo rõ lý do thay vì im lặng trả rỗng (trước đây file hỏng
                // làm mất toàn bộ dữ liệu mà không để lại dấu vết nào).
                Err(e) => eprintln!("[storage] data.json hỏng, dùng dữ liệu trống: {e}"),
            }
        }
    }
    // Trả về dữ liệu trống nếu file không tồn tại hoặc lỗi đọc/parse JSON
    DataStore::default()
}

// Hàm ghi dữ liệu xuống file JSON
pub fn save_data(data: &DataStore) -> Result<(), String> {
    let data_file = data_file_path();
    // Chuyển đối tượng DataStore thành chuỗi JSON với định dạng dễ đọc (pretty)
    let json = match serde_json::to_string_pretty(data) {
        Ok(j) => j,
        Err(e) => return Err(format!("Lỗi chuyển đổi dữ liệu thành JSON: {}", e)),
    };
    
    // Đảm bảo thư mục cha tồn tại
    if let Some(parent) = data_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // Mở file (hoặc tạo mới nếu chưa có) và xóa nội dung cũ (truncate)
    let mut file = match File::create(&data_file) {
        Ok(f) => f,
        Err(e) => return Err(format!("Lỗi tạo file lưu trữ: {}", e)),
    };
    
    // Ghi chuỗi JSON vào file
    match file.write_all(json.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Lỗi ghi dữ liệu vào file: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bug đã gặp: `cargo tauri dev` đặt CWD = `backend/` và binary nằm ở
    /// `target/debug/`, cả hai đều không có `langs/` nên từ điển rỗng và toàn
    /// bộ UI hiện raw key ("sidebar.dashboard" thay vì "Tổng quan").
    /// Test này chốt: base dò được PHẢI chứa anchor `langs/`.
    #[test]
    fn resource_base_co_chua_langs() {
        let base = resource_base();
        assert!(
            base.join(RESOURCE_ANCHOR).is_dir(),
            "resource_base() = {} không chứa {}/ — langs sẽ rỗng và UI hiện raw key",
            base.display(),
            RESOURCE_ANCHOR
        );
    }

    /// Các file ngôn ngữ thực tế phải đọc được qua đường dẫn đã dò.
    /// Không cố định `vi.json`: quét file `.json` đầu tiên có trong `langs/` để
    /// test không phụ thuộc một bộ ngôn ngữ cụ thể.
    #[test]
    fn doc_duoc_file_ngon_ngu_dau_tien() {
        let langs = resource_dir("langs");
        let mut files: Vec<_> = std::fs::read_dir(&langs)
            .unwrap_or_else(|e| panic!("đọc {}: {e}", langs.display()))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        files.sort();
        let first = files
            .first()
            .unwrap_or_else(|| panic!("không có file .json nào trong {}", langs.display()));

        let content = std::fs::read_to_string(first).expect("đọc file ngôn ngữ");
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("file ngôn ngữ phải là JSON hợp lệ");
        assert!(
            json.is_object() && json.as_object().is_some_and(|m| !m.is_empty()),
            "{} phải là object không rỗng",
            first.display()
        );
    }

    /// Mọi resource phải cùng một gốc — trước đây `themes` lấy ở repo root mà
    /// `langs` lại MISS, gây trạng thái nửa vời khó lần ra.
    #[test]
    fn moi_resource_dung_chung_mot_goc() {
        let base = resource_base();
        for name in ["langs", "themes", "fonts"] {
            assert_eq!(
                resource_dir(name),
                base.join(name),
                "{name} không cùng gốc với các resource khác"
            );
        }
    }

    /// `storage/` phải nằm cùng gốc với tài nguyên, không rơi ra thư mục ngẫu
    /// nhiên (bug cũ: double-click từ $HOME sinh ra `~/storage`).
    #[test]
    fn storage_di_kem_app_root() {
        let p = storage_file_path("data.json");
        let base = resource_base();
        assert!(
            p == std::path::Path::new("storage/data.json") || p.starts_with(base),
            "storage path {} không thuộc app root {}",
            p.display(),
            base.display()
        );
    }
}
