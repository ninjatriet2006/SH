# Rclone GUI — verification

- Result: PASS (2026-09-09). Working directories: `GUI/rclone_gui/frontend` and `GUI/rclone_gui/backend`; A.4-V `[x]`.
- Reviewer-fix validation (2026-09-09): `npm test` PASS, 6 files / 35 tests; `npm run build` PASS with 2 non-fatal dynamic-import warnings.
- `cargo check`: PASS. `cargo test`: PASS, 34 passed / 0 failed. `cargo clippy -- -D warnings`: PASS, 0 warnings.
- Static A.3–A.4: PASS — sole capability, exact CSP/scope/permissions, drag plugin registered and shell removed, updater disabled, resources/product/identifier/binary/targets, DejaVu hashes/license. Linux verified; MSI/DMG `deferred/platform-unverified`.
- EN/VI recursive key parity: PASS. Runtime application/fallback: `frontend/src/features/{i18n.ts,appearance.ts}`, tests `i18n.test.ts` and `appearance.test.ts`; backend fallback/discovery: `backend/src/api/{lang.rs,appearance.rs}`.
- Defaults/persistence/legacy migration: `frontend/src/store.ts`; `frontend/src/store.test.ts` verifies all three `filen_*` to `rclonegui_*` migrations without overwrite and invalid language/theme/font normalization to the first available ID (4 added tests). Normalized values are persisted by `initSettings()` via `saveSettings()` in `frontend/src/main.ts`. Browser keys are `rclonegui_settings`, `rclonegui_activity_log`, `rclonegui_bookmarks`.
- Event/plugin evidence: `backend/src/lib.rs`, `backend/src/logic/{watcher.rs,transfer.rs}`, `frontend/src/components/DualPaneExplorer.ts`, `frontend/src/features/transferManager.ts`.
- Expanded standalone audit PASS after correcting stale cross-app names in `themes/{style.css,tokens.css}`: latest `.opencode/audit-gui-standalone.sh` run scanned 18 manifests, 319 runtime/config files and 72 symlinks; canonical output `.opencode/gui-standalone-audit.md`.
- `cargo tauri build --bundles deb,appimage` and external-CWD AppImage readiness smoke: PASS; package details are in `package.md`, launch/resource details in `smoke.md`.
- Preference restart survival: PASS. `frontend/src/store.test.ts` uses isolated localStorage, writes all four settings via `saveSettings()`, resets the module cache as a restart-equivalent, then fresh-imports `store.ts` and reads back identical values (targeted rerun 10/10; full suite 35/35).
- Phase 8 rerun (2026-09-12): frontend 35/35 plus build PASS; Rust 34/34, fmt/check/clippy PASS with 0 warnings; fresh package/smoke PASS.
- Post-four-fix rerun: unchanged PASS — frontend 35/35 + build; Rust fmt/check/test/clippy 34/34, 0 warnings; fresh DEB/AppImage and dual external-CWD smoke PASS.
