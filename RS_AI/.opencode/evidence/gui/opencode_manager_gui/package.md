# OpenCode Manager GUI — package evidence

- Run: `cargo tauri build --bundles deb,appimage` in `GUI/opencode_manager_gui/backend` — PASS (combined syntax accepted), 2026-09-09. No tool installed; smoke result is recorded separately in `smoke.md`.
- Binary: `target/release/opencode_manager_gui` — 8,908,224 bytes — SHA-256 `5d87d3ff8523653d366fbf740074eb16dcc4b2316b9bfe7cc41c7a1993e1702a`.
- DEB: `target/release/bundle/deb/opencode_manager_gui_0.1.0_amd64.deb` — 3,863,018 bytes — SHA-256 `65f52f3589deaf9d2fb9660684d38d36dfac621a232d5f6b6d9154aab26c154d`; `dpkg-deb --info` PASS (`opencode-manager-gui`, `0.1.0`, `amd64`).
- AppImage (regenerated after final source checks, then preserved outside the shared target): `/tmp/opencode/opencode_manager_gui_0.1.0_amd64.AppImage` — 82,209,272 bytes — SHA-256 `17dc79df37e353414c16a5e1a9439f60246b8e45e6d229fd4d7a270580d09f13`.
- Host: Tauri CLI `2.11.4`, WebKitGTK `2.52.6`, GTK `3.24.41`, `/usr/bin/dpkg-deb`, `DISPLAY=:0` and X0 socket present; `xvfb-run` absent but smoke was excluded.
- Root `Cargo.lock` was not edited/reconciled and retained pre-existing dirty hash `1795db56fd55f8977ec05d1f58abde691972231476ee26d539607e7ebec255fe`.
- MSI/DMG: `deferred/platform-unverified` on Linux.
- Phase 8.4 refresh (2026-09-12): preserved AppImage `5c7bad3e35950803fd9aea3cf8c1d47b3ff6f815419ef531c0c5a4568937f5bc`, DEB `bcd95e3a45dac5c2959235eecd7a49f9e7cbc7e830925c749257c43e39a96481` under `/tmp/opencode/phase-8-artifacts-20260912`.
- Post-four-fix Phase 8.4 rebuild: AppImage `c0e6aba676a0f4fa86ecb492edf80cef56835d5019ae528e0f8e47281f086f48`; DEB `025a9f6eaa7cebeca3cfcebff1db8bc0450123c51385856c04b82472c7b56f9b`; preserved under `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Post-IPC Phase 8.4/8.6 rebuild (2026-09-13): `CARGO_TARGET_DIR=/tmp/opencode/opencode-ipc-refresh-20260913/target cargo tauri build --bundles deb,appimage`; DEB completed in the combined run and AppImage completed on an isolated retry after concurrent `linuxdeploy` cache contention (`Text file busy`). Fresh outputs are preserved in `/tmp/opencode/opencode-ipc-refresh-20260913/`.
- DEB: `opencode_manager_gui_0.1.0_amd64.deb`, 3,947,104 bytes, SHA-256 `77a50844b4abc0f2b13eb821229d2ca756bca9a8ff2eddad834ef9bb9ad4be8b`; `dpkg-deb --info/-f` PASS: package `opencode-manager-gui`, version `0.1.0`, architecture `amd64`.
- AppImage: `opencode_manager_gui_0.1.0_amd64.AppImage`, 82,287,096 bytes, SHA-256 `ded40c2659cf6df262ed399217add49d6839af0769b9743951de429a52c6a563`; x86-64 ELF and `--appimage-extract` PASS. Linux binary SHA-256: `ccd94daf64bd58f3e539b8a09581ee0f7acc2733b6a17842c962bf4199559bab`.
- DEB/AppImage resource trees are regular, byte-identical copies of app-owned `langs` 2/2, `themes` 2/2 and `fonts` 4/4; both packaged font manifests verify 2/2. Config metadata remains `opencode_manager_gui` / `com.opencode.manager` / `0.1.0`, updater artifacts disabled; MSI/DMG remain deferred/platform-unverified on Linux.
