# Universal Converter — final package evidence

- Status: PASS (2026-09-10); `cargo tauri build --bundles deb,appimage` completed from `bridge` using workspace `/home/bimatkeo/Documents/SH/RS_AI/target` only; bridge-local `target` absent.
- Binary: `target/release/universal-converter-gui` — 9,587,888 bytes — SHA-256 `c3002c2df2f364980ea98a322f7b59e5eea052932260812e9e3066345ef8953d`.
- DEB: `target/release/bundle/deb/Universal Converter_0.1.0_amd64.deb` — 3,576,356 bytes — SHA-256 `d4c8696fd50fffe4c09e9eb2bc55cbbe15b6990e1dfda3b2420e2f15fd7e61f2`.
- AppImage: `target/release/bundle/appimage/Universal Converter_0.1.0_amd64.AppImage` — 79,890,936 bytes — SHA-256 `5c2eba9f52e774d1fc045b3782466cb2de4f6d9f7bd1601278089de9e4368045`.
- Metadata: product `Universal Converter`; binary `universal-converter-gui`; identifier `com.sh.universal-converter`; version `0.1.0`; DEB package `universal-converter`, architecture `amd64`.
- Resources: exact 8/8 payload files verified in DEB and AppDir: langs 2, themes 3, fonts 3; font manifest 2/2 hashes PASS.
- Updater: disabled (`createUpdaterArtifacts:false`), plugins empty, no updater entry in workspace lockfile.
- Final release deliverables: 3 (binary 1, DEB 1, AppImage 1); MSI/DMG deferred; no artifacts deleted.
- Phase 8.4 refresh (2026-09-12): preserved AppImage `d0bfcfcb59966c29c4bec5e4cf0120b644f34826fe6db074dca8064ad3a8369b`, DEB `6accd06678129f7bb5ad5c70d728c45132e93603a84e478c8339e580b17e6aed` under `/tmp/opencode/phase-8-artifacts-20260912`.
- Post-four-fix Phase 8.4 rebuild: AppImage `5d7dc2fe43500e3fa7cff3afa05bad1af571236fecafa37fc9970beee151c0c1`; DEB `7f639a9e9d4ecc4e37f60b8e71670846e2e17c00e2d08941b785725d2d7321d1`; preserved under `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Reviewer F1 refresh (2026-09-13): fresh AppImage `2e5189685a7d8aca7c3fc60aba96c508610ba8b930b5af749374d01102be024c` and DEB `9d3b5e1e4ad99f2785c1f9d352722b0bb875d807fa57f7489f10911d6593496b`; metadata `universal-converter`/`0.1.0`/`amd64`; durable evidence under `/tmp/opencode/phase8-f1-five-20260913T000000Z/universal`.
