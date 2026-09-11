# Rclone GUI — package evidence

- Run: `cargo tauri build --bundles deb,appimage` in `GUI/rclone_gui/backend` — PASS (combined syntax accepted), 2026-09-09. No tool installed; smoke result is in `smoke.md`. Frontend emitted 2 non-fatal ineffective-dynamic-import warnings.
- Binary: `target/release/rclone_gui` — 12,863,336 bytes — SHA-256 `b6c9bf8885024249f0b7e717f1cedc955946bddc118d42cfd6d5712b6796f38c`.
- DEB: `target/release/bundle/deb/rclone_gui_0.1.0_amd64.deb` — 5,104,760 bytes — SHA-256 `37099a666ea50b523b58273904d5c23d2c5614583ca9eb4624d42589869bdbee`; `dpkg-deb --info` PASS (`rclone-gui`, `0.1.0`, `amd64`).
- AppImage (regenerated after final source checks, then preserved outside the shared target): `/tmp/opencode/rclone_gui_0.1.0_amd64.AppImage` — 81,283,576 bytes — SHA-256 `459ceb87c9c953c5749d775751b2d38aca9377d69648a62466c656039e5b949a`.
- Host: Tauri CLI `2.11.4`, WebKitGTK `2.52.6`, GTK `3.24.41`, `/usr/bin/dpkg-deb`, `DISPLAY=:0` and X0 socket present; `xvfb-run` absent but smoke was excluded.
- Root `Cargo.lock` was not edited/reconciled and retained pre-existing dirty hash `1795db56fd55f8977ec05d1f58abde691972231476ee26d539607e7ebec255fe`.
- MSI/DMG: `deferred/platform-unverified` on Linux.
