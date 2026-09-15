# Feature: Fingerprint Pool & Unified Session Rotation (Đồng bộ danh tính)

## Hiện trạng & Phân tích từ User
1. **Tính cứng nhắc của Fingerprint hiện tại:** Các ô tick hiện nay chỉ mang tính chất "Cắt bỏ" (Strip) một cách tiêu cực. Nếu Cloudflare hoặc máy chủ API yêu cầu bắt buộc phải có header `x-stainless-os` mới cho qua (để chống bot), thì việc xóa nó đi lại tác dụng ngược.
2. **Định danh rời rạc (Identity Leakage):** Hiện tại hệ thống xoay Key (Key Rotation) hoạt động độc lập. Nếu Key A bị block (403/401), Proxy lấy Key B ra xài tiếp. NHƯNG nếu lúc đó Proxy VẪN DÙNG chung địa chỉ IP cũ (VPN cũ) và chung một Fingerprint cũ, máy chủ API sẽ dễ dàng nhận ra "À, thằng xài Key B này vẫn chính là cái thằng vừa xài Key A", và nó sẽ block nốt Key B.

Giải pháp mà bạn đề xuất là một kiến trúc đỉnh cao trong lĩnh vực Anti-Detect (Chống dò thám): **Đồng bộ vòng đời (Unified Session Rotation)**. Khi một yếu tố "chết", toàn bộ "vỏ bọc" phải thay mới!

## Nhiệm vụ (Task List)
- [ ] **Nâng cấp Fingerprint thành Dạng Pool (Hồ chứa Profile):**
  - Đập bỏ giao diện tick tĩnh hiện tại. Thay bằng bảng danh sách "Fingerprint Profiles".
  - Mỗi Profile cho phép người dùng tự cấu hình `User-Agent` và một bộ cấu hình `Custom Headers Spoofing` (Ví dụ điền: `x-stainless-os: Mac`, `machine-id: abc-123`).
  - Logic: Thay vì chỉ xóa (Strip), nếu người dùng có điền giá trị Spoof, Proxy sẽ **GHI ĐÈ (Overwrite)** giá trị thật bằng giá trị giả. Nếu người dùng để trống, Proxy mới dùng lệnh xóa (Strip).
- [ ] **Xây dựng Thuật toán Unified Session Rotation (Đồng bộ Xoay Danh Tính):**
  - Sửa đổi cơ chế tại `backend/src/proxy/key_manager.rs` và `vpn/mod.rs`.
  - Khai báo một khái niệm `Current_Session`. Một Session bao gồm tổ hợp: `[1 Key + 1 VPN Tunnel + 1 Fingerprint Profile]`.
  - **Sự kiện Kích hoạt (Trigger):** Nếu Key báo lỗi 403/401 -> Key Manager vứt Key đó đi -> Báo tín hiệu (Event) cho Session Manager.
  - **Hành động:** Session Manager lập tức quay vòng: Rút một Key mới + Đổi sang VPN Tunnel mới (nếu VPN đang bật) + Đổi sang Fingerprint Profile mới.
  - Kết quả: Request tiếp theo bắn lên sẽ là một con người hoàn toàn khác (IP khác, Máy tính khác, Key khác), ngắt đứt hoàn toàn dấu vết với Key cũ bị ban.

## ⚠️ Yêu cầu bắt buộc: Kế hoạch AI tự kiểm chứng độc lập (Self-Verification)
1. **Kiểm chứng Spoofing (Ghi đè):** Truyền header `cursor-version: 1.0` vào. Cấu hình Profile giả mạo thành `cursor-version: 9.9`. Đảm bảo Proxy bắn ra ngoài `9.9` chứ không phải xóa trắng nó.
2. **Kiểm chứng Sync Rotation:** Bắn request giả lập lỗi 401. Log phải ghi nhận đồng thời 3 hành động: Đổi Key, Chuyển Tunnel, và Đổi Fingerprint Profile thành công trước khi gửi Request tiếp theo.
3. **Ghi nhận:** Báo cáo chi tiết vào cuối file.
