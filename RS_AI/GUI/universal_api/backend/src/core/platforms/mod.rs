//! Modular Platform Handlers — Mỗi nền tảng/trang nằm ở một file riêng biệt, không gom chung.
//!
//! Tuân thủ kiến trúc của Cockpit Tools:
//! - `codebuddy_cn`: Tencent Copilot (copilot.tencent.com / codebuddy.cn)
//! - `codebuddy_global`: CodeBuddy International (codebuddy.ai)
//! - `zed`: Zed Cloud
//! - (Sẵn sàng mở rộng: cursor, windsurf, copilot, antigravity, grok...)

pub mod codebuddy_cn;
pub mod codebuddy_global;
pub mod zed;

use serde::{Deserialize, Serialize};

/// Thông tin khởi đầu phiên đăng nhập qua trình duyệt (Device Code / OAuth State).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformLoginStart {
    pub platform_id: String,
    pub auth_url: String,
    pub state: String,
}

/// Token gói kết quả sau khi đăng nhập thành công.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformTokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub domain: String,
}

/// Kết quả polling một chu kỳ.
#[derive(Debug, Clone)]
pub enum PlatformPollOutcome {
    Pending,
    Done(PlatformTokenBundle),
}

/// Thông tin hồ sơ tài khoản từ nhà cung cấp.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformAccountInfo {
    pub uid: String,
    pub nickname: String,
    pub domain: String,
    pub enterprise_id: String,
    pub plan_tier: String,
    pub credits: i64,
}

/// Trait chuẩn hóa mà mọi nền tảng bắt buộc phải triển khai độc lập.
pub trait PlatformHandler: Send + Sync {
    /// Định danh duy nhất của nền tảng (ví dụ: "codebuddy_cn", "codebuddy_global", "zed").
    fn platform_id(&self) -> &'static str;

    /// Tên hiển thị người dùng (ví dụ: "CodeBuddy (China)", "CodeBuddy (Global)").
    fn display_name(&self) -> &'static str;

    /// Khởi động phiên đăng nhập trình duyệt (lấy authUrl + state).
    fn start_login(&self, proxy_url: Option<&str>) -> Result<PlatformLoginStart, String>;

    /// Kiểm tra trạng thái xác thực từ máy chủ của hãng.
    fn poll_login(&self, state: &str, proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String>;

    /// Lấy thông tin tài khoản và quota trực tiếp.
    fn fetch_account(&self, state: &str, access_token: &str, proxy_url: Option<&str>) -> PlatformAccountInfo;
}
