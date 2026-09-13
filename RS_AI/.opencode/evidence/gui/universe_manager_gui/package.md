# Universe Manager GUI — Phase 7.7 package evidence

- Status: **PASS** (fresh rebuild 2026-09-11 after five Reviewer fixes).
- Fresh gates: frontend 9/9 plus build (12 modules); backend 13/13; bridge 16/16; Universe TUI 32/32; fmt/check/clippy PASS with 0 warnings.
- Command: `cargo tauri build --bundles deb,appimage` completed from `GUI/universe_manager_gui/bridge`.
- Binary: `bridge/target/release/universe-manager-gui` — 16,070,416 bytes — SHA-256 `3e31bb00e0a268ef38f167ea0a2f5186b78cc3bde35a24b73d5045b2b87d29b4`.
- DEB: `bridge/target/release/bundle/deb/Universe Manager_0.1.0_amd64.deb` — 5,145,760 bytes — SHA-256 `edaff3cd77deb83303c26e9dfc83cfe15c5c516f712c70db1712d7fa8f8901ac`; package `universe-manager`, version `0.1.0`, architecture `amd64`.
- AppImage: `bridge/target/release/bundle/appimage/Universe Manager_0.1.0_amd64.AppImage` — 81,259,000 bytes — SHA-256 `95e467b25eb44a9c645641e8740bd0f76cb03cdf0fe3e9b914a6008fd5b77165`.
- Phase 8.4 refresh (2026-09-12): preserved AppImage `eda1b8624667c5a7ddd0b3bf45cabfee14998810060c478a3de1451f41c828aa`, DEB `a3bbae4782cb1ed61646d95c050fded77868647fb0049b2d5782facc6d0521e5` under `/tmp/opencode/phase-8-artifacts-20260912`.
- Exact metadata: product `Universe Manager`; binary `universe-manager-gui`; identifier `com.sh.universe-manager`; version `0.1.0`; output root `GUI/universe_manager_gui/bridge/target/release/`.
- Resources: all 16 checks (8 files × 2 extracted bundles) are regular/no-symlink and byte-identical to app-owned source. Font hashes: DejaVuSans `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280`; license `63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9`.
- Updater is off: `createUpdaterArtifacts:false`, empty plugins, and no updater dependency/registration. Config retains `deb`, `appimage`, `msi`, `dmg`; MSI/DMG are platform-deferred and unverified.
- Preserved extraction/logs: `/tmp/opencode/universe-reviewer-fixes-95e467b-20260911-qa`; nothing deleted. Runtime details are in `smoke.md`.
- Post-four-fix Phase 8.4 rebuild: AppImage `dfd954e07d6eb6f07966580bf223f8223671d90f73f6ee36bbafc4b14953cac0`; DEB `79257cd4eddacaf8d7459f991d611292ab3761cec634028f8a3ec4353900a5d8`; preserved under `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Reviewer F1 refresh (2026-09-13): fresh AppImage `f5c971dbf8fd96d33440645f1726fbdcd940dd0df6b5bcef617824601f93c315` and DEB `ac2ee889ea3dc94cc7e3dbf43094176045396938989bc4d89089fad1f5a32326`; metadata `universe-manager`/`0.1.0`/`amd64`; durable evidence under `/tmp/opencode/phase8-f1-five-20260913T000000Z/universe`.
