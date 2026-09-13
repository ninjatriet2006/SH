# IMG_SPLT GUI — migration map

| Legacy concern | Tauri destination | Required parity/safety |
|---|---|---|
| eframe window and tabs | `frontend/` + bridge `main` window | 900×650, resizable, same primary flows |
| TUI settings/domain | `backend/` | typed settings and domain behavior without UI/Tauri dependency |
| mpsc workers | bridge jobs + four A.7 topics | exact request ID, monotonic sequence and one terminal event |
| CWD path text | bridge-owned `picker_select` | one canonical directory per window/kind; no CWD or drag/drop provenance |
| FFmpeg/FFprobe check | `capabilities_check` | inherited PATH only; report availability/version; never prompt/sudo/exit |
| scan/process/distribute | exact A.6 commands | containment, cancellation, typed errors and confirmation before destructive output |
| eframe preferences | app-data `preferences.json` | one-time import, ID validation, atomic replace, `.bak`, restart persistence |
| host fonts/inline strings | local `langs/themes/fonts` | EN/VI, theme/font fallback, exact DejaVu manifest hashes |
| no package | Tauri v2 bridge | shared workspace target, updater disabled, DEB/AppImage and external-CWD smoke |

The old egui source remains preserved for rollback but is excluded from the workspace; only the Tauri `Image Splitter` release entry is selectable. No user file is deleted by this map.
