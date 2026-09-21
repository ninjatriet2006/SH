/*
[INTEGRITY NOTES]
- Mục đích: Tầng Logic - Tiện ích chạy tác vụ blocking ngoài async runtime.
- Trách nhiệm: Đưa các lời gọi chặn luồng (spawn tiến trình rclone, đọc/ghi file)
  sang thread pool riêng để không chặn async runtime của Tauri.
- Tương tác: Dùng bởi các `#[tauri::command]` async trong tầng `api/`.

Vì sao cần: một `#[tauri::command] async fn` chạy trên async runtime. Nếu thân hàm
gọi `std::process::Command::output()` (chặn cho tới khi tiến trình kết thúc) thì nó
giữ luôn worker thread của runtime. Với remote cloud chậm, `rclone about` có thể mất
vài giây — đủ để làm treo các lời gọi IPC khác đang chờ.
*/

// UNIVERSAL: việc hỏi-nhanh (đọc/ghi nhỏ, gọi rclone ngắn) đi thẳng qua đây;
// việc làm-lâu (copy/move/delete/list lớn) đi đường jobs (`logic::jobs`).

/// Tên hàm: fastlane
/// Mô tả: Chạy closure chặn luồng trên thread pool riêng rồi trả kết quả về async.
/// Phẳng hoá luôn `JoinError` thành `String` để dùng trực tiếp trong Tauri command.
pub async fn fastlane<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Lỗi thực thi tác vụ nền: {}", e))?
}
