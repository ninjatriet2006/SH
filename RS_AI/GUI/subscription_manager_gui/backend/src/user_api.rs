/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các nghiệp vụ (CRUD) liên quan đến Người dùng (User).
- Trách nhiệm: Cung cấp các lệnh (commands) Tauri để Frontend có thể gọi (add_user, update_user, delete_user, list_users).
- Tương tác: Giao tiếp với module `storage` để đọc/ghi dữ liệu danh sách người dùng và module `models` để định nghĩa kiểu dữ liệu trả về.
*/

// Import các Struct từ module models
use crate::models::User;
// Import các hàm đọc/ghi dữ liệu từ module storage
use crate::storage::{load_data, save_data};
// Import thư viện quản lý thời gian của Rust để sinh ID và timestamp
use crate::utils::{current_timestamp, generate_id};



// Lệnh Tauri để thêm một người dùng mới
#[tauri::command]
pub fn add_user(username: String, email: Option<String>, phone: Option<String>, contact_url: Option<String>) -> Result<User, String> {
    // Tải toàn bộ dữ liệu từ file storage
    let mut data = load_data();
    
    // Khởi tạo một đối tượng User mới với ID và timestamp tự động
    let new_user = User {
        id: generate_id("usr"),           // Gán ID duy nhất
        username,                    // Gán tên người dùng
        email,                       // Gán email (có thể có hoặc không)
        phone,
        contact_url,
        created_at: current_timestamp(), // Gán thời gian tạo
    };
    
    // Thêm người dùng vừa tạo vào danh sách
    data.users.push(new_user.clone());
    
    // Lưu lại dữ liệu xuống ổ đĩa
    save_data(&data)?;
    
    // Trả về kết quả là người dùng vừa tạo để frontend cập nhật UI
    Ok(new_user)
}

// Lệnh Tauri để cập nhật thông tin người dùng
#[tauri::command]
pub fn update_user(id: String, username: Option<String>, email: Option<String>, phone: Option<String>, contact_url: Option<String>) -> Result<User, String> {
    // Tải toàn bộ dữ liệu hiện có
    let mut data = load_data();
    
    // Tìm kiếm người dùng trong danh sách bằng ID
    if let Some(user) = data.users.iter_mut().find(|u| u.id == id) {
        // Nếu client có truyền username mới, thì cập nhật lại username
        if let Some(new_username) = username {
            user.username = new_username;
        }
        // Nếu client có truyền email mới, thì cập nhật lại email
        if let Some(new_email) = email {
            user.email = Some(new_email);
        }
        if let Some(new_phone) = phone {
            user.phone = Some(new_phone);
        }
        if let Some(new_contact_url) = contact_url {
            user.contact_url = Some(new_contact_url);
        }
        
        // Clone dữ liệu người dùng sau khi đã cập nhật để trả về
        let updated_user = user.clone();
        
        // Ghi toàn bộ dữ liệu mới xuống file
        save_data(&data)?;
        
        // Trả về người dùng đã cập nhật thành công
        return Ok(updated_user);
    }
    
    // Nếu vòng lặp tìm kiếm kết thúc mà không thấy ID, trả về lỗi
    Err(format!("Không tìm thấy người dùng với ID: {}", id))
}

// Lệnh Tauri để xóa một người dùng
#[tauri::command]
pub fn delete_user(id: String) -> Result<(), String> {
    // Lấy dữ liệu hiện tại
    let mut data = load_data();
    
    // Lấy số lượng người dùng ban đầu
    let initial_len = data.users.len();
    
    // Lọc và giữ lại những người dùng có ID khác với ID cần xóa
    data.users.retain(|u| u.id != id);
    
    // Nếu số lượng sau khi lọc vẫn bằng ban đầu, tức là không tìm thấy user cần xóa
    if data.users.len() == initial_len {
        return Err(format!("Không tìm thấy người dùng với ID: {}", id));
    }
    
    // Lưu dữ liệu sau khi xóa thành công
    save_data(&data)?;
    
    // Trả về kết quả rỗng (tương đương với thành công)
    Ok(())
}

// Lệnh Tauri để lấy danh sách người dùng (hỗ trợ phân trang cơ bản)
#[tauri::command]
pub fn list_users(page: Option<u32>, limit: Option<u32>) -> Result<Vec<User>, String> {
    // Tải toàn bộ dữ liệu từ storage
    let data = load_data();
    
    // Nếu người dùng yêu cầu phân trang, chúng ta tính toán điểm bắt đầu và kết thúc
    if let (Some(p), Some(l)) = (page, limit) {
        // Tính vị trí phần tử bắt đầu
        let start = (p * l) as usize;
        // Lấy danh sách cắt từ vị trí start
        let paged_users = data.users.into_iter().skip(start).take(l as usize).collect();
        // Trả về mảng đã cắt
        return Ok(paged_users);
    }
    
    // Nếu không truyền tham số phân trang, trả về toàn bộ danh sách
    Ok(data.users)
}
