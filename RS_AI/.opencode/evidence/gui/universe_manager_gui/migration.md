# Universe Manager GUI — migration map

- Baseline SHA: `ede94236afbc78b5d53ef46bcf512470933d47c1`.
- Legacy shell: `GUI/universe_manager_gui/src/main.rs`; domain: `TUI/universe_manager/src/`.
- UI parity: Dashboard, Config, Scan, App Manager, Search and Settings tabs; async load, scan,
  detect, start, stop and search flows.
- Persistence: canonical config is `~/.config/universe-manager/config.json`; legacy preferences use
  platform config/APPDATA `preferences.conf` and require one-time migration to atomic app-data storage.
- Window parity: `Universe Manager — GUI`, 1000x680, resizable.
- Native risks: broad filesystem and `/proc` discovery, custom `sh -c`, heuristic kill/taskkill,
  non-atomic preferences and recursive host-font lookup. The migration must use stable app/process IDs,
  explicit confirmation, typed errors, local bundled resources and no CWD/source-tree fallback.
- Legacy baseline has no bundled icon/runtime assets and directly links `universe_manager`; Tauri output
  must own frontend/backend/bridge plus physical `langs`, `themes`, and `fonts`.
