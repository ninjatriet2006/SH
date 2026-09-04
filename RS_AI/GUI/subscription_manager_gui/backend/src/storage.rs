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

// Tìm file `<name>` trong thư mục storage theo thứ tự ưu tiên:
//   1. `./storage/<name>` tính từ thư mục làm việc (tương thích data của
//      bản cũ — user cũ mở app từ đâu thì data vẫn ở đó, không mất);
//   2. `<thư mục chứa binary>/storage/<name>` (layout `release/<app>/`
//      mà GUI BUILDER xuất: binary + storage nằm cạnh nhau);
//   3. Nếu chưa có ở đâu: tạo mới ở CWD (mặc định khi dev bằng cargo run).
// Trước đây chỉ dùng CWD nên double-click binary từ chỗ khác là "mất" data.
pub fn storage_file_path(name: &str) -> PathBuf {
    let rel = format!("storage/{name}");
    let cwd_path = PathBuf::from(&rel);
    if cwd_path.is_file() {
        return cwd_path;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Neo bằng `langs/` (builder luôn copy cho app Tauri) chứ KHÔNG
            // neo bằng `storage/` — thư mục đó là runtime data, bản cài mới
            // chưa có nên neo vào nó là rơi lại về CWD (bug đã gặp: double-click
            // từ $HOME tạo ra ~/storage rác).
            if exe_dir.join("langs").is_dir() {
                return exe_dir.join(&rel);
            }
        }
    }
    cwd_path
}

fn data_file_path() -> PathBuf {
    storage_file_path("data.json")
}

/// Tìm THƯ MỤC tài nguyên `<name>` (langs/themes/fonts) — cùng rule với file:
/// CWD trước (tương thích cũ), rồi tới cạnh binary (layout release/).
/// Trước đây mỗi API tự chế một cách tìm khác nhau (themes/fonts chỉ nhìn CWD
/// rồi tự mkdir) nên mở app từ thư mục khác là mất theme/font.
pub fn resource_dir(name: &str) -> PathBuf {
    let cwd_path = PathBuf::from(name);
    if cwd_path.is_dir() {
        return cwd_path;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_path = exe_dir.join(name);
            if exe_path.is_dir() {
                return exe_path;
            }
        }
    }
    cwd_path
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
