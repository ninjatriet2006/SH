# sys/mod.rs
Tài liệu tham chiếu cấu trúc Abstraction Layer (Tầng trừu tượng).

- **Mô tả**: Đây là file định tuyến module dựa trên hệ điều hành đang chạy (OS-specific abstraction). Nó sẽ quyết định biên dịch các module tương ứng (`unix.rs` hoặc `windows.rs`) và đưa (expose) tất cả các hàm ra ngoài (`pub use ...`).
- **Logic hoạt động**: 
  - Nếu biên dịch trên môi trường Unix (`cfg(unix)`): load và expose `unix`, `desktop_apps`, `custom_actions`, `doc_search`.
  - Nếu biên dịch trên môi trường Windows (`cfg(windows)`): load và expose module `windows`.
