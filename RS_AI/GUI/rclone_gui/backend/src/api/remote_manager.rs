/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints quản lý Remote (giữ nguyên tên IPC/api cũ).
- Trách nhiệm: Tầng API mỏng — nhận request từ Frontend rồi chuyển cho
  `actions::{remote_view,remote_edit}` qua `logic::fastlane::fastlane`. Không chạy rclone trực tiếp.
- Tương tác: Được gọi từ frontend qua Tauri command (`list_remotes`, `create_remote`...).
*/
// UNIVERSAL: thợ chạy ở `actions::{remote_view,remote_edit,checkfeature,checksize}::*_inner` (đồng bộ); cửa chỉ bọc `fastlane`.

use crate::actions::{checkfeature, checksize, remote_edit, remote_view};
use crate::logic::fastlane::fastlane;
use serde_json::Value;
use std::collections::HashMap;

/// Tên hàm: list_remotes
/// Mô tả: Trả về danh sách tất cả các remote đã cấu hình từ rclone config dump.
pub async fn list_remotes() -> Result<Vec<Value>, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::remote_view::list_remotes_inner`.
    fastlane(remote_view::list_remotes_inner).await
}

/// Tên hàm: get_providers
/// Mô tả: Trả về danh sách tất cả các loại cloud (Google Drive, Dropbox...) được rclone hỗ trợ dưới dạng JSON.
pub async fn get_providers() -> Result<String, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::remote_view::get_providers_inner`.
    fastlane(remote_view::get_providers_inner).await
}

/// Tên hàm: create_remote
/// Mô tả: Tạo mới một cấu hình cloud (remote). Nhận vào tên, loại (provider) và các tùy chọn bổ sung (options).
pub async fn create_remote(
    name: String,
    provider: String,
    options: HashMap<String, String>,
) -> Result<String, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::remote_edit::create_remote_inner`.
    fastlane(move || remote_edit::create_remote_inner(&name, &provider, &options)).await
}

/// Tên hàm: delete_remote
/// Mô tả: Xóa một cấu hình cloud (remote) khỏi rclone.
pub async fn delete_remote(name: String) -> Result<String, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::remote_edit::delete_remote_inner`.
    fastlane(move || remote_edit::delete_remote_inner(&name)).await
}

/// Tên hàm: update_remote
/// Mô tả: Cập nhật các thông số của một cấu hình cloud hiện có.
pub async fn update_remote(name: String, options: HashMap<String, String>) -> Result<String, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::remote_edit::update_remote_inner`.
    fastlane(move || remote_edit::update_remote_inner(&name, &options)).await
}

/// Tên hàm: get_backend_features
/// Mô tả: Kiểm tra xem ổ đĩa cloud này có hỗ trợ tính năng nào (vd: Thùng rác, copy server-side...).
pub async fn get_backend_features(remote: String) -> Result<Value, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::checkfeature::query_backend_features`.
    fastlane(move || checkfeature::query_backend_features(&remote)).await
}

/// Tên hàm: check_transfer_capability
/// Mô tả: Đánh giá khả năng copy/move giữa 2 đường dẫn thông qua rclone features.
pub async fn check_transfer_capability(src: String, dst: String) -> Result<Value, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::checkfeature::query_transfer_options` (não chung `move_op::check_cap`).
    fastlane(move || checkfeature::query_transfer_options(&src, &dst)).await
}

/// Tên hàm: rclone_about
/// Mô tả: Lấy thông tin dung lượng của một remote (total, used, free, trashed, other).
pub async fn rclone_about(remote: String) -> Result<Value, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::checksize::about_inner`.
    fastlane(move || checksize::about_inner(&remote)).await
}

/// Tên hàm: rclone_size
/// Mô tả: Lấy thông tin kích thước và số lượng tệp của một remote hoặc thư mục.
pub async fn rclone_size(remote: String) -> Result<Value, String> {
    // UNIVERSAL: wrapper mỏng — logic chạy ở `actions::checksize::size_inner`.
    fastlane(move || checksize::size_inner(&remote)).await
}
