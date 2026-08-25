/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp cơ chế lưu trữ dữ liệu đơn giản bằng JSON file.
- Trách nhiệm: Đọc và ghi các danh sách User, Package, và Subscription ra file `data.json` tại thư mục gốc.
- Tương tác: Được gọi bởi các API modules để lưu trữ hoặc lấy dữ liệu bền vững (persistent data).
*/

// Import thư viện File và xử lý đường dẫn
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
// Import Serialize/Deserialize và các Models đã tạo
use serde::{Deserialize, Serialize};
use crate::models::{Package, Subscription, User, Transaction};

// Tên file JSON dùng để lưu trữ dữ liệu (đặt ở thư mục storage ngang hàng với dist, icon)
const DATA_FILE: &str = "storage/data.json";

// Cấu trúc DataStore chứa toàn bộ dữ liệu của ứng dụng
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Khởi tạo một đối tượng DataStore mới với các danh sách rỗng
impl Default for DataStore {
    fn default() -> Self {
        Self {
            users: Vec::new(),
            packages: Vec::new(),
            subscriptions: Vec::new(),
            transactions: Vec::new(),
        }
    }
}

// Hàm đọc dữ liệu từ file JSON
pub fn load_data() -> DataStore {
    // Kiểm tra xem file dữ liệu đã tồn tại hay chưa
    if Path::new(DATA_FILE).exists() {
        // Mở file với quyền đọc
        let mut file = match File::open(DATA_FILE) {
            Ok(f) => f,
            Err(_) => return DataStore::default(), // Trả về mặc định nếu lỗi mở file
        };
        // Khởi tạo chuỗi để chứa nội dung file
        let mut contents = String::new();
        // Đọc toàn bộ nội dung file vào chuỗi
        if file.read_to_string(&mut contents).is_ok() {
            // Cố gắng chuyển đổi chuỗi JSON thành đối tượng DataStore
            if let Ok(data) = serde_json::from_str(&contents) {
                return data;
            }
        }
    }
    // Trả về dữ liệu trống nếu file không tồn tại hoặc lỗi đọc/parse JSON
    DataStore::default()
}

// Hàm ghi dữ liệu xuống file JSON
pub fn save_data(data: &DataStore) -> Result<(), String> {
    // Chuyển đối tượng DataStore thành chuỗi JSON với định dạng dễ đọc (pretty)
    let json = match serde_json::to_string_pretty(data) {
        Ok(j) => j,
        Err(e) => return Err(format!("Lỗi chuyển đổi dữ liệu thành JSON: {}", e)),
    };
    
    // Đảm bảo thư mục cha tồn tại
    if let Some(parent) = Path::new(DATA_FILE).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // Mở file (hoặc tạo mới nếu chưa có) và xóa nội dung cũ (truncate)
    let mut file = match File::create(DATA_FILE) {
        Ok(f) => f,
        Err(e) => return Err(format!("Lỗi tạo file lưu trữ: {}", e)),
    };
    
    // Ghi chuỗi JSON vào file
    match file.write_all(json.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Lỗi ghi dữ liệu vào file: {}", e)),
    }
}
