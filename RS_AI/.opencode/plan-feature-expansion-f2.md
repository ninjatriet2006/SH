# Feature Phase F2 — Inverse-safety NSFW scoring

| id | công việc | phụ thuộc | agent | trạng thái |
|---|---|---|---|---|
| F2.1 | Mở schema safety_freedom/confidence/evidence | F1 | rust-dev | [x] |
| F2.2 | Soạn rubric prompt inverse-safety/uncertainty | F2.1 | rust-dev | [x] |
| F2.3 | Parse, kiểm biên, chuẩn hóa unknown | F2.1,F2.2 | rust-dev | [x] |
| F2.4 | Aggregation weight/version monotonic | F2.3 | rust-dev | [x] |
| F2.5 | Ánh xạ GUI backend API | F2.4 | rust-dev | [x] |
| F2.6 | Cập nhật bridge DTO | F2.5 | rust-dev | [x] |
| F2.7 | Hiển thị trên ModelsPage | F2.6,F2.8 | rust-dev | [x] |
| F2.8 | Thêm EN/VI app-local | F2.6 | rust-dev | [x] |
| F2.9 | Test toàn bộ domain/aggregation | F2.1-F2.4 | tester | [x] |
| F2.10 | Test DTO và UI render/unknown | F2.5-F2.8 | tester | [x] |

Acceptance kế thừa `.opencode/plan-feature-expansion.md`; `100` là ít/không restriction nhất,
unknown không thành 100, mọi số finite trong 0–100 và overall có version/weight rõ ràng.
F2.9 khóa prompt rubric, missing/NaN/out-of-range/malformed confidence/evidence, unknown, exact
weight/version, monotonic và no-safety=100. F2.10 chỉ kiểm DTO serialization và UI known/unknown.

Kết quả: Reviewer APPROVE; Rust 127/127 và frontend 2/2 tests PASS; fmt/check/clippy `-D warnings`
và npm build PASS. Persisted v2 malformed chuẩn hóa toàn assessment thành unknown; v1 tương thích.
