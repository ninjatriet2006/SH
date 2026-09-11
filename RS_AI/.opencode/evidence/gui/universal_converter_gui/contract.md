# Phase 5 — Universal Converter contract evidence

- Status: PASS (2026-09-10) against Phase 2 A.1/A.3.2/A.5.1/A.6/A.7.
- Exact registry: nine invokes — six jobs (`dependencies_check`, `classify_file`, `scan_directory`, `batch_convert`, `native_install`, `native_uninstall`) plus `preferences_get`, `preferences_set`, and bridge-owned `picker_select`.
- Exact events: only six `job.*` topics; schema version, nullable envelope fields, monotonic `seq`, discriminated terminal result, and exactly one terminal event are contract-tested.
- Error sets match A.6 per command; native commands include typed `Unavailable` for the Linux-only native contract, returned before job registration on other platforms.
- Picker provenance is explicit, per-window/per-kind, deny-by-default, and established only by successful picker selection; traversal, sibling, and direct/intermediate symlink escape tests PASS.
- Native requests require `confirmed:true`; Linux uses no-follow native operations, while macOS/Windows return pre-job `Unavailable`; no runtime parity is claimed for deferred MSI/DMG.
- Native install performs full filesystem/root preflight on a blocking worker before job registration, preserves request-ID reuse on failure, then rechecks under the per-window operation lock before mutation.
- Picker provenance remains explicit and window-scoped through bridge-owned exact-pinned `rfd`; no dialog plugin or webview dialog capability is present.
- Current gates: backend 33/33, bridge 36 PASS plus one intentionally ignored lock-child helper, frontend 19/19, build/fmt/check/clippy/audit/diff-check PASS. This contract refresh did not package; the final current-code package and smoke did, with results in [package evidence](package.md) and [smoke evidence](smoke.md).
