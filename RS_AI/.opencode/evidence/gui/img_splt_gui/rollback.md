# IMG_SPLT GUI — safe rollback drill

- Status: **PASS with documented baseline lint debt** (rerun 2026-09-11). Immutable SHA `c24388b30c99323d7fc81032f0454e9e9f1fb705` is recoverable, buildable and testable without active-tree mutation.
- New preserved detached worktree: `/tmp/opencode/img-splt-rollback-c24388b-20260911-rerun-01`; `HEAD` is the exact baseline SHA and `git worktree list` records it as detached.
- Preserved command logs: `/tmp/opencode/img-splt-rollback-c24388b-20260911-rerun-01-logs/` (commands, exit codes, complete stdout/stderr, worktree lists and status snapshots).
- From detached `RS_AI`: `cargo check --package img_splt --package img_splt_gui` PASS; `cargo test --package img_splt --package img_splt_gui` PASS, 18 passed and 0 failed.
- Expected immutable-baseline debt: fmt FAIL at `TUI/img_splt/src/lib.rs:1`; strict clippy FAIL with 3 warnings-as-errors at `GUI/img_splt_gui/src/main.rs:92,158,213` (`derivable_impls`, two `collapsible_if`).
- Active status SHA-256 immediately before/after: `0294c1bc671767468962f764afd64a62b0fbb5d6ff2de65b97e055e00be6af82` (identical). Detached status is clean: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- No reset, restore, deletion, pruning, formatting write or worktree removal was performed; worktree and logs remain preserved.
