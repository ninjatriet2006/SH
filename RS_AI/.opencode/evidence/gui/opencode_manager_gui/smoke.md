# OpenCode Manager GUI — smoke evidence

- Status: PASS (2026-09-09).
- Smoke artifact at execution time: shared-target AppImage with SHA-256 `f5c908c503eea25ec5b5b06c9cc761a46fb9b1b6f6b9aaecdf928a80cc3b898b`.
- Final post-check package artifact: `/tmp/opencode/opencode_manager_gui_0.1.0_amd64.AppImage`, SHA-256 `17dc79df37e353414c16a5e1a9439f60246b8e45e6d229fd4d7a270580d09f13`, matching `package.md`; rerun from `/tmp/opencode` with the command below remained alive until timeout (exit 124), with no panic or resource/cross-GUI lookup.
- Display route: `DISPLAY=:0`; `test -S /tmp/.X11-unix/X0` PASS; fresh D-Bus route `/usr/bin/dbus-run-session`. External-CWD command from `/tmp/opencode`: `DISPLAY=:0 timeout --signal=TERM --kill-after=2s 20s dbus-run-session -- <AppImage>`; no tool installed.
- Readiness: final artifact process `opencode_manager_gui` remained alive for the full 8-second rerun; timeout sent SIGTERM and wrapper exit `124` is accepted. No app panic, resource-load, source-tree, launch-CWD or cross-GUI lookup appeared in output.
- Benign host-service output: xdg-document portal/GVFS reported FUSE permission errors under `/run/user/1000/{doc,gvfs}`; AppImage itself mounted and app remained ready, so extraction fallback was not required for launch.
- Bundle inspection: `--appimage-extract` PASS; `usr/lib/opencode_manager_gui/{langs,themes,fonts}` present with 2/2/4 files respectively.
- Phase 8.6 refresh (2026-09-12): AppImage `5c7bad3e...f5bc` launched from `/tmp`; timeout 124, only benign host portal/GVFS/FUSE diagnostics.
- Post-four-fix Phase 8.6: fresh AppImage `c0e6aba6...086f48` and extracted DEB binary passed isolated external-CWD/XDG launch (timeout 124); extraction/resources/font manifest PASS. Evidence root: `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
