/*
[INTEGRITY NOTES]
- Mục đích: API xử lý việc đọc lịch sử giao dịch (Transactions).
- Trách nhiệm: Đọc danh sách giao dịch từ DataStore, lọc theo user_id nếu cần.
- Tương tác: Được gọi từ frontend để hiển thị bảng lịch sử giao dịch.
*/

use crate::models::Transaction;
use crate::storage::{load_data, save_data};

// Lệnh Tauri lấy lịch sử giao dịch của một người dùng cụ thể
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_user_transactions(user_id: String) -> Result<Vec<Transaction>, String> {
    let data = load_data();
    // Lọc và sắp xếp mới nhất lên đầu
    let mut user_txs: Vec<Transaction> = data.transactions.into_iter().filter(|t| t.user_id == user_id).collect();
    user_txs.sort_by_key(|t| std::cmp::Reverse(t.created_at));
    Ok(user_txs)
}

// Lệnh Tauri lấy tất cả giao dịch (để làm báo cáo nếu cần)
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_all_transactions() -> Result<Vec<Transaction>, String> {
    let data = load_data();
    let mut all_txs = data.transactions;
    all_txs.sort_by_key(|t| std::cmp::Reverse(t.created_at));
    Ok(all_txs)
}

// Lệnh Tauri để xóa giao dịch theo ID
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn delete_transaction(id: String) -> Result<(), String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();
    let initial_len = data.transactions.len();
    
    data.transactions.retain(|t| t.id != id);
    
    if data.transactions.len() == initial_len {
        return Err(format!("Không tìm thấy giao dịch với ID: {}", id));
    }
    
    save_data(&data)?;
    Ok(())
}
