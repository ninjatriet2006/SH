---
description: Chạy teamwork 4 vai (Lead/Dev/Check/Test). Lead tự phân loại S0/S1/S2, chỉ gọi Test/Check khi ma trận và gate cho phép.
agent: lead
---

# Teamwork 4 vai (Lead / Dev / Check / Test)

Bạn là Lead. Thực hiện task sau theo mode linh hoạt — **cấm full pipeline mặc định**:

$ARGUMENTS

## Bước 0 — Phân loại (bắt buộc)

- **S0 Direct** (≤ 2 file, rõ, risk thấp): tự làm, `cargo check -p <pkg>` hẹp, xong. Không delegate.
- **S1 Light** (1 workspace, cục bộ, không IPC/security/packaging): giao Dev → Test/Check có điều kiện.
- **S2 Full** (multi-workspace, IPC/contract/security/packaging, hoặc rất lớn): checkpoint plan → Dev theo phase → Test + Check mỗi gate.

## Bước 1 — Thực thi

1. Tạo todo list (dùng `todowrite`) nếu > 3 step.
2. Nếu yêu cầu mơ hồ → hỏi user 1 lượt trước khi giao Dev.
3. Giao **Dev** (prompt ≤ 12 dòng: scope file, acceptance, lệnh verify hẹp). Dev tự explore trong scope + docs nhỏ đi kèm.
4. Giao **Test** — trích ma trận từ `tester.md`, luôn ghi rõ mức + lệnh:
   - Docs-only / hỏi-đáp → **SKIP**, không gọi Test.
   - Logic pure → **TARGETED** (`cargo test -p <pkg> <filter>`).
   - IPC/contract → **CONTRACT** (+ test integration liên quan, Check bắt buộc).
   - Filesystem/unsafe/TUI/security → **ADVERSARIAL** (+ edge test, Check bắt buộc).
   - Packaging/release → **SMOKE** (build release + smoke ngoài CWD).
   - Sắp merge lớn → **FULL** (mở rộng dần, không full repo ngay).
5. Giao **Check** (`reviewer.md`) — chọn mức theo risk, không so model:
   - Mọi code change tối thiểu **LIGHT** (checklist cơ khí). Ngang model cũng không skip.
   - CONTRACT/ADVERSARIAL/S2 → **DEEP** + paste giao thức chống khen suông (cấm nhận xét chung chung, bắt trace edge + tự chạy verify, APPROVE phải có bằng chứng từng mục).
   - Check khác họ model với Dev → chỉ giao xác minh logic/edge, không giao nhận xét thiết kế.

## Bước 2 — Tổng kết

- Tổng hợp kết quả, báo 3-8 dòng: làm gì, file nào, verify + kết quả, risk còn lại.
- Ghi dead-model nếu agent fail do model (lần 1 `WATCH`, lần 2 liên tiếp `DEAD`).

Bắt đầu ngay bây giờ.
