/*
[INTEGRITY NOTES]
- Mục đích: API xử lý việc đọc lịch sử giao dịch (Transactions).
- Trách nhiệm: Đọc danh sách giao dịch từ DataStore, lọc theo user_id nếu cần.
- Tương tác: Được gọi từ frontend để hiển thị bảng lịch sử giao dịch.
*/

use crate::models::Transaction;
use crate::storage::load_data;

// Lệnh Tauri lấy lịch sử giao dịch của một người dùng cụ thể
#[tauri::command]
pub fn list_user_transactions(user_id: String) -> Result<Vec<Transaction>, String> {
    let data = load_data();
    // Lọc và sắp xếp mới nhất lên đầu
    let mut user_txs: Vec<Transaction> = data.transactions.into_iter().filter(|t| t.user_id == user_id).collect();
    user_txs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(user_txs)
}

// Lệnh Tauri lấy tất cả giao dịch (để làm báo cáo nếu cần)
#[tauri::command]
pub fn list_all_transactions() -> Result<Vec<Transaction>, String> {
    let data = load_data();
    let mut all_txs = data.transactions;
    all_txs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(all_txs)
}

// Lệnh Tauri để xóa giao dịch theo ID
#[tauri::command]
pub fn delete_transaction(id: String) -> Result<(), String> {
    let mut data = load_data();
    let initial_len = data.transactions.len();
    
    data.transactions.retain(|t| t.id != id);
    
    if data.transactions.len() == initial_len {
        return Err(format!("Không tìm thấy giao dịch với ID: {}", id));
    }
    
    crate::storage::save_data(&data)?;
    Ok(())
}
