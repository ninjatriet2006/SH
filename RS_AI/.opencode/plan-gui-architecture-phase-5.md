# Phase 5 — Migrate `universal_converter_gui`

## Mục tiêu
Migrate app egui hiện hữu sang Tauri v2 ba lớp theo contract Phase 2, giữ parity/persistence/native
behavior, tự sở hữu assets và chứng minh rollback trong worktree tạm.

| id | công việc | deliverable / acceptance | phụ thuộc | agent | trạng thái |
|---|---|---|---|---|---|
| 5.1 | Chốt baseline và checkpoint | Baseline build/test + ghi immutable current commit SHA | - | tester | [x] |
| 5.2 | Kiểm kê parity và assets | Map egui state/storage/window/native/assets vào evidence | 5.1 | explorer | [x] |
| 5.3 | Di chuyển Rust core | `backend/` độc lập, giữ domain behavior | 5.1,5.2 | rust-dev | [x] |
| 5.4 | Xây bridge IPC Tauri | Phase 2 DTO/events/capabilities + containment/confirmation/native safety | 5.3 | rust-dev | [x] |
| 5.5 | Xây frontend và state | `frontend/` parity luồng chính, chỉ gọi bridge | 5.2,5.4 | rust-dev | [x] |
| 5.6 | Tích hợp assets app-local | EN/VI parity; theme/font fallback/persist/package | 5.5 | rust-dev | [x] |
| 5.7 | Kiểm chứng parity | Behavior/persist/native + traversal/symlink/allowed-root/confirmed tests PASS | 5.3-5.6 | tester | [x] |
| 5.8 | Test Rust core | Unit tests backend, `cargo test` PASS | 5.3 | tester | [x] |
| 5.9 | Test IPC/frontend | Contract + frontend tests/build PASS | 5.4-5.6 | tester | [x] |
| 5.10 | Build package Tauri | `cargo check/test/clippy`; deb+AppImage PASS | 5.7-5.9 | tester | [x] |
| 5.11 | Smoke artifact | Launch từ temp ngoài source; resource manifest/loader/hash PASS | 5.10 | tester | [x] |
| 5.12 | Rollback drill | Detached worktree từ SHA 5.1, baseline PASS | 5.11 | tester | [x] |
| 5.13 | Review code và evidence | Reviewer APPROVE toàn Phase 5 | 5.12 | reviewer | [x] |
| 5.14 | Cập nhật docs | Docs/evidence khớp code và commands | 5.13 | docs | [x] |
| 5.15 | Cross-check kết quả | Cross Checker CLEAN trước Phase 6 | 5.14 | cross-checker | [x] |

## Evidence và verify
- Evidence: `.opencode/evidence/gui/universal_converter_gui/{baseline,contract,migration,verify,package,smoke,rollback}.md`.
- Frontend: chạy script build/test được khai báo trong package; backend/bridge: `cargo fmt --check`,
  `cargo check`, `cargo test`, `cargo clippy -- -D warnings`.
- Package Linux: `cargo tauri build --bundles deb,appimage`; MSI/DMG ghi
  `deferred/platform-unverified`. Chạy `.opencode/audit-gui-standalone.sh` sau migration.
- 5.12 dùng worktree tạm, không reset/restore/delete worktree hiện tại; log lệnh và kết quả baseline.
  Không cleanup worktree tạm bằng thao tác xóa nếu chưa có xác nhận user theo destructive policy.
- Native acceptance: contract mutation chỉ hỗ trợ Linux với no-follow API và allowed roots; macOS/Windows
  trả typed `Unavailable` trước job. Linux yêu cầu `confirmed:true`; không `sudo`, CWD hay process exit.
- Contract acceptance (5.4/5.9): checklist exact A.1/A.6/A.7 commands, request/response envelope,
  typed errors, exact event topics/payload, terminal lifecycle và frontend `unlisten` đúng một lần.
  A.3.2 phải test exact allowed root từng command, picker provenance gắn đúng window và empty-root deny.
- Asset acceptance (5.6/5.11): exact A.3.1 DejaVu IDs/files/SHA-256/license, file vật lý không
  symlink, recursive UI-key EN/VI coverage, runtime apply/persist, corrupt/missing fallback và bundle manifest.
- Package acceptance (5.10): exact A.4 product/binary/identifier/version/output, Linux target metadata,
  updater disabled; MSI/DMG chỉ deferred runtime build, static contract vẫn phải PASS.
- Agent mapping xác nhận từ pool runtime của Lead: `explorer`, `rust-dev`, `tester` đều khả dụng;
  model Terra đang DEAD nên chỉ giao các vai dùng Sol, và Lead tự làm phần research nhẹ nếu cần.

## Trạng thái baseline
- Immutable checkpoint: `c24388b30c99323d7fc81032f0454e9e9f1fb705`.
- Baseline fmt/check/test/build PASS; clippy FAIL do 4 lỗi `new_without_default` hiện hữu trong
  `TUI/universal_converter` (`ConfigManager`, `ArchiveEngine`, `DocEngine`, `MediaEngine`). Đây là
  debt baseline cần sửa hoặc được chứng minh không còn trong backend mới trước gate 5.10.
- Evidence: `.opencode/evidence/gui/universal_converter_gui/{baseline,migration}.md`.
- Backend 5.3/5.8: Reviewer APPROVE; fmt/check/clippy sạch, 15/15 tests PASS. Core độc lập có
  containment/recheck, cancellation/progress, overwrite staging, archive repack và native user-scope.
  Portable `std::fs` vẫn có cửa sổ TOCTOU nhỏ giữa recheck cuối và syscall, đã ghi nhận cho review cuối.
- Bridge 5.4: Reviewer APPROVE; backend 17/17 và bridge 15/15 tests PASS, fmt/check/clippy sạch.
  Exact envelopes/events, explicit picker provenance, cancellation teardown, native confirmation và
  preference migration/atomic backup đã có regression tests. Sol từng trả `<none>` một lần rồi retry
  thành công; registry đã chuyển `RECOVERED`.
- Frontend 5.5: Reviewer APPROVE; frontend 8/8 và bridge 15/15 tests PASS, Vite build PASS.
  Exact typed command map/DTO, six job listener lifecycles, file/directory picker intents, native
  confirmation và terminal error de-dup đã được khóa; assets/i18n hoàn chỉnh tiếp tục ở 5.6.
- Assets 5.6: Reviewer APPROVE; frontend 13/13, bridge 17/17 tests PASS, build/fmt/check/clippy sạch.
  EN/VI parity, immediate theme/font/language persistence, exact DejaVu hashes, component-level symlink
  rejection, app-local fallback và exact Tauri resource mappings đã được xác minh.
- Parity 5.7: PASS; behavior/persistence/native confirmation, traversal/intermediate symlink,
  exact roots, explicit picker provenance, cancellation/progress/terminal lifecycle đã được kiểm chứng.
- Contract 5.9/Review 5.13, refreshed by 5.15: Reviewer APPROVE; backend 33/33, bridge 36 PASS + 1 intentionally ignored lock-child helper, frontend 19/19 tests PASS.
  A.6 có 9 exact commands gồm bridge-owned explicit `picker_select`; native operations serialized
  per window; full native install preflight chạy `spawn_blocking` trước registration, failure giữ request ID
  reusable, và preflight được recheck dưới operation lock trước mutation; lifecycle có regression tests.
- Package 5.10: PASS; final deb/AppImage built with exact metadata/updater disabled and A.3.1 two-entry
  font manifest. AppImage SHA-256 `4db4e25ae2d096f9781ce294b66aa3c11d928e08831a519293e3c682c40a9721`.
- Smoke 5.11: PASS from external temp/CWD via extracted AppRun (FUSE unavailable); startup, physical
  bundled resources, exact hashes, app-local DejaVu load and no source/CWD/cross-GUI fallback verified.
  Host GTK/fontconfig reads are correctly allowed for the approved system-font sentinel.
- Rollback 5.12: PASS at immutable SHA `c24388b30c99323d7fc81032f0454e9e9f1fb705` in preserved detached
  worktree `/tmp/opencode/universal-rollback-c24388b-20260910`; baseline commands PASS and current
  worktree status hash remained unchanged. No destructive cleanup was performed.
- Final rebuild blocker: after retiring the legacy egui workspace member and resolving Cross Checker
  issues, a fresh package build stopped with `ENOSPC`. Root filesystem is full; ignored reproducible
  Cargo targets occupy `RS_AI/target` (~51 GiB) and `GUI/universal_converter_gui/bridge/target`
  (~12 GiB). No artifact was deleted; 5.10/5.11 and downstream final gates remain partial until an
  explicitly approved cleanup permits rebuild and re-smoke of the current code.
- Final current-code rebuild after approved cache cleanup: PASS using the shared workspace `target/`.
  Backend 33, bridge 36 + 1 ignored child helper, frontend 19 tests PASS. Final AppImage SHA-256
  `5c2eba9f52e774d1fc045b3782466cb2de4f6d9f7bd1601278089de9e4368045`; final external-CWD smoke
  PASS for both direct AppImage and extracted AppRun. The earlier ENOSPC blocker is resolved.
- Cross-check 5.15: retired egui package excluded/documented without deletion; Linux-only native
  contract, exact-pinned app-owned picker, startup fallback persistence, final current-code rebuild and
  external-CWD smoke, standalone audit fixture/live audit, and `git diff --check` PASS.
