---
description: Review các thay đổi hiện tại (git diff) bởi Reviewer agent.
agent: lead
---

# Teamwork Review Command

Hãy điều phối Reviewer agent để review các thay đổi hiện tại trong repo.

$ARGUMENTS

Nếu có staged/unstaged changes, review diff. Nếu không có changes, báo cáo rằng không có gì để review.

Review chéo đối ứng (`lead.md` mục 6): Check mặc định chạy SeekAI GLM flash
(`reviewer.md`). Nếu diff hiện tại do GLM tạo ra (phase lẻ / Dev = GLM), Lead đảo
2 dòng `model:` trong `rust-dev.md` ↔ `reviewer.md` trước để Muse Spark duyệt —
cấm cùng model vừa làm vừa duyệt.

Reviewer phải xuất kết quả rõ ràng: APPROVED hoặc REQUEST_CHANGES với danh sách cụ thể.
