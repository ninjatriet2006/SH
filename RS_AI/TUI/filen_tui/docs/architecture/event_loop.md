# Event Loop (app/mod.rs)

## 1. Trách nhiệm (Responsibility)
Định tuyến và xử lý các sự kiện bất đồng bộ phát sinh từ bàn phím, tick timer (đếm nhịp) hoặc kết quả từ mạng, không để giao diện bị chặn.

## 2. Luồng dữ liệu (Data Flow)
1. **Source**: Có 3 nguồn phát sự kiện (`tx.send()`):
   - Luồng bàn phím (`crossterm::event::read()`).
   - Luồng tick timer (mỗi 100ms).
   - Tiến trình nền `tokio::spawn` (khi API trả về dữ liệu).
2. **Sink**: `rx.recv().await` tại khối vòng lặp `app.run()`.
3. **Dispatcher**: Phân loại theo enum `AppEvent` (ví dụ `RemoteLoadFinished`, `LoginFinished`) và cập nhật thẳng vào `App` state, sau đó gọi Terminal để vẽ lại giao diện.

## 3. Định hướng Refactor
- Logic match Event trong `run()` đang rất dài và lộn xộn.
- **Giải pháp**: Tách toàn bộ vòng lặp sự kiện ra một module `app/event_dispatcher.rs`, viết các hàm nhỏ lẻ cho từng nhánh (ví dụ: `handle_login_finished`, `handle_remote_load`).
