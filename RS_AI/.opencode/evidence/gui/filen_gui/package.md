# Filen GUI — package evidence

- Run: `cargo tauri build --bundles deb,appimage` in `GUI/filen_gui/bridge` — PASS (combined syntax accepted), 2026-09-09. No tool installed; smoke result is recorded separately in `smoke.md`.
- First attempt selected auxiliary binary `gtk_test` and emitted `__TAURI_BUNDLE_TYPE variable not found`; direct packaging fix `default-run = "filen_gui_tauri"` was added to `bridge/Cargo.toml`. Rerun selected the intended binary and emitted no bundler-type warning.
- Binary: `GUI/filen_gui/bridge/target/release/filen_gui_tauri` — 20,043,752 bytes — SHA-256 `29a1311089ee340de9970cfc5896931741fb57b634ce2e96ada11725e40a7a1a`.
- DEB: `GUI/filen_gui/bridge/target/release/bundle/deb/filen_gui_0.1.0_amd64.deb` — 7,131,662 bytes — SHA-256 `b13726f99bb556a8264088c631e2b154ee75935aa14737ee492960a735cd5c85`; `dpkg-deb --info` PASS (`filen-gui`, `0.1.0`, `amd64`).
- AppImage: `GUI/filen_gui/bridge/target/release/bundle/appimage/filen_gui_0.1.0_amd64.AppImage` — 83,012,088 bytes — SHA-256 `a9220b192ad32b828538ebc453ee2841f7762f20cb47badedb7a1b6f30078f64`.
- Host: Tauri CLI `2.11.4`, WebKitGTK `2.52.6`, GTK `3.24.41`, `/usr/bin/dpkg-deb`, `DISPLAY=:0` and X0 socket present; `xvfb-run` absent but not required because smoke was excluded.
- Lockfiles were not edited/reconciled: root hash `1795db56fd55f8977ec05d1f58abde691972231476ee26d539607e7ebec255fe`; Filen hash `c49da9bb24acc0af82eeb834dbbb3f054390a9af0e754828b195d0df58294c0c` (both pre-existing dirty).
- MSI/DMG: `deferred/platform-unverified` on Linux.
