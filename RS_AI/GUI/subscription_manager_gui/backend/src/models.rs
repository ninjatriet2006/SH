/*
[INTEGRITY NOTES]
- Mục đích: Định nghĩa các Struct lõi cho hệ thống quản lý đăng ký dịch vụ (Subscription Manager).
- Trách nhiệm: Cung cấp kiểu dữ liệu cho User, Package, và Subscription. Sử dụng Serde để serialize/deserialize qua lại giữa Frontend và Backend.
- Tương tác: Các file `user_api.rs`, `package_api.rs`, `subscription_api.rs` sẽ gọi và thao tác với các Struct này. Storage module sẽ lưu trữ các Struct này.
*/

// Nhúng thư viện Serialize và Deserialize từ Serde để chuyển đổi dữ liệu
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Định nghĩa cấu trúc cho một Người dùng (User)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    // ID duy nhất của người dùng
    pub id: String,
    // Tên đăng nhập hoặc tên hiển thị của người dùng
    pub username: String,
    // Địa chỉ email (tùy chọn, có thể có hoặc không)
    pub email: Option<String>,
    // Số điện thoại (tùy chọn)
    #[serde(default)]
    pub phone: Option<String>,
    // URL liên hệ (tùy chọn, ví dụ Facebook, Telegram)
    #[serde(default)]
    pub contact_url: Option<String>,
    // Thời điểm người dùng được tạo (được lưu dưới dạng số nguyên timestamp)
    pub created_at: i64,
}

// Định nghĩa cấu trúc cho Gói dịch vụ (Package)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    // ID duy nhất của gói dịch vụ
    pub id: String,
    // Tên của gói dịch vụ (ví dụ: Gói Cơ Bản, Gói Cao Cấp)
    pub name: String,
    // Mô tả chi tiết về gói dịch vụ (tùy chọn)
    pub description: Option<String>,
    // Thời lượng của gói tính bằng ngày (ví dụ: 30 ngày)
    pub duration_days: u32,
    // Giá tiền của gói (sử dụng serde default để tương thích ngược)
    #[serde(default)]
    pub price: u64,
}

// Định nghĩa cấu trúc cho Gói đăng ký (Subscription) của người dùng
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    // ID duy nhất của gói đăng ký
    pub id: String,
    // ID của người dùng sở hữu gói đăng ký này
    pub user_id: String,
    // ID của gói dịch vụ được đăng ký
    pub package_id: String,
    // Thời điểm hết hạn của gói đăng ký (timestamp)
    pub expiration_date: i64,
    // Trạng thái kích hoạt của gói đăng ký (true = đang hoạt động, false = đã hủy/hết hạn)
    pub is_active: bool,
}

// Định nghĩa cấu trúc Lịch sử giao dịch (Transaction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    // ID duy nhất của giao dịch
    pub id: String,
    // ID của người dùng thực hiện giao dịch
    pub user_id: String,
    // ID của gói dịch vụ liên quan
    pub package_id: String,
    // Số tiền thu thực tế (VNĐ)
    pub amount: u64,
    // Hành động: "ASSIGN" (Gán mới) hoặc "RENEW" (Gia hạn)
    pub action: String,
    // Thời điểm ghi nhận giao dịch (timestamp)
    pub created_at: i64,
}

// Định nghĩa cấu trúc cho Theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: String, // "dark" or "light"
    pub colors: HashMap<String, String>,
}

// Định nghĩa cấu trúc cho Font (chứa thông tin metadata cho cả Web Font và Local Font)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontInfo {
    pub id: String,
    pub name: String,
    pub provider: String, // "google", "local", ...
    pub family: String,
    pub is_local: bool,
    pub src_url: Option<String>,
}
