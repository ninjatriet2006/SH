---
description: Khởi chạy full teamwork multi-agent để giải quyết một task phức tạp. Team Lead sẽ điều phối Plan Splitter, Plan Reviewer, Explorer, Rust Dev, Tester, Reviewer, Docs.
agent: lead
model: opencode/x-preview-f-free
---

# Teamwork Multi-Agent Mode

Bạn đang ở chế độ Teamwork. Hãy thực hiện task sau theo quy trình multi-agent:

$ARGUMENTS

## Quy trình bắt buộc

### Phase 1 — Lập kế hoạch
1. **Tạo todo list** (dùng `todowrite`).
2. **Giao Plan Splitter** phân rã task thành subtask.
3. **Giao Plan Reviewer** phản biện bản phân rã. Nếu REQUEST_CHANGES → quay lại bước 2.

### Phase 2 — Thực thi
4. **Giao Explorer** khảo sát codebase (nếu cần hiểu code hiện tại).
5. **Giao Rust Dev** thực hiện code change.
6. **Giao Tester** chạy test / verify build (`cargo check`, `cargo test`, `cargo clippy`).
7. **Giao Reviewer** review diff trước khi kết thúc.
8. **Giao Docs Manager** cập nhật tài liệu nếu có thay đổi đáng kể.

### Phase 3 — Tổng kết
9. **Tổng hợp kết quả** và báo cáo cho user.

## Quy tắc
- Các subtask độc lập thì chạy song song.
- Subtask phụ thuộc thì chờ đợi.
- Luôn verify bằng `cargo check` / `cargo test` trước khi hoàn thành.
- Nếu task đơn giản (ví dụ chỉ 1 file 1 dòng), có thể tự xử lý nhưng vẫn phải tạo todo và verify.

Bắt đầu ngay bây giờ.
