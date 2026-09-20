---
description: Review chéo các thay đổi hiện tại (git diff) theo hệ 2-model.
agent: lead
---

# Teamwork Review Command

Điều phối review chéo 2 chiều cho các thay đổi hiện tại trong repo.

$ARGUMENTS

Nếu có staged/unstaged changes, review diff. Nếu không có changes, báo cáo rằng không có gì để review.

Review chéo 2 chiều (`lead.md` mục 3 + 6): **model nào viết code thì model KIA review** — cấm tự approve.
- Diff do **Lead** viết → giao **Peer** (`peer.md`, mũ Review) soi.
- Diff do **Peer** viết → **Lead** tự review inline (đọc diff, trace edge, chạy verify).

Bên review phải xuất kết quả rõ ràng: APPROVE / REQUEST_CHANGES / FOUND_ISSUES, mỗi finding kèm `file:line — impact — fix`. Cấm khen suông.
