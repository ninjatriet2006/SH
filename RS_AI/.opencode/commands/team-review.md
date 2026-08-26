---
description: Review các thay đổi hiện tại (git diff) bởi Reviewer agent.
agent: lead
model: opencode/x-preview-f-free
---

# Teamwork Review Command

Hãy điều phối Reviewer agent để review các thay đổi hiện tại trong repo.

$ARGUMENTS

Nếu có staged/unstaged changes, review diff. Nếu không có changes, báo cáo rằng không có gì để review.

Reviewer phải xuất kết quả rõ ràng: APPROVED hoặc REQUEST_CHANGES với danh sách cụ thể.
