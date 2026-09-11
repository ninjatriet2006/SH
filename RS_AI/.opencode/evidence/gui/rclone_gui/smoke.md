# Rclone GUI — smoke evidence

- Status: PASS (2026-09-09).
- Smoke artifact at execution time: shared-target AppImage with SHA-256 `a6c778afa51a3c315a008d023cc51b589346d6415c14a54a5d4857d8219bffc1`.
- Final post-check package artifact: `/tmp/opencode/rclone_gui_0.1.0_amd64.AppImage`, SHA-256 `459ceb87c9c953c5749d775751b2d38aca9377d69648a62466c656039e5b949a`, matching `package.md`; rerun from `/tmp/opencode` with the command below remained alive until timeout (exit 124), with no panic or resource/cross-GUI lookup.
- Display route: `DISPLAY=:0`; `test -S /tmp/.X11-unix/X0` PASS; fresh D-Bus route `/usr/bin/dbus-run-session`. External-CWD command from `/tmp/opencode`: `DISPLAY=:0 timeout --signal=TERM --kill-after=2s 20s dbus-run-session -- <AppImage>`; no tool installed.
- Readiness: final artifact process `rclone_gui` remained alive for the full 8-second rerun; timeout sent SIGTERM and wrapper exit `124` is accepted. No app panic, resource-load, source-tree, launch-CWD or cross-GUI lookup appeared in output.
- Benign host-service output: xdg-document portal/GVFS reported FUSE permission errors under `/run/user/1000/{doc,gvfs}`; AppImage itself mounted and app remained ready, so extraction fallback was not required for launch.
- Bundle inspection: `--appimage-extract` PASS; `usr/lib/rclone_gui/{langs,themes,fonts}` present with 2/3/4 files respectively.
