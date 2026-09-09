/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các nghiệp vụ quản lý Gói dịch vụ (Package).
- Trách nhiệm: Định nghĩa các lệnh (commands) Tauri cho Frontend gọi tới (thêm, sửa, xóa, danh sách gói dịch vụ).
- Tương tác: Đọc và ghi dữ liệu gói dịch vụ thông qua module `storage` và sử dụng struct `Package` từ module `models`.
*/

// Import struct Package từ module models
use crate::models::Package;
// Import các hàm load/save từ module storage
use crate::storage::{load_data, save_data};
use crate::utils::generate_id;

// Lệnh Tauri để tạo một gói dịch vụ mới
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn add_package(
    name: String,
    duration_days: u32,
    description: Option<String>,
    price: Option<u64>,
) -> Result<Package, String> {
    if name.trim().is_empty() {
        return Err("Tên gói dịch vụ không được để trống".to_string());
    }
    if duration_days == 0 {
        return Err("Thời hạn gói phải lớn hơn 0 ngày".to_string());
    }
    let _store_guard = crate::storage::lock_store();
    // Tải dữ liệu toàn hệ thống
    let mut data = load_data();

    // Khởi tạo đối tượng Package mới
    let new_package = Package {
        id: generate_id("pkg"),    // Gán ID tự động
        name,                      // Tên gói (ví dụ: "Premium")
        description,               // Mô tả về gói
        duration_days,             // Thời hạn sử dụng gói (tính bằng ngày)
        price: price.unwrap_or(0), // Giá tiền (mặc định 0 nếu không truyền)
    };

    // Thêm gói mới vào danh sách packages
    data.packages.push(new_package.clone());

    // Ghi lưu dữ liệu vào file JSON
    save_data(&data)?;

    // Trả về gói dịch vụ vừa tạo thành công
    Ok(new_package)
}

// Lệnh Tauri để cập nhật thông tin của một gói dịch vụ hiện có
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn update_package(
    id: String,
    name: Option<String>,
    duration_days: Option<u32>,
    description: Option<String>,
    price: Option<u64>,
) -> Result<Package, String> {
    let _store_guard = crate::storage::lock_store();
    // Tải dữ liệu hệ thống
    let mut data = load_data();

    // Tìm kiếm gói dịch vụ theo ID
    if let Some(pkg) = data.packages.iter_mut().find(|p| p.id == id) {
        // Cập nhật tên nếu có truyền vào
        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err("Tên gói dịch vụ không được để trống".to_string());
            }
            pkg.name = n;
        }
        // Cập nhật thời hạn ngày nếu có
        if let Some(d) = duration_days {
            if d == 0 {
                return Err("Thời hạn gói phải lớn hơn 0 ngày".to_string());
            }
            pkg.duration_days = d;
        }
        // Cập nhật mô tả nếu có (`Some("")` = xóa về None, `None` = giữ nguyên).
        if let Some(desc) = description {
            pkg.description = if desc.is_empty() { None } else { Some(desc) };
        }
        // Cập nhật giá tiền nếu có
        if let Some(pr) = price {
            pkg.price = pr;
        }

        // Lưu lại bản sao của gói đã được cập nhật
        let updated_pkg = pkg.clone();

        // Ghi thay đổi xuống ổ đĩa
        save_data(&data)?;

        // Trả về gói đã sửa
        return Ok(updated_pkg);
    }

    // Báo lỗi nếu không tìm thấy ID gói
    Err(format!("Không tìm thấy gói dịch vụ với ID: {}", id))
}

// Lệnh Tauri để xóa gói dịch vụ
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn delete_package(id: String) -> Result<(), String> {
    let _store_guard = crate::storage::lock_store();
    // Tải dữ liệu hệ thống
    let mut data = load_data();

    // Ghi nhớ số lượng gói ban đầu
    let initial_len = data.packages.len();

    // Loại bỏ gói dịch vụ trùng ID khỏi mảng
    data.packages.retain(|p| p.id != id);

    // Nếu số lượng không đổi, nghĩa là không tìm thấy gói để xóa
    if data.packages.len() == initial_len {
        return Err(format!("Không tìm thấy gói dịch vụ với ID: {}", id));
    }

    // Chặn xóa khi còn subscription tham chiếu — trước đây để lại package_id treo.
    if data.subscriptions.iter().any(|s| s.package_id == id) {
        return Err("Không thể xóa: vẫn còn đăng ký đang dùng gói này".to_string());
    }

    // Lưu lại dữ liệu sau khi xóa
    save_data(&data)?;

    // Xóa thành công
    Ok(())
}

// Lệnh Tauri để lấy danh sách toàn bộ các gói dịch vụ (có phân trang)
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_packages(page: Option<u32>, limit: Option<u32>) -> Result<Vec<Package>, String> {
    // Tải danh sách từ storage
    let data = load_data();

    // Như list_users: thiếu một phía là lỗi gọi sai, không im lặng trả full.
    if page.is_some() ^ limit.is_some() {
        return Err("Phân trang cần cả `page` và `limit`".to_string());
    }

    // Kiểm tra và thực hiện phân trang nếu có đủ 2 tham số page và limit
    if let (Some(p), Some(l)) = (page, limit) {
        // Vị trí bắt đầu cắt mảng (saturating chống tràn u32 ở bản debug)
        let start = (p as usize).saturating_mul(l as usize);
        // Thực hiện skip và take để lấy mảng con
        let paged_pkgs = data.packages.into_iter().skip(start).take(l as usize).collect();
        return Ok(paged_pkgs);
    }

    // Nếu không phân trang, trả về tất cả
    Ok(data.packages)
}
