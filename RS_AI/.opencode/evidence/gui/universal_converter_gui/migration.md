# Phase 5.2 - `universal_converter_gui` migration inventory

## Scope and baseline

- Read-only inventory against immutable Phase 5.1 SHA `c24388b30c99323d7fc81032f0454e9e9f1fb705`; application source was not modified.
- Current app surface is only `GUI/universal_converter_gui/{Cargo.toml,src/main.rs}`. No `bridge/`, `frontend/`, `langs/`, `themes/`, or `fonts/` exists yet.
- Target is the Phase 2 three-layer standalone Tauri v2 contract: frontend owns UI state/forms, bridge owns exact IPC/events/security/persistence, and backend owns conversion/domain behavior.

## State and workflow parity map

| Existing egui state/behavior | Current source | Required migration owner and parity acceptance |
|---|---|---|
| Navigation `selected_tab`: Dashboard, Dependencies, Classify File, Scan Directory, Settings | `src/main.rs:48-86,279-307` | Frontend state; preserve all five views and default Dashboard. Dashboard must continue reflecting operation status. |
| Dependency state `deps_status`, `deps_result`, `deps_is_ok`; check button | `src/main.rs:54-57,434-497` | Frontend presents typed `DependencyReport`; bridge invokes exact `dependencies_check` with `Req<Empty>` and emits `job.dependencies_check`. Preserve missing-tool reporting for ffmpeg, 7z, and soffice from `TUI/universal_converter/src/system/dependencies.rs:10-29`. |
| Classification input/status/result | `src/main.rs:59-62,503-547` | Frontend form plus explicit picker provenance; bridge exact `classify_file`/`job.classify_file`. Preserve extension classification categories, including Directory/Unknown, while returning authoritative A.6 `Classification {path,file_type,size_bytes}` rather than debug JSON. |
| Directory scan input/status/result/count | `src/main.rs:64-68,553-612` | Frontend form plus explicit picker provenance; bridge exact `scan_directory`/`job.scan_directory`. Preserve non-recursive scan and hidden/junk filtering. Correct current boolean-order mismatch: GUI supplies `[video,audio,image,document,archive,...]`, while scanner indexes `[archive,video,image,audio,document]` (`scanner.rs:81-89`); migrate to exact `allowed_types:String[]`. |
| Bottom log and Clear action | `src/main.rs:70,309-325,358-385` | Frontend transient state; retain user-visible operation/error history and clear behavior. Typed `IpcError` replaces string-only errors. |
| Persistent worker channel and per-action threads/Tokio runtime | `src/main.rs:73-75,97,358-385,481-496,541-546,595-611` | Replace with bridge jobs and exact A.7 topics. Frontend listens once per active job and calls returned `UnlistenFn` exactly once at terminal/disposal/window close; exactly one terminal event and increasing `seq`. |
| Preferences `language`, `theme`, `font`; immediate apply | `src/main.rs:8-35,338-356,614-683` | Frontend presentation plus bridge persistence. Map enum values to exact IDs `vi|en`, `system|light|dark`, and `font_id`; default `vi/system/system-default`. Preserve immediate visual application, validate IDs, and safely normalize invalid/missing/corrupt assets. |
| Roadmap parity absent from current egui screens | Phase 2 A.5-A.7 | Add exact `batch_convert`, `native_install`, `native_uninstall` flows and topics. Batch must expose files, output directory, format, overwrite, progress/result; native operations require explicit UI confirmation before `confirmed:true`. |

## Storage, window, and IPC contract map

| Concern | Existing behavior | Locked migration target |
|---|---|---|
| Persistence | eframe storage key `preferences`, loaded at construction and saved through `eframe::App::save` (`src/main.rs:6,96-101,328-330`) | `app_data_dir()/preferences.json`; one-time import of eframe key `preferences`; validate IDs; atomic temp-file plus rename; retain previous `preferences.json.bak`. Commands are exact `preferences_get` and `preferences_set`. |
| Window | Native viewport title `Universal Converter - GUI`, inner size `900x650`; default resizable (`src/main.rs:240-252`) | Tauri window label `main`, title `Universal Converter — GUI`, `900x650`, resizable. Product/binary/identifier: `Universal Converter` / `universal-converter-gui` / `com.sh.universal-converter`. |
| IPC envelope | Direct in-process calls, ad-hoc JSON strings, string errors | Every exact A.6 command takes `Req<T>` and returns `IpcResult<T>` using A.1 schema version 1, nullable-but-present request ID/details, exact DTO fields, and typed allowed error codes. Job command request IDs originate in frontend and are not regenerated. |
| Capability/events | None (egui process) | Sole `bridge/capabilities/main.json`: identifier `main`, windows exactly `["main"]`, permissions exactly event listen/unlisten. Bridge-owned exact-pinned `rfd` provides the window-parented picker without a Tauri plugin or webview dialog capability. Register only A.6 commands; emit only six Universal Converter A.7 job topics and no non-job event. |

## Native and path-security map

- `dependencies_check` may only resolve executables through inherited `PATH`; it must not call `prompt_install`, prompt on stdin, invoke `sudo`, or exit. The upstream helpers currently contain all three hazards (`dependencies.rs:41-90`, `permissions.rs:3-33`).
- `classify_file` and `scan_directory` accept only canonical roots selected through an explicit frontend picker and recorded in bridge state for window `main`. Empty root sets deny; arbitrary payloads cannot establish trust.
- `batch_convert` reads only selected input roots and writes only selected output roots. For every external path: require absolute, canonicalize target or parent, canonicalize root, require containment, re-check after open/rename, deny resolution/traversal/symlink escape. Existing dispatcher derives outputs beside inputs and falls back to `Path::new(".")` (`dispatcher.rs:103-219`), so it cannot be exposed directly.
- On Linux, `native_install` reads selected artifact roots and writes only `$HOME/.local/{bin,share/applications}`; `native_uninstall` removes only a recorded installation there. Other platforms return typed `Unavailable` before job registration. Both require `confirmed:true` on the supported path and never prompt, elevate, depend on CWD, or exit.
- Upstream watchdog installs process-wide hooks that call `process::exit` and sends force-kill commands (`watchdog.rs:11-24,39-54`); migration must use cancellable job-local lifecycle instead of initializing this behavior in the Tauri bridge.

## Asset and localization map

- Current localization is inline `tr(en, vi)` and several labels/status strings remain hard-coded English or Vietnamese (`src/main.rs:264-325,358-426`). Extract every visible UI key recursively into app-owned `langs/en.json` and `langs/vi.json`; acceptance is identical complete key sets and runtime language switching.
- Current theme is egui System/Light/Dark with immediate `ctx.set_theme`; create app-owned themes with all required tokens and preserve `system`, `light`, and `dark` behavior plus safe fallback.
- Current font discovery and fallback read OS-specific absolute paths (`src/main.rs:136-233`), so selection varies by host and is not packageable. App must own physical, non-symlink `fonts/DejaVuSans.ttf`, `fonts/LICENSE.txt`, and `fonts/manifest.sha256`; IDs are `dejavusans` and system sentinel `system-default`. Required hashes are `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280` for the font and `63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9` for the license.
- Bundle mappings are exactly `../langs/ -> langs/`, `../themes/ -> themes/`, and the three font resources `../fonts/DejaVuSans.ttf -> fonts/DejaVuSans.ttf`, `../fonts/LICENSE.txt -> fonts/LICENSE.txt`, and `../fonts/manifest.sha256 -> fonts/manifest.sha256`. Loader resolves validated relative IDs only through Tauri `$RESOURCE/{langs,themes,fonts}/`, checks containment, records a relative-path SHA-256 manifest, and falls back safely for missing/corrupt JSON/fonts without source-path, CWD, cross-GUI, or system-font lookup.

## Risks and required migration checks

1. **Behavior gap:** current GUI exposes dependency/classify/scan only; Phase 2 additionally requires batch and native flows. Do not report parity until their typed jobs, cancellation, progress, overwrite/conflict behavior, and confirmation UI are tested.
2. **Contract mismatch:** existing scanner DTO lacks `size_bytes`, scan filter ordering is inconsistent, calls serialize implementation structs to strings, and errors do not update panel statuses. A.6 DTO/error mapping needs explicit tests, not direct serialization leakage.
3. **Security risk:** upstream domain modules include prompt/sudo/admin/process-exit, CWD fallbacks, derived write paths, and unmanaged deletion. Keep these out of bridge call paths or refactor behind contained, explicit-path, cancellable APIs.
4. **Persistence risk:** eframe storage location/format is framework-managed. One-time import must be demonstrated without overwriting a valid canonical file; corrupt/unknown IDs normalize to defaults while retaining backup and atomic-write guarantees.
5. **Asset risk:** there are currently no local assets or licenses, and inline strings make EN/VI coverage incomplete. Packaging/smoke must prove physical files, exact hashes, manifest/loader containment, runtime apply/persist, and missing/corrupt fallback from outside the source CWD.
6. **Release risk:** bridge/frontend/config/capability files and output metadata do not exist. Updater must remain disabled, CSP/scope must match A.3 exactly, Linux deb/AppImage must be verified, and MSI/DMG remain `deferred/platform-unverified` until native builds.

## Evidence sources

- `.opencode/evidence/gui/universal_converter_gui/baseline.md`
- `.opencode/plan-gui-architecture-phase-5.md`
- `.opencode/plan-gui-architecture-phase-2.md` sections A.1, A.3-A.7
- `GUI/universal_converter_gui/Cargo.toml`; `GUI/universal_converter_gui/src/main.rs`
- `TUI/universal_converter/src/{core/{scanner.rs,dispatcher.rs,watchdog.rs},system/{dependencies.rs,context_menu.rs,permissions.rs}}`

## Phase 5.6 asset acceptance

- App-local physical assets now contain 2 recursively parity-checked language dictionaries (56 leaf keys each), 3 themes (`system`, `light`, `dark`), and the A.3.1 DejaVu font/license/manifest set.
- Exact SHA-256: `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  DejaVuSans.ttf`; `63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9  LICENSE.txt`.
- The bridge resolves only fixed relative IDs beneath Tauri's canonical resource root, rejects traversal/symlinks, verifies font hashes/manifest, and injects only contained bundle paths. Missing/corrupt assets normalize to `vi/system/system-default`; the app resource loader has no CWD, source-tree, cross-app, or host-font fallback. The approved `system-default` sentinel may still cause host GTK/fontconfig stack reads.
- Preferences remain atomically persisted and eframe-imported through the existing bridge migration; frontend applies language/theme/font immediately and calls `preferences_set` during startup whenever asset loading normalizes a fallback.
- Bundle mappings are exactly `../langs/ -> langs/`, `../themes/ -> themes/`, and the three font resources `../fonts/DejaVuSans.ttf -> fonts/DejaVuSans.ttf`, `../fonts/LICENSE.txt -> fonts/LICENSE.txt`, and `../fonts/manifest.sha256 -> fonts/manifest.sha256`.
