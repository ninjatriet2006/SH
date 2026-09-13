# Subscription Manager GUI — verification

- Result: PASS (2026-09-09). Working directories: `GUI/subscription_manager_gui/frontend` and `GUI/subscription_manager_gui/backend`; A.4-V `[x]`.
- `npm run build`: PASS; package has no `test` script. `cargo fmt --check`: PASS. `cargo test`: PASS, 48 passed / 0 failed (45 unit + 3 integration). `cargo clippy -- -D warnings`: PASS, 0 warnings.
- Static A.3–A.4: PASS — sole capability, exact CSP/scope/dialog permission, dialog plugin registered, debug-only log backend plugin, updater disabled, resources/metadata/targets, font hashes/license. MSI/DMG `deferred/platform-unverified`.
- EN/VI recursive parity PASS. Runtime language/theme/font/fallback: `backend/src/{lang_api.rs,theme_api.rs,font_api.rs,settings_api.rs}`, frontend `SettingsPage.tsx` and theme store; corrupt settings fallback is covered at `settings_api.rs:86`.
- Persistence/migration: canonical paths and data mirror tests in `backend/src/storage.rs`; settings persistence in `settings_api.rs`. Browser keys in `frontend/src/components/InvoiceModal.tsx`: `vietqr_bank_bin`, `vietqr_account_no`, `vietqr_account_name`.
- Plugin evidence: `backend/src/lib.rs`; authoritative event row says no event, matching audited backend/frontend.
- Preference restart survival: PASS. Final targeted rerun `settings_api::tests::save_then_fresh_read_giu_nguyen_settings` PASS (1/1); it writes canonical+mirror under a unique temp directory, fresh-reads and deserializes all settings unchanged, bypasses global env and never touches user files. Packaging/readiness smoke remain PASS.
- Phase 8 rerun (2026-09-12): frontend build PASS (no test script), lint exits 0 with 20 warnings. Rust FAIL: 52/53; `tests/ipc_param_names.rs:352` reports public command names missing and `ipc_*` wrappers registered instead; clippy not reached. Fresh package/smoke PASS.
- Post-four-fix rerun: IPC regression fixed; Rust fmt/check/test/clippy PASS, 55/55 and 0 warnings. Frontend still has no test script; lint/build PASS with 20 non-blocking lint warnings. Fresh DEB/AppImage and dual external-CWD smoke PASS.
- Reviewer F1 rerun (2026-09-13): no frontend test script; lint/build PASS with 20 warnings. Rust fmt/check/test/clippy 55/55, 0 warnings; fresh DEB/AppImage inspection and dual external-CWD smoke PASS. Evidence: `/tmp/opencode/phase8-f1-five-20260913T000000Z/subscription`.
