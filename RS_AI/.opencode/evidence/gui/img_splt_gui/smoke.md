# IMG_SPLT GUI — fresh artifact external-CWD smoke evidence
- Status: **PASS** (2026-09-11): fresh DEB and AppImage start outside repo/CWD; metadata/resources/hashes/updater checks PASS.
- Commands: extract with `dpkg-deb -x` and AppImage `--appimage-extract`; launch direct AppImage and extracted DEB `usr/bin/img-splt-gui` under `timeout 15s` from isolated CWDs.
- External CWD `/tmp/opencode/img-splt-final-409a07d9-20260911`: both launches stayed alive to timeout (exit 124), with no app panic/resource error; only host portal/GVFS FUSE permission diagnostics.
- DEB: 3,566,906 bytes; SHA-256 `6ca6dac46c8877179688e62a4c3d55e513985247391c1fd0a8f96bd586108382`; metadata `image-splitter`/`0.1.0`/`amd64`.
- AppImage: 79,890,936 bytes; SHA-256 `409a07d9966d229d8c7a5b4fff6e7139acf9c38ee94a60d32b9642f6bd0a2473`.
- Resources PASS: exact required 8/8 files in both extracted artifacts are regular/no-symlink and `cmp` byte-identical to `GUI/img_splt_gui/{langs,themes,fonts}`.
- Font manifest PASS in both artifacts: DejaVuSans `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280`; license `63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9`.
- Updater disabled: config has `createUpdaterArtifacts: false`, empty plugins, and bridge has no updater dependency or registration.
- Preserved evidence/temp: `/tmp/opencode/img-splt-final-409a07d9-20260911` (fresh extraction, launch logs/exits, exact hashes); nothing deleted.
