/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các nghiệp vụ gán và quản lý Subscription (Đăng ký gói) cho User.
- Trách nhiệm: Tính toán ngày hết hạn, tạo mới Subscription, cập nhật thời gian và kiểm tra trạng thái kích hoạt.
- Tương tác: Gọi `storage` để lấy và lưu dữ liệu. Cần đọc `packages` để tính toán `duration_days` nếu không truyền `custom_expiration_date`.
*/

// Import struct Subscription và Transaction từ module models
use crate::models::{Subscription, Transaction};
// Import hàm đọc/ghi file lưu trữ
use crate::storage::{load_data, save_data};
use crate::utils::{current_timestamp, generate_id};

// Lệnh Tauri để gán một gói dịch vụ cho một người dùng
#[tauri::command]
pub fn add_subscription_to_user(
    user_id: String,
    package_id: String,
    custom_expiration_date: Option<i64>,
    amount: Option<u64>,
) -> Result<Subscription, String> {
    // Tải toàn bộ cơ sở dữ liệu
    let mut data = load_data();
    
    // Kiểm tra xem user có tồn tại không
    if !data.users.iter().any(|u| u.id == *user_id) {
        return Err(format!("Người dùng không tồn tại: {}", user_id));
    }
    
    // Tìm gói dịch vụ theo ID
    let package = match data.packages.iter().find(|p| p.id == *package_id) {
        Some(pkg) => pkg.clone(), // Copy dữ liệu gói
        None => return Err(format!("Gói dịch vụ không tồn tại: {}", package_id)),
    };
    
    // Tính toán ngày hết hạn (expiration_date)
    let expiration_date = match custom_expiration_date {
        // Nếu admin truyền vào ngày tùy chỉnh, thì dùng ngày đó
        Some(custom_date) => custom_date,
        // Nếu không có, tự động tính bằng cách cộng số ngày của gói vào thời gian hiện tại
        None => {
            // 1 ngày = 86400 giây = 86_400_000 mili-giây
            let ms_in_day: i64 = 86_400_000;
            // Tính hạn sử dụng
            current_timestamp() + (package.duration_days as i64 * ms_in_day)
        }
    };
    
    // Tính trạng thái is_active
    let is_active = expiration_date > current_timestamp();
    
    // Khởi tạo đối tượng Subscription
    let new_subscription = Subscription {
        id: generate_id("sub"), // ID ngẫu nhiên tự tạo
        user_id: user_id.clone(),               // ID người dùng
        package_id: package_id.clone(),            // ID gói dịch vụ
        expiration_date,       // Ngày hết hạn đã tính toán
        is_active,             // Bật/tắt tùy theo thời hạn
    };
    
    // Khởi tạo đối tượng Transaction log
    let new_tx = Transaction {
        id: generate_id("tx"),
        user_id: user_id.clone(),
        package_id: package_id.clone(),
        amount: amount.unwrap_or(0),
        action: "ASSIGN".to_string(),
        created_at: current_timestamp(),
    };
    
    // Thêm vào danh sách subscriptions và transactions
    data.subscriptions.push(new_subscription.clone());
    data.transactions.push(new_tx);
    
    // Ghi dữ liệu
    save_data(&data)?;
    
    // Trả về gói đăng ký vừa tạo
    Ok(new_subscription)
}

// Lệnh Tauri để thay đổi ngày hết hạn của một subscription
#[tauri::command]
pub fn update_subscription_expiry(
    subscription_id: String,
    new_expiration_date: i64,
    amount: Option<u64>,
) -> Result<Subscription, String> {
    // Tải dữ liệu
    let mut data = load_data();
    
    // Tìm kiếm đăng ký theo ID
    if let Some(sub) = data.subscriptions.iter_mut().find(|s| s.id == subscription_id) {
        // Đặt lại ngày hết hạn
        sub.expiration_date = new_expiration_date;
        
        // Nếu hạn sử dụng mới lớn hơn thời điểm hiện tại, có thể kích hoạt lại gói
        if new_expiration_date > current_timestamp() {
            sub.is_active = true;
        } else {
            // Ngược lại, nếu set ngày quá khứ, thì vô hiệu hóa
            sub.is_active = false;
        }
        
        // Ghi log giao dịch gia hạn
        let new_tx = Transaction {
            id: generate_id("tx"),
            user_id: sub.user_id.clone(),
            package_id: sub.package_id.clone(),
            amount: amount.unwrap_or(0),
            action: "RENEW".to_string(),
            created_at: current_timestamp(),
        };
        data.transactions.push(new_tx);
        
        // Tạo bản copy để trả về
        let updated_sub = sub.clone();
        
        // Lưu dữ liệu
        save_data(&data)?;
        
        // Trả về thành công
        return Ok(updated_sub);
    }
    
    // Báo lỗi nếu không tìm thấy ID
    Err(format!("Không tìm thấy gói đăng ký: {}", subscription_id))
}

// Lệnh Tauri để gỡ bỏ / xóa một gói đăng ký khỏi người dùng
#[tauri::command]
pub fn remove_subscription_from_user(subscription_id: String) -> Result<(), String> {
    // Lấy dữ liệu hiện tại
    let mut data = load_data();
    // Lọc mảng, bỏ qua ID cần xóa
    let initial_len = data.subscriptions.len();
    data.subscriptions.retain(|s| s.id != subscription_id);
    
    // Kiểm tra số lượng
    if data.subscriptions.len() == initial_len {
        return Err(format!("Không tìm thấy gói đăng ký để xóa: {}", subscription_id));
    }
    
    // Lưu lại
    save_data(&data)?;
    // Thành công
    Ok(())
}

// Lệnh Tauri để liệt kê các đăng ký của một người dùng cụ thể
#[tauri::command]
pub fn list_user_subscriptions(user_id: String) -> Result<Vec<Subscription>, String> {
    // Lấy dữ liệu
    let mut data = load_data();
    let now = current_timestamp();
    let mut changed = false;
    
    // Tự động quét và cập nhật trạng thái
    for sub in data.subscriptions.iter_mut() {
        let should_be_active = sub.expiration_date > now;
        if sub.is_active != should_be_active {
            sub.is_active = should_be_active;
            changed = true;
        }
    }
    
    // Lưu lại nếu có thay đổi
    if changed {
        let _ = save_data(&data);
    }

    // Lọc ra các subscription mà trường user_id trùng khớp
    let user_subs: Vec<Subscription> = data.subscriptions.into_iter().filter(|s| s.user_id == user_id).collect();
    
    // Trả về danh sách
    Ok(user_subs)
}

// Lệnh Tauri để kiểm tra xem một đăng ký còn hạn hay không
#[tauri::command]
pub fn check_subscription_status(subscription_id: String) -> Result<bool, String> {
    // Tải dữ liệu
    let mut data = load_data();
    
    // Tìm kiếm đăng ký
    if let Some(sub) = data.subscriptions.iter_mut().find(|s| s.id == subscription_id) {
        let now = current_timestamp();
        // Nếu ngày hết hạn bé hơn hoặc bằng hiện tại, gói đã hết hạn
        if sub.expiration_date <= now {
            sub.is_active = false;
        } else {
            sub.is_active = true;
        }
        
        // Trạng thái (để trả về hàm)
        let active_status = sub.is_active;
        
        // Lưu lại trạng thái kích hoạt nếu có thay đổi
        save_data(&data)?;
        
        // Trả về boolean true/false
        return Ok(active_status);
    }
    
    Err(format!("Không tìm thấy đăng ký: {}", subscription_id))
}

// Lệnh Tauri để liệt kê toàn bộ gói đăng ký (hỗ trợ hiển thị cảnh báo cho tất cả người dùng)
#[tauri::command]
pub fn list_all_subscriptions() -> Result<Vec<Subscription>, String> {
    let mut data = load_data();
    let now = current_timestamp();
    let mut changed = false;
    
    // Tự động quét và cập nhật trạng thái
    for sub in data.subscriptions.iter_mut() {
        let should_be_active = sub.expiration_date > now;
        if sub.is_active != should_be_active {
            sub.is_active = should_be_active;
            changed = true;
        }
    }
    
    // Lưu lại nếu có thay đổi
    if changed {
        let _ = save_data(&data);
    }
    
    Ok(data.subscriptions)
}
