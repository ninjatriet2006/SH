# Filen GUI — smoke evidence

- Status: PASS (2026-09-09).
- Artifact: `/home/bimatkeo/Documents/SH/RS_AI/GUI/filen_gui/bridge/target/release/bundle/appimage/filen_gui_0.1.0_amd64.AppImage`.
- Artifact SHA-256: `a9220b192ad32b828538ebc453ee2841f7762f20cb47badedb7a1b6f30078f64`, identical to `package.md`.
- Display route: `DISPLAY=:0`; `test -S /tmp/.X11-unix/X0` PASS; fresh D-Bus route `/usr/bin/dbus-run-session`. External-CWD command from `/tmp/opencode`: `DISPLAY=:0 timeout --signal=TERM --kill-after=2s 20s dbus-run-session -- <AppImage>`; no tool installed.
- Readiness: process `filen_gui_tauri` remained alive >=5 seconds; QA sent SIGTERM; wrapper exit `143` accepted. No app panic, resource-load, source-tree, launch-CWD or cross-GUI lookup appeared in output.
- Benign host-service output: xdg-document portal/GVFS reported FUSE permission errors under `/run/user/1000/{doc,gvfs}`; AppImage itself mounted and app remained ready, so extraction fallback was not required for launch.
- Bundle inspection: `--appimage-extract` PASS; `usr/lib/filen_gui/{langs,themes,fonts}` present with 2/1/4 files respectively.
- Phase 8.6 refresh (2026-09-12): fresh AppImage `3759463b...edf5` launched from `/tmp` with `APPIMAGE_EXTRACT_AND_RUN=1`; alive to timeout 124. Log has only benign host portal/GVFS/FUSE diagnostics.
- Post-four-fix Phase 8.6: fresh AppImage `05fd1fd...56f9b` and extracted DEB binary both launched from isolated non-repo CWD/XDG roots and stayed alive to timeout 124; extraction/resources/font manifest PASS. Logs: `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Post-IPC Phase 8.6 smoke (2026-09-13): PASS from external CWD `/tmp/opencode` with separate empty XDG config/cache/data roots and fresh D-Bus sessions. AppImage `7d6dc4f...d15148` and extracted-DEB `filen_gui_tauri` both remained alive for the full 12-second gate and exited by expected `timeout` code 124.
- AppImage extraction and `dpkg-deb -x` PASS. Both payloads contain byte-identical app-owned `langs/`, `themes/`, and `fonts/` resources (7/7 checked against source); EN/VI key parity is 104/104 and the DejaVu font manifest verifies 2/2. No app panic, fatal error, IPC error, missing-resource, source-tree, or cross-GUI lookup appeared; logs only contain benign host portal/GVFS FUSE permission diagnostics. Evidence/log root: `/tmp/opencode/filen-phase8-ipc-20260913T005538Z`.
