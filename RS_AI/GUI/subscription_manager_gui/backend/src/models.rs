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
    #[serde(default)]
    pub email: Option<String>,
    // Số điện thoại (tùy chọn)
    #[serde(default)]
    pub phone: Option<String>,
    // URL liên hệ (tùy chọn, ví dụ Facebook, Telegram)
    #[serde(default)]
    pub contact_url: Option<String>,
    // Thời điểm người dùng được tạo (được lưu dưới dạng số nguyên timestamp)
    // `default` để file data.json cũ thiếu field vẫn đọc được thay vì mất toàn bộ.
    #[serde(default)]
    pub created_at: i64,
    // Số dư của khách (VNĐ). CÓ DẤU vì gộp hai trạng thái khác nhau:
    //   > 0  → số dư khả dụng (khách đã nạp trước);
    //   < 0  → công nợ (khách đang nợ tiền).
    // Dùng i64 chứ không phải u64 để biểu diễn được nợ; VNĐ không có phần thập
    // phân nên số nguyên là đủ và tránh sai số dấu phẩy động.
    // `serde(default)` để data.json của bản cũ (chưa có field) vẫn đọc được.
    #[serde(default)]
    pub balance: i64,
}

// Định nghĩa cấu trúc cho Gói dịch vụ (Package)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    // ID duy nhất của gói dịch vụ
    pub id: String,
    // Tên của gói dịch vụ (ví dụ: Gói Cơ Bản, Gói Cao Cấp)
    pub name: String,
    // Mô tả chi tiết về gói dịch vụ (tùy chọn)
    #[serde(default)]
    pub description: Option<String>,
    // Thời lượng của gói tính bằng ngày (ví dụ: 30 ngày)
    #[serde(default)]
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
    #[serde(default)]
    pub expiration_date: i64,
    // Trạng thái kích hoạt của gói đăng ký (true = đang hoạt động, false = đã hủy/hết hạn)
    #[serde(default)]
    pub is_active: bool,
    // Bật tự động gia hạn cho RIÊNG đăng ký này. Mặc định false: không tự ý
    // trừ tiền của khách khi họ chưa đồng ý — người dùng phải tự tick.
    #[serde(default)]
    pub auto_renew: bool,
    // Thời điểm gia hạn tự động gần nhất (timestamp). `None` = chưa từng.
    // Lưu để hiển thị và để phân biệt "chưa chạy" với "đã chạy nhưng thất bại".
    #[serde(default)]
    pub last_auto_renew_at: Option<i64>,
}

// Định nghĩa cấu trúc Lịch sử giao dịch (Transaction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    // ID duy nhất của giao dịch
    pub id: String,
    // ID của người dùng thực hiện giao dịch
    pub user_id: String,
    // ID của gói dịch vụ liên quan. Rỗng với giao dịch không gắn gói (DEPOSIT).
    pub package_id: String,
    // Số tiền ghi nhận (VNĐ)
    #[serde(default)]
    pub amount: u64,
    // Hành động, xem các hằng trong `tx_action`:
    // "ASSIGN" (gán mới), "RENEW" (gia hạn tay),
    // "AUTO_RENEW" (gia hạn tự động, trừ số dư), "DEPOSIT" (nạp/điều chỉnh số dư).
    #[serde(default)]
    pub action: String,
    // Thời điểm ghi nhận giao dịch (timestamp)
    #[serde(default)]
    pub created_at: i64,
}

/// Tên hành động của `Transaction.action`. Trước đây các chuỗi này rải rác
/// dạng literal ở nhiều file nên dễ gõ sai mà compiler không phát hiện.
pub mod tx_action {
    pub const ASSIGN: &str = "ASSIGN";
    pub const RENEW: &str = "RENEW";
    pub const AUTO_RENEW: &str = "AUTO_RENEW";
    pub const DEPOSIT: &str = "DEPOSIT";
}

/// Mã tra cứu thanh toán — dùng làm NỘI DUNG CHUYỂN KHOẢN trên hóa đơn/QR.
///
/// Vì sao cần: sao kê ngân hàng chỉ cho ta thấy số tiền + nội dung chuyển. Nội
/// dung cũ ("Thanh toan don hang tx_1757...") vừa dài quá giới hạn của nhiều
/// ngân hàng, vừa không tra được ai chuyển khi khách gõ thiếu/sai. Mã này ngắn,
/// chỉ dùng ký tự an toàn, và MANG SẴN dấu hiệu nhận dạng người chuyển
/// (`user_token`) nên vẫn truy được chủ giao dịch dù mã bị gõ lệch vài ký tự.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRef {
    /// Mã đầy đủ, ví dụ `SM7K2MX4B9`. Đây là chuỗi khách gõ vào nội dung CK.
    pub code: String,
    /// 4 ký tự nhận dạng người chuyển, nằm trong `code`. Đối chiếu nhanh khi mã
    /// bị gõ sai phần còn lại.
    pub user_token: String,
    pub user_id: String,
    /// Tên khách tại thời điểm phát hành — snapshot để hóa đơn cũ vẫn đọc được
    /// tên dù sau này khách đổi tên.
    pub username: String,
    /// Các giao dịch mà mã này thu tiền cho (1 hoặc nhiều khi in gộp).
    pub transaction_ids: Vec<String>,
    /// Tổng tiền tại thời điểm phát hành (VNĐ).
    pub amount: u64,
    pub created_at: i64,
    /// Thời điểm đối soát xong (đã nhận được tiền). `None` = chưa nhận.
    #[serde(default)]
    pub settled_at: Option<i64>,
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
