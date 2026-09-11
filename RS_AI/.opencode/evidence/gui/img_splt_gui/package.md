# IMG_SPLT GUI — final 6.10 package evidence

- Status: **PASS** (2026-09-11); `cargo tauri build --bundles deb,appimage` completed from `GUI/img_splt_gui/bridge`.
- Gates: frontend 7/7 and 12 modules; backend 27/27; bridge 5/5; TUI 18/18; fmt/check/clippy PASS with 0 warnings.
- Binary: `/home/bimatkeo/Documents/SH/RS_AI/target/release/img-splt-gui` — 9,522,480 bytes — SHA-256 `7cb50ff8626ceb2f2ae0803c0b5dc8b255bc8ce8993998c16c6bcdcdb5fcf076`.
- DEB: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/deb/Image Splitter_0.1.0_amd64.deb` — 3,566,906 bytes — SHA-256 `6ca6dac46c8877179688e62a4c3d55e513985247391c1fd0a8f96bd586108382`.
- AppImage: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/appimage/Image Splitter_0.1.0_amd64.AppImage` — 79,890,936 bytes — SHA-256 `409a07d9966d229d8c7a5b4fff6e7139acf9c38ee94a60d32b9642f6bd0a2473`.
- Metadata: product `Image Splitter`; binary `img-splt-gui`; identifier `com.sh.image-splitter`; version `0.1.0`; DEB package `image-splitter`, architecture `amd64`; updater disabled (`createUpdaterArtifacts=false`, no updater dependency/plugin).
- Platform qualification: verified only on Linux `amd64` using DEB and AppImage; Windows MSI and macOS DMG were not built or verified.
- Resources: required payload 8/8 in both freshly extracted bundles is regular/no-symlink and byte-identical to app-owned source; font manifest 2/2 PASS.
- Fresh artifacts and `/tmp/opencode/img-splt-final-409a07d9-20260911` preserved; external-CWD runtime evidence is in `smoke.md`; nothing deleted.
