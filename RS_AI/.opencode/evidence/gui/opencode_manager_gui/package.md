# OpenCode Manager GUI — package evidence

- Run: `cargo tauri build --bundles deb,appimage` in `GUI/opencode_manager_gui/backend` — PASS (combined syntax accepted), 2026-09-09. No tool installed; smoke result is recorded separately in `smoke.md`.
- Binary: `target/release/opencode_manager_gui` — 8,908,224 bytes — SHA-256 `5d87d3ff8523653d366fbf740074eb16dcc4b2316b9bfe7cc41c7a1993e1702a`.
- DEB: `target/release/bundle/deb/opencode_manager_gui_0.1.0_amd64.deb` — 3,863,018 bytes — SHA-256 `65f52f3589deaf9d2fb9660684d38d36dfac621a232d5f6b6d9154aab26c154d`; `dpkg-deb --info` PASS (`opencode-manager-gui`, `0.1.0`, `amd64`).
- AppImage (regenerated after final source checks, then preserved outside the shared target): `/tmp/opencode/opencode_manager_gui_0.1.0_amd64.AppImage` — 82,209,272 bytes — SHA-256 `17dc79df37e353414c16a5e1a9439f60246b8e45e6d229fd4d7a270580d09f13`.
- Host: Tauri CLI `2.11.4`, WebKitGTK `2.52.6`, GTK `3.24.41`, `/usr/bin/dpkg-deb`, `DISPLAY=:0` and X0 socket present; `xvfb-run` absent but smoke was excluded.
- Root `Cargo.lock` was not edited/reconciled and retained pre-existing dirty hash `1795db56fd55f8977ec05d1f58abde691972231476ee26d539607e7ebec255fe`.
- MSI/DMG: `deferred/platform-unverified` on Linux.
