# IMG_SPLT GUI — final 6.10 package evidence

- Status: **PASS** (2026-09-11); `cargo tauri build --bundles deb,appimage` completed from `GUI/img_splt_gui/bridge`.
- Gates: frontend 7/7 and 12 modules; backend 30/30; bridge 6/6; TUI 18/18; fmt/check/clippy PASS with 0 warnings.
- Binary: `/home/bimatkeo/Documents/SH/RS_AI/target/release/img-splt-gui` — 9,535,536 bytes — SHA-256 `688418fd618715c03b1197cf375f9f6a2ca68aa03e2a5daa6194b9f86f310063`.
- DEB: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/deb/Image Splitter_0.1.0_amd64.deb` — 3,573,546 bytes — SHA-256 `727583579fb58be886d76abf53ae71a78220e56eae5d397c9b96a27d82b0a1a5`.
- AppImage: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/appimage/Image Splitter_0.1.0_amd64.AppImage` — 79,895,032 bytes — SHA-256 `039b2ce60b7d5b71e2c4df67ad29a6a4e365dc2736f000d9087a9d79bd4a4976`.
- Metadata: product `Image Splitter`; binary `img-splt-gui`; identifier `com.sh.image-splitter`; version `0.1.0`; DEB package `image-splitter`, architecture `amd64`; updater disabled (`createUpdaterArtifacts=false`, no updater dependency/plugin).
- Platform qualification: verified only on Linux `amd64` using DEB and AppImage; Windows MSI and macOS DMG were not built or verified.
- Resources: required payload 8/8 in both freshly extracted bundles is regular/no-symlink and byte-identical to app-owned source; font manifest 2/2 PASS.
- Fresh artifacts and `/tmp/opencode/img-splt-final-039b2ce6-20260911-1059` preserved; external-CWD runtime evidence is in `smoke.md`; nothing deleted.
- Phase 8.4 attempt (2026-09-12): no fresh artifact claimed. Bridge build fails workspace discovery because excluded `GUI/img_splt_gui/Cargo.toml` inherits workspace fields without a root; root Tauri with bridge config selected the wrong Universal binary, so that output was rejected.
- Post-four-fix Phase 8.4 rebuild: workspace discovery fixed; fresh AppImage `fd7b24f30f05cf1678da3d32df980db5d80687425c935308f44937506c90b3a4` and DEB `5b842ae968309646571d823ac75d00cce4b871888f621724b051babfaa47293c` passed metadata/resource checks and are preserved under `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Reviewer F1 refresh (2026-09-13): fresh AppImage `5de40ce0a35c236e295c7b8e0d198eb4419ec51a8f8ad549acac258c7c5159e4` and DEB `600abce87be711ea0c9af3fa8cf52454d428dce9d40fb9279823e6e09b8ce049`; metadata `image-splitter`/`0.1.0`/`amd64`; durable evidence under `/tmp/opencode/phase8-f1-five-20260913T000000Z/img-splt`.
