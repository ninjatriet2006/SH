# IMG_SPLT GUI — final 6.10 verification evidence

- Status: **PASS** (2026-09-11); final package and both external-CWD artifact smokes pass.
- Frontend: `npm test` 7/7 PASS; `npm run build` PASS (12 modules).
- Backend: fmt/check/test/clippy `-D warnings` PASS; 27/27 tests PASS; warnings 0.
- Bridge: fmt/check/test/clippy `-D warnings` PASS; 5/5 tests PASS; warnings 0.
- TUI: `cargo test --manifest-path TUI/img_splt/Cargo.toml --all-targets` 18/18 PASS.
- Aggregate tests: 57 PASS, 0 FAIL, 0 ignored (frontend 7 + backend 27 + bridge 5 + TUI 18).
- Build: `cargo tauri build --bundles deb,appimage` PASS from bridge; binary + DEB + AppImage emitted.
- Metadata/resources/updater: PASS; exact 8/8 regular resources in each bundle, font manifest 2/2, updater disabled.
- Exact current sizes/SHA-256 and fresh external evidence are recorded in `package.md` and `smoke.md`.
