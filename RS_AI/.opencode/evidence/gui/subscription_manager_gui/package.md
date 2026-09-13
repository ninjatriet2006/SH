# Subscription Manager GUI — package evidence

- Run: `cargo tauri build --bundles deb,appimage` in `GUI/subscription_manager_gui/backend` — PASS (combined syntax accepted), 2026-09-09. No tool installed; smoke result is recorded separately in `smoke.md`.
- Binary: `target/release/subscription_manager_gui` — 7,737,200 bytes — SHA-256 `3b543be1615e3f08a1c7ba4c91510f1871617f78d5b2e63e883b41c63cd59623`.
- DEB: `target/release/bundle/deb/subscription_manager_gui_0.1.0_amd64.deb` — 3,513,068 bytes — SHA-256 `bfe0eca23e347887da77c79c8da15ed8f7193694e0ca9e8d87ceffc07965896d`; `dpkg-deb --info` PASS (`subscription-manager-gui`, `0.1.0`, `amd64`).
- AppImage (regenerated/recomputed for final review): `target/release/bundle/appimage/subscription_manager_gui_0.1.0_amd64.AppImage` — 79,907,320 bytes — SHA-256 `ef8e6f1bf98f0fba7d9aa277ba3f7306344746c7afd28477c93b9a7719b4f638`.
- Host: Tauri CLI `2.11.4`, WebKitGTK `2.52.6`, GTK `3.24.41`, `/usr/bin/dpkg-deb`, `DISPLAY=:0` and X0 socket present; `xvfb-run` absent but smoke was excluded.
- Root `Cargo.lock` was not edited/reconciled and retained pre-existing dirty hash `1795db56fd55f8977ec05d1f58abde691972231476ee26d539607e7ebec255fe`.
- MSI/DMG: `deferred/platform-unverified` on Linux.
- Phase 8.4 refresh (2026-09-12): preserved AppImage `d5a3dca2e489ff54271983b0710062002b27e50542743e3c6cb372ece8601e84`, DEB `b3513ddabec4253c9687c5a9deffe2057ec1cab826303abf7f9a8c3f872fc43b` under `/tmp/opencode/phase-8-artifacts-20260912`.
- Post-four-fix Phase 8.4 rebuild: AppImage `491d793512ffe1b41b98b27491f02b8882f70502a5d2348e5d2b2187aa3db08a`; DEB `f059d15f8a3e1aa1214e400abefa2cc074ec4069a941869952b80ec6023b5cf5`; preserved under `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
