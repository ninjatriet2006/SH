# Archived egui rollback checkpoint

`Cargo.toml` and `src/` in this directory are retained legacy egui sources. The root workspace
excludes that package, so normal workspace builds do not compile it. These files are not an immutable
checkpoint; rollback comparison uses the detached worktree at the Phase 5.1 SHA. The active app is the app-owned
`frontend/`, `backend/`, and `bridge/` Tauri implementation in this directory.

This archive is not release evidence and must not be described as a currently built package.
