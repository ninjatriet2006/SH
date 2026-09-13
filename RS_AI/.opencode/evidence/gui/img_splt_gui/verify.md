# IMG_SPLT GUI — final 6.10 verification evidence

- Status: **PASS** (2026-09-11); latest backend/bridge fixes, fresh package, and both external-CWD artifact smokes pass.
- Frontend: `npm test` 7/7 PASS; `npm run build` PASS (12 modules).
- Backend: fmt/check/test/clippy `-D warnings` PASS; 30/30 tests PASS (including identity/rollback edge coverage); warnings 0.
- Bridge: fmt/check/test/clippy `-D warnings` PASS; 6/6 tests PASS (including structured partial-rollback mapping); warnings 0.
- TUI: fmt/check/test/clippy `-D warnings` PASS; 18/18 tests PASS; warnings 0.
- Aggregate tests: 61 PASS, 0 FAIL, 0 ignored (frontend 7 + backend 30 + bridge 6 + TUI 18).
- Review/audit: Reviewer **APPROVE**; current full-GUI standalone audit scanned 30 manifests, 436 runtime/config files, and 108 symlinks with no allowlist or GUI-to-GUI coupling.
- Build: `cargo tauri build --bundles deb,appimage` PASS from bridge; binary + DEB + AppImage emitted.
- Metadata/resources/updater: PASS; exact 8/8 regular resources in each bundle, font manifest 2/2, updater disabled.
- Exact current sizes/SHA-256 and fresh external evidence are recorded in `package.md` and `smoke.md` under `/tmp/opencode/img-splt-final-039b2ce6-20260911-1059`.
- Phase 8 rerun (2026-09-12): frontend 7/7 plus build PASS; backend+bridge 36/36 and fmt/check/clippy PASS with 0 warnings. Overall FAIL because fresh packaging is blocked by nested workspace discovery.
- Post-four-fix rerun: frontend 7/7 + build; backend+bridge fmt/check/test/clippy 36/36, 0 warnings; workspace/package blocker fixed; fresh DEB/AppImage and dual external-CWD smoke PASS.
