# GUI standalone audit

- Root: `/home/bimatkeo/Documents/SH/RS_AI`
- GUI roots (7): `filen_gui, img_splt_gui, opencode_manager_gui, rclone_gui, subscription_manager_gui, universal_converter_gui, universe_manager_gui`
- Scope: `GUI/` Cargo/npm manifests, Rust/web runtime source, HTML/CSS/JSON/TOML/YAML config, and symlinks
- Workspace manifests: 5 (root plus app-owned)
- Excluded generated trees: `.git, dist, gen, node_modules, target`
- Command: `bash .opencode/audit-gui-standalone.sh`
- Scanned: 31 manifests, 448 runtime/config files, 144 symlinks
- Allowlist entries: 0

## PASS — no non-allowlisted GUI-to-GUI reference found

## Artifact and gate evidence

The [seven-row artifact matrix](evidence/gui/phase-8-artifact-matrix.md) records each app's frontend and Rust commands/results, fresh Linux AppImage/DEB hashes, package/resource checks, and isolated external-CWD smoke. Aggregate declared tests are **437 passed, 0 failed, 1 ignored**; all seven rows pass, with only 20 non-blocking Subscription frontend lint warnings.

Per-app package, smoke, and verification evidence: [Filen](evidence/gui/filen_gui/verify.md), [Rclone](evidence/gui/rclone_gui/verify.md), [OpenCode Manager](evidence/gui/opencode_manager_gui/verify.md), [Subscription](evidence/gui/subscription_manager_gui/verify.md), [Universal Converter](evidence/gui/universal_converter_gui/verify.md), [IMG_SPLT](evidence/gui/img_splt_gui/verify.md), and [Universe Manager](evidence/gui/universe_manager_gui/verify.md). Declared gates comprise `npm test` where present, frontend lint/build where declared, Rust `cargo fmt --check`, `cargo check`, `cargo test`, strict `cargo clippy -- -D warnings`, `cargo tauri build --bundles deb,appimage`, artifact inspection/hash verification, and dual external-CWD readiness smoke.

Linux DEB/AppImage evidence is complete. MSI and DMG builds remain `deferred/platform-unverified` because native Windows/macOS packaging was unavailable on Linux.

Checkpoint HEAD is `335a64c202611323b9c430e1468635751d1b891b`. The current worktree is intentionally dirty with 24 status entries/24 tracked modified paths after fresh evidence regeneration. Historical baseline `ede9423..HEAD` spans 220 paths and is separate from this uncommitted evidence delta. Current `gitnexus detect-changes --repo SH` reports no indexed symbol changes; direct artifact hashes, status, audit, and diff checks are authoritative. `git diff --check` passes.
