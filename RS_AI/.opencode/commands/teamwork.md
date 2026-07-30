---
description: Khởi chạy full teamwork multi-agent để giải quyết một task phức tạp. Team Lead sẽ điều phối Explorer, Rust Dev, Tester, Reviewer, Docs.
agent: lead
model: custom_3/moonshotai/kimi-k3-free
---

# Teamwork Multi-Agent Mode

Bạn đang ở chế độ Teamwork. Hãy thực hiện task sau theo quy trình multi-agent:

$ARGUMENTS

## Quy trình bắt buộc

1. **Tạo todo list** (dùng `todowrite`) chia nhỏ task thành các subtask rõ ràng.
2. **Giao Explorer** khảo sát codebase (nếu cần hiểu code hiện tại).
3. **Giao Rust Dev** thực hiện code change.
4. **Giao Tester** chạy test / verify build (`cargo check`, `cargo test`, `cargo clippy`).
5. **Giao Reviewer** review diff trước khi kết thúc.
6. **Giao Docs Manager** cập nhật tài liệu nếu có thay đổi đáng kể.
7. **Tổng hợp kết quả** và báo cáo cho user.

## Quy tắc
- Các subtask độc lập thì chạy song song.
- Subtask phụ thuộc thì chờ đợi.
- Luôn verify bằng `cargo check` / `cargo test` trước khi hoàn thành.
- Nếu task đơn giản (ví dụ chỉ 1 file 1 dòng), có thể tự xử lý nhưng vẫn phải tạo todo và verify.

Bắt đầu ngay bây giờ.
