# IMG_SPLT GUI — Phase 6 baseline

- Immutable source checkpoint: `c24388b30c99323d7fc81032f0454e9e9f1fb705` (`main`). The active worktree was dirty, so this SHA is the rollback identity; 6.11 must verify it in a detached worktree.
- Scope: legacy `GUI/img_splt_gui` eframe shell plus `TUI/img_splt` domain implementation.
- Active-tree baseline `cargo fmt --check --package img_splt --package img_splt_gui`: PASS; the immutable detached SHA later reproduced a pre-existing module-order fmt finding, recorded under Phase 6.11.
- `cargo check --package img_splt --package img_splt_gui`: PASS.
- `cargo test --package img_splt --package img_splt_gui`: PASS, 18 passed, 0 failed, 0 ignored (GUI has no tests).
- `cargo clippy --package img_splt --package img_splt_gui --all-targets -- -D warnings`: legacy FAIL with three GUI findings: `derivable_impls` at `src/main.rs:92`, `collapsible_if` at `:158` and `:213`; TUI emitted none.
- Baseline package/smoke: not available; legacy app is eframe, not Tauri. Phase 6 creates the standalone Tauri artifact.
- Existing app-local `langs/`, `themes/`, and `fonts/`: absent. Phase 6 must create physical local resources under the IMG_SPLT project.

## Legacy behavior

- Window: `Image Splitter — GUI`, 900×650, resizable; Dashboard, Settings, Scan Images, Environment tabs and bottom log.
- Preferences: eframe keys `language`, `theme`, `font`; defaults `en`, `dark`, system default. Existing font choices are host absolute paths.
- Settings: reads `settings.yaml` from process CWD; defaults are balanced distribution, 80 files/folder, 5 fixed folders, 5 retries, upscale 600→1280.
- Capability flow: invokes `ffmpeg`/`ffprobe`; legacy TUI may prompt, run password-fed `sudo apt`, and exit. Those behaviors are forbidden in the new backend/bridge.
- Image flow: scan supported images, probe width, process/upscale with FFmpeg, then distribute using balanced/greedy/fixed modes and collision suffixes.
- Native risks to remove: process-wide CWD mutation, interactive terminal/install behavior, destructive swap before all moves succeed, unchecked filesystem errors/panics, and uncontained user paths.

## Acceptance source

Canonical migration contract is `.opencode/plan-gui-architecture-phase-2.md`, sections A.1, A.3/A.3.1/A.3.2, A.4, A.5.2/A.5.4, A.6 and A.7. Detailed execution is `.opencode/plan-gui-architecture-phase-6.md`.

- Safe rollback details are recorded separately in `rollback.md`.
