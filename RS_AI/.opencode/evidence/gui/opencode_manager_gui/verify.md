# OpenCode Manager GUI — verification

- Result: PASS (2026-09-09). Working directories: `GUI/opencode_manager_gui/frontend` and `GUI/opencode_manager_gui/backend`; A.4-V `[x]`.
- `npm run build`: PASS; no `test` script exists, so npm tests are not applicable under the plan.
- Backend rerun after the persistence test: `cargo fmt -- --check`, `cargo check`, and `cargo clippy -- -D warnings`: PASS, 0 warnings; `cargo test`: PASS, 57 passed / 0 failed (54 unit + 3 integration).
- The four prior diagnostics at `backend/src/api/ckey.rs:165` and `provider.rs:165,191,447` are resolved without changing provider command signatures/IPC DTOs or provider persistence behavior.
- Static A.3–A.4: PASS — exact capability/CSP/resources/metadata/targets/updater and font hashes/license. MSI/DMG `deferred/platform-unverified`.
- EN/VI recursive parity PASS. Runtime language/theme/font and persistence paths: `backend/src/api/{lang.rs,font.rs,settings.rs}`, frontend stores `useSettingsStore.ts`, `useThemeStore.ts`, `useFontStore.ts`; legacy `manager_gui.json` import and `.legacy` rename are tested in `settings.rs`.
- Event evidence: producer `backend/src/api/arbiter.rs` and listener
  `frontend/src/pages/ModelsPage.tsx:125` use `arbiter://progress`. `cargo tauri build --bundles deb,appimage`
  and external-CWD AppImage readiness smoke PASS; details are in `package.md` and `smoke.md`.
- Preference restart survival: PASS by exact targeted rerun `cargo test api::settings::tests::save_then_fresh_read_giu_nguyen_language_theme_font -- --exact` (1/1). It saves `vi` / `red_blood` / `dejavusans` to a unique system-temp path, then performs a fresh filesystem read and verifies all three exact values; it bypasses migration/global env and never accesses user config/data paths.
- Phase 8 rerun (2026-09-12): frontend 8/8, lint/build PASS; Rust 75/75, fmt/check/clippy PASS with 0 warnings; fresh package/smoke PASS.
- Post-four-fix rerun: unchanged PASS — frontend 8/8 + lint/build; Rust fmt/check/test/clippy 75/75, 0 warnings; fresh DEB/AppImage and dual external-CWD smoke PASS.
