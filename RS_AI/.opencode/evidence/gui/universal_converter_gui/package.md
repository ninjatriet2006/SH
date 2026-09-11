# Universal Converter — final package evidence

- Status: PASS (2026-09-10); `cargo tauri build --bundles deb,appimage` completed from `bridge` using workspace `/home/bimatkeo/Documents/SH/RS_AI/target` only; bridge-local `target` absent.
- Binary: `target/release/universal-converter-gui` — 9,587,888 bytes — SHA-256 `c3002c2df2f364980ea98a322f7b59e5eea052932260812e9e3066345ef8953d`.
- DEB: `target/release/bundle/deb/Universal Converter_0.1.0_amd64.deb` — 3,576,356 bytes — SHA-256 `d4c8696fd50fffe4c09e9eb2bc55cbbe15b6990e1dfda3b2420e2f15fd7e61f2`.
- AppImage: `target/release/bundle/appimage/Universal Converter_0.1.0_amd64.AppImage` — 79,890,936 bytes — SHA-256 `5c2eba9f52e774d1fc045b3782466cb2de4f6d9f7bd1601278089de9e4368045`.
- Metadata: product `Universal Converter`; binary `universal-converter-gui`; identifier `com.sh.universal-converter`; version `0.1.0`; DEB package `universal-converter`, architecture `amd64`.
- Resources: exact 8/8 payload files verified in DEB and AppDir: langs 2, themes 3, fonts 3; font manifest 2/2 hashes PASS.
- Updater: disabled (`createUpdaterArtifacts:false`), plugins empty, no updater entry in workspace lockfile.
- Final release deliverables: 3 (binary 1, DEB 1, AppImage 1); MSI/DMG deferred; no artifacts deleted.
