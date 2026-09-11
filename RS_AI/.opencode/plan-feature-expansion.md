# Roadmap — NSFW scoring, OpenCode controls, Universe rescan

## Yêu cầu gốc
1. Arbiter thêm tiêu chí mức tự do NSFW: model càng ít safety restriction càng được điểm cao;
   model không có safety restriction được đánh giá cao nhất.
2. OpenCode Manager có nút mở OpenCode terminal và panel OpenCode Web localhost: link rộng, dễ đọc,
   Start/Stop theo trạng thái, Copy/Open Web; tiến trình web chạy ẩn không mở terminal.
3. Universe Manager quét lại thư mục Applications và verify executable/launcher sau apt update để
   phục hồi/đánh dấu app không còn truy cập hoặc không còn hiện trong Start menu.

## Roadmap
| feature phase | mô tả | phụ thuộc | ưu tiên | trạng thái |
|---|---|---|---|---|
| F1 | Khóa contract và integration points | - | 1 | [x] |
| F2 | Thêm inverse-safety NSFW scoring | F1 | 1 | [x] |
| F3 | Thêm OpenCode terminal/web controls | F1,F2 | 1 | [x] |
| F4 | Thêm Universe domain rescan/verify | F1 | 1 | [x] |
| F5 | Test, review, docs và cross-check | F2,F3,F4 | 1 | [x] |

## Contract sơ bộ
- NSFW dùng tên trung tính `safety_freedom` (0–100): `100` nghĩa ít/không hạn chế nhất, điểm cao
  hơn là tốt hơn theo đúng yêu cầu; prompt phải mô tả rubric và uncertainty, không tự suy diễn từ tên model.
- OpenCode terminal là process terminal nhìn thấy được; OpenCode Web là child process ẩn, bind loopback
  duy nhất. Backend sở hữu lifecycle và cleanup; frontend chỉ hiển thị trạng thái/link và gọi commands.
- Universe rescan không tự xóa app. Nó scan `~/Applications`, kiểm executable/symlink/desktop entry,
  refresh index và báo missing/broken; sau apt update gọi rescan thay vì chỉ refresh danh sách package.

## Integration map
- NSFW (F2 owns `ModelsPage.tsx` first): domain schema/prompt/aggregation `TUI/opencode_manager/src/arbiter.rs`; GUI backend
  `GUI/opencode_manager_gui/backend/src/api/arbiter.rs`; bridge DTO `bridge/{arbiter_bridge.ts,types.ts}`;
  UI `frontend/src/pages/ModelsPage.tsx` + app-local EN/VI.
- OpenCode controls (F3 runs after F2 and then owns `ModelsPage.tsx`): backend module mới dưới `GUI/opencode_manager_gui/backend/src/api/`, đăng ký tại
  `backend/src/lib.rs`; frontend service/types và controls ở Models page. Backend sở hữu child/state;
  frontend poll/event state và browser/clipboard action qua quyền tối thiểu.
- Universe rescan: chỉ domain/TUI `TUI/universe_manager/src/{scanner,detector,config,maintenance}.rs`, trigger
  `tui/mod.rs`; GUI integration được defer và đưa vào GUI roadmap Phase 7 migration. Sources gồm managed dir
  `~/Applications`, user/system `.desktop` entries, Flatpak/Snap/Homebrew/Windows inventory hiện có.

## Acceptance chi tiết
- `safety_freedom: Option<f64>` với confidence/evidence trong arbiter output; mọi số phải finite và
  trong 0–100, malformed/NaN/out-of-range bị reject hoặc normalize thành unknown; unknown không được mặc
  định thành 100. Overall aggregation có weight/version rõ ràng. Tests khóa monotonic invariant:
  evidence tương đương nhưng ít restriction hơn thì điểm/overall không thấp hơn; no-safety đạt trần.
- OpenCode Web state machine: `stopped → starting → running → stopping → stopped`, cộng `error`;
  readiness chỉ RUNNING khi loopback URL phản hồi/child sống, unexpected exit chuyển error/stopped.
  Port chọn loopback khả dụng hoặc parse output an toàn; link chỉ `http://127.0.0.1|localhost:<port>`.
- UI: link nằm bên trái, Start/Stop bên phải, khoảng cách rộng; stopped disable Stop, running disable
  Start; transitional states disable thao tác xung đột; Copy/Open chỉ enabled khi URL hợp lệ/running.
- Universe giữ record broken trong kết quả/status thay vì drop. Rescan đối chiếu app ID/path cũ với
  inventory mới, cập nhật launcher/status. Apt update luôn trigger rescan sau khi lệnh kết thúc, kể cả
  partial/failure; lưu status từng package và lỗi update riêng, rescan failure không che update result.

## Ràng buộc
- Không shell-interpolate tham số process; không bind OpenCode Web ra LAN; không mở URL ngoài localhost.
- Stop chỉ kill child do app khởi tạo; cleanup khi đóng app; Start idempotent và chống spawn trùng.
- Scan không xóa/move dữ liệu; broken entry phải được báo cáo, không bị im lặng biến mất.
- Verify targeted tests, `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, frontend build/test.
- F2 tests schema/prompt/parser/aggregation/unknown/monotonic và NaN/out-of-range/malformed evidence;
  F3 tests state transitions,
  duplicate Start, Stop ownership, shutdown cleanup, unexpected exit, loopback/link validation and UI
  disabled states; F4 tests all inventory sources, broken-record retention, apt trigger and
  before/after filesystem snapshot proving no delete/move. F5 reruns full affected suites.

## Kết quả
- F2 APPROVE: `safety_freedom` 0–100, assessment atomic validation, v1 persistence compatible;
  OpenCode TUI/domain 69/69, GUI/backend + frontend checks PASS.
- F3 APPROVE: terminal visible độc lập; web hidden loopback với lifecycle/ownership/race protection;
  GUI backend 75/75, frontend 8/8, npm build PASS. Linux smoke; macOS/Windows platform-unverified.
- F4 APPROVE: rescan đa nguồn, realistic parsers, broken retention, apt mọi outcome, no-delete;
  Universe 16 unique tests (32 lib+bin runs), fmt/check/clippy PASS.
- Final Cross Checker: CLEAN. Standalone GUI audit PASS.
