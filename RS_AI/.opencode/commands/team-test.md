---
description: Chạy full test & lint cho toàn bộ repo hoặc workspace cụ thể.
agent: lead
model: custom_3/moonshotai/kimi-k3-free
---

# Teamwork Test Command

Hãy điều phối Tester agent để chạy kiểm thử cho:

$ARGUMENTS

Nếu không có workspace cụ thể, chạy cho tất cả workspace chính: `universe_manager`, `filen_tui`, `IMG_SPLT.rs`, `opencode_manager`, `universal_converter`.

Báo cáo kết quả chi tiết: PASS/FAIL, số test, clippy warnings, và log lỗi nếu có.
