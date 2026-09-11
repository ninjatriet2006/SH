# IMG_SPLT GUI — canonical contract evidence

- Source of truth: `.opencode/plan-gui-architecture-phase-2.md`, A.1, A.3/A.3.1/A.3.2, A.4, A.5.2/A.5.4, A.6 and A.7.
- Envelope: schema version 1 `Req<T>`, `Res<T>`, typed `IpcError`; nullable Rust fields serialize as JSON `null`.
- Exact commands (9): `settings_load`, `settings_save`, `capabilities_check`, `scan_images`, `process_images`, `distribute`, `preferences_get`, `preferences_set`, `picker_select`.
- Exact job topics (4): `job.capabilities_check`, `job.scan_images`, `job.process_images`, `job.distribute`; request ID is retained, sequence is monotonic, exactly one terminal event is emitted and the frontend unlistens once.
- Picker: bridge-owned, parented, exact-pinned `rfd = 0.16.0`, one directory per successful input/output selection and per-window provenance; no webview dialog permission or drag/drop provenance.
- Security: capabilities are only event listen/unlisten; production CSP permits only required local IPC and Tauri asset origins; external paths require canonical picker provenance and containment; destructive output requires frontend confirmation.
- Runtime: no CWD, source-tree or cross-GUI lookup; no prompt, password, sudo or process exit; FFmpeg/FFprobe are discovered only through inherited PATH.
- Window/resources: `main`, `Image Splitter — GUI`, 900×650, resizable; app-local EN/VI, system/light/dark and system-default/dejavusans resources with exact two-entry font manifest.
- Persistence: app-data `preferences.json`, legacy import, ID validation, atomic replacement and `.bak`; defaults `en/dark/system-default`, valid selected assets survive restart.
- Release: `Image Splitter` / `img-splt-gui` / `com.sh.image-splitter`, updater disabled, shared workspace target; final Linux DEB/AppImage evidence is in `package.md` and `smoke.md`; MSI/DMG deferred/platform-unverified.
