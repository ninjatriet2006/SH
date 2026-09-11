# Phase 4 slice — bốn Tauri app hiện hữu

Phạm vi này triển khai phần A.3–A.4 đã xác minh mà không chạm IPC DTO hoặc external-path policy
đang tiếp tục được khóa. Mỗi app phải tự sở hữu `langs/`, `themes/`, `fonts/`; cấm dependency GUI chéo.

| id | công việc | acceptance | phụ thuộc | agent | trạng thái |
|---|---|---|---|---|---|
| A.3-F | Chuẩn hóa Filen | Tự hoàn tất Phase 3 app-local asset gate rồi exact A.3–A.4 | Phase 2 contract | rust-dev | [x] |
| A.3-R | Chuẩn hóa Rclone | Tự hoàn tất Phase 3 app-local asset gate rồi exact A.3–A.4 | Phase 2 contract | rust-dev | [x] |
| A.3-O | Chuẩn hóa OpenCode | Tự hoàn tất Phase 3 app-local asset gate rồi exact A.3–A.4 | Phase 2 contract | rust-dev | [x] |
| A.3-S | Chuẩn hóa Subscription | Tự hoàn tất Phase 3 app-local asset gate rồi exact A.3–A.4 | Phase 2 contract | rust-dev | [x] |
| A.4-V | Validate bốn app | Sở hữu audit script/report; npm/cargo; Linux deb/AppImage; smoke | A.3-F,R,O,S | tester | [x] |
| A.4-R | Review diff/evidence | Review toàn A.3–A.4/no-cross-GUI; loại IPC DTO/path policy | A.4-V | reviewer | [x] |

Current gate status: A.3-F/R/O/S implementation is complete and Reviewer-approved. A.4-V is complete:
all required npm/Cargo checks, Linux packages, bundled-resource inspection, package/smoke hash
correlation, validated X11/D-Bus readiness smoke, preference restart tests and standalone audit pass
for all four apps. A.4-R was APPROVED on 2026-09-09; independent cross-check issues were corrected
(versionable checkpoints, generic runtime-path audit, event registry, and regenerated artifacts).

## Validation gate
- Working directories: frontend commands run in each `GUI/<app>/frontend`; Rust/Tauri commands run
  in Filen `bridge/` and the other three apps' `backend/`. Run `npm run build`; run `npm test` only
  when `package.json` defines a `test` script. Run `cargo check`, `cargo test`, and
  `cargo clippy -- -D warnings` in each Rust/Tauri directory.
- Before bundle/smoke, A.4-V requires successful probes `cargo tauri --version`,
  `pkg-config --exists webkit2gtk-4.1`, `pkg-config --exists gtk+-3.0`, and `command -v dpkg-deb`.
  For display isolation, prefer `xvfb-run`; when it is unavailable, an existing X11 display may be
  used only after recording `DISPLAY`, verifying its Unix socket, and running under a fresh
  `dbus-run-session`. A missing display route is a named environment blocker, never a silent skip.
- Build Linux chỉ `deb` và `appimage`; MSI/DMG chỉ kiểm tra cấu hình và ghi
  `deferred/platform-unverified`, không tuyên bố artifact pass.
- In each Rust/Tauri working directory run
  `cargo tauri build --bundles deb,appimage`; three workspace app tasks must not edit the shared
  root `Cargo.lock`, and A.4-V owns any required lockfile reconciliation after all four tasks.
- Smoke AppImage từ `/tmp/opencode` bằng `xvfb-run`, hoặc validated existing X11 display theo probe
  trên; nếu FUSE thiếu thì extract AppImage và chạy `AppRun` từ external CWD. Start dưới timeout
  20 giây, chờ process sống tối thiểu 5 giây và log
  không có resource-load/panic error, rồi gửi SIGTERM; timeout/SIGTERM sau readiness được chấp nhận,
  crash hoặc tự thoát trước readiness là fail. Log command, status và resource lookup từng app.
- Static assertions per exact A.3–A.4 row: CSP, plugin add/remove, permission set, updater disabled,
  capability duy nhất target `main`, resource mappings, product/identifier/binary/targets, font SHA-256
  and license presence; also defaults, persistence/legacy migration, browser-key prefixes and actual
  event/plugin registration evidence named by the app row.
- Bước đầu A.4-V tạo `.opencode/audit-gui-standalone.sh` theo roadmap: search Cargo/npm manifests,
  TypeScript imports, symlinks và runtime resource paths trong toàn `GUI/`; exit `0` only when no
  non-allowlisted GUI-to-GUI match exists. Run it and save output to
  `.opencode/gui-standalone-audit.md`; mỗi allowlist phải ghi path, match và lý do. Smoke logs must
  contain no source-tree, launch-CWD or other-GUI resource path; any such lookup is a failure.
- Save command/output, manifests and A.3–A.4 source-path comparison in exact paths
  `.opencode/evidence/gui/filen_gui/{verify,package,smoke}.md`,
  `.opencode/evidence/gui/rclone_gui/{verify,package,smoke}.md`,
  `.opencode/evidence/gui/opencode_manager_gui/{verify,package,smoke}.md`, and
  `.opencode/evidence/gui/subscription_manager_gui/{verify,package,smoke}.md`; audit output remains
  the canonical `.opencode/gui-standalone-audit.md`.
- For each app, validate EN/VI key parity, runtime language/theme/font application, corrupt/missing
  asset fallback, and preference survival across process restart; record the test or reproducible
  smoke steps and result in that app's `verify.md`.
