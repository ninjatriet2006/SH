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
#[tauri::command]
pub fn add_package(name: String, duration_days: u32, description: Option<String>, price: Option<u64>) -> Result<Package, String> {
    // Tải dữ liệu toàn hệ thống
    let mut data = load_data();
    
    // Khởi tạo đối tượng Package mới
    let new_package = Package {
        id: generate_id("pkg"), // Gán ID tự động
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
#[tauri::command]
pub fn update_package(id: String, name: Option<String>, duration_days: Option<u32>, description: Option<String>, price: Option<u64>) -> Result<Package, String> {
    // Tải dữ liệu hệ thống
    let mut data = load_data();
    
    // Tìm kiếm gói dịch vụ theo ID
    if let Some(pkg) = data.packages.iter_mut().find(|p| p.id == id) {
        // Cập nhật tên nếu có truyền vào
        if let Some(n) = name {
            pkg.name = n;
        }
        // Cập nhật thời hạn ngày nếu có
        if let Some(d) = duration_days {
            pkg.duration_days = d;
        }
        // Cập nhật mô tả nếu có
        if let Some(desc) = description {
            pkg.description = Some(desc);
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
#[tauri::command]
pub fn delete_package(id: String) -> Result<(), String> {
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
    
    // Lưu lại dữ liệu sau khi xóa
    save_data(&data)?;
    
    // Xóa thành công
    Ok(())
}

// Lệnh Tauri để lấy danh sách toàn bộ các gói dịch vụ (có phân trang)
#[tauri::command]
pub fn list_packages(page: Option<u32>, limit: Option<u32>) -> Result<Vec<Package>, String> {
    // Tải danh sách từ storage
    let data = load_data();
    
    // Kiểm tra và thực hiện phân trang nếu có đủ 2 tham số page và limit
    if let (Some(p), Some(l)) = (page, limit) {
        // Vị trí bắt đầu cắt mảng
        let start = (p * l) as usize;
        // Thực hiện skip và take để lấy mảng con
        let paged_pkgs = data.packages.into_iter().skip(start).take(l as usize).collect();
        return Ok(paged_pkgs);
    }
    
    // Nếu không phân trang, trả về tất cả
    Ok(data.packages)
}
