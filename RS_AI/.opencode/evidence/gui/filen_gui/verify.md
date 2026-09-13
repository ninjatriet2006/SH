# Filen GUI — verification

- Result: PASS (2026-09-09). Working directory: `GUI/filen_gui/frontend` for npm and `GUI/filen_gui/bridge` for Cargo; A.4-V `[x]`.
- `npm run build`: PASS. `npm test`: PASS, 6 files / 39 tests; expected mocked-runtime stderr reports missing `window`/`localStorage`, with no failed test. Build emitted 6 ineffective-dynamic-import warnings.
- `cargo check --manifest-path GUI/filen_gui/bridge/Cargo.toml`: PASS. `cargo test ...`: PASS, 1 passed / 0 failed. `cargo clippy ... -- -D warnings`: PASS, 0 warnings.
- Static A.3–A.4: PASS — sole `capabilities/main.json`, exact CSP/scope/permissions, drag plugin registered and shell removed, updater disabled, resources/product/identifier/binary/targets, and exact DejaVu SHA-256/license all match the plan. Linux verified; MSI/DMG `deferred/platform-unverified`.
- EN/VI recursive key parity: PASS. Runtime language/theme/font paths: `frontend/src/main.ts`, `components/SettingsModal.ts`, `appearance.ts`; corrupt/unknown theme and missing-font fallback: `tests/frontend/unit/themes.test.ts`; defaults and persisted-setting normalization: `tests/frontend/unit/store.test.ts`.
- Browser keys: `frontend/src/store.ts` (`filen_settings`, `filen_activity_log`, `filen_bookmarks`). Event/plugin evidence: `bridge/src/lib.rs`, `bridge/src/transfer_cmds.rs`, `frontend/src/components/DualPaneExplorer.ts`, `frontend/src/features/transferManager.ts`.
- Preference restart survival: PASS. `tests/frontend/unit/store.test.ts` uses isolated localStorage, writes all four settings via `saveSettings()`, resets the module cache as a restart-equivalent, then fresh-imports `store.ts` and reads back identical values (targeted rerun 3/3; full suite 39/39).
- `cargo tauri build --bundles deb,appimage`: PASS after direct packaging fix selecting `filen_gui_tauri` as `default-run`; exact paths/sizes/hashes are in `package.md`.
- Phase 8 rerun (2026-09-12): frontend 39/39 and build PASS; fresh package/smoke PASS. Rust gate FAILS at `cargo fmt --check` (first diff `bridge/build.rs:1`), so later Rust gates are not claimed.
- Post-four-fix rerun: frontend 39/39 + build PASS; Rust fmt/check/test/clippy PASS (1/1, 0 warnings); fresh DEB/AppImage and dual external-CWD smoke PASS. Exact hashes are in `package.md`.
