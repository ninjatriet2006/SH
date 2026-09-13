# Subscription Manager GUI — smoke evidence

- Status: PASS (2026-09-09).
- Artifact: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/appimage/subscription_manager_gui_0.1.0_amd64.AppImage` (regenerated because the shared bundle directory had been overwritten by a later app build).
- Artifact SHA-256: `ef8e6f1bf98f0fba7d9aa277ba3f7306344746c7afd28477c93b9a7719b4f638`, identical to `package.md`.
- Display route: `DISPLAY=:0`; `test -S /tmp/.X11-unix/X0` PASS; fresh D-Bus route `/usr/bin/dbus-run-session`. External-CWD command from `/tmp/opencode`: `DISPLAY=:0 timeout --signal=TERM --kill-after=2s 20s dbus-run-session -- <AppImage>`; no tool installed.
- Readiness: process `subscription_manager_gui` remained alive >=5 seconds; QA sent SIGTERM; wrapper exit `143` accepted. No app panic, resource-load, source-tree, launch-CWD or cross-GUI lookup appeared in output.
- Benign host-service output: xdg-document portal/GVFS reported FUSE permission errors under `/run/user/1000/{doc,gvfs}`; AppImage itself mounted and app remained ready, so extraction fallback was not required for launch.
- Bundle inspection: `--appimage-extract` PASS; `usr/lib/subscription_manager_gui/{langs,themes,fonts}` present with 2/2/4 files respectively.
- Phase 8.6 refresh (2026-09-12): AppImage `d5a3dca2...1e84` launched from `/tmp`; alive to timeout 124 with an empty log.
- Post-four-fix Phase 8.6: fresh AppImage `491d7935...3db08a` and extracted DEB binary passed isolated external-CWD/XDG launch (timeout 124); extraction/resources/font manifest PASS. Evidence root: `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
