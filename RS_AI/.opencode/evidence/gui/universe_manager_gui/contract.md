# Universe Manager GUI — Phase 7.4 contract

- Schema v1 uses nullable `request_id` envelopes and typed errors. The exact invoke registry is 10 commands: `config_load`, `config_save`, `scan_apps`, `detect_app`, `start_app`, `stop_app`, `search_apps`, `preferences_get`, `preferences_set`, and `picker_select`.
- The exact five job topics are `job.scan_apps`, `job.detect_app`, `job.start_app`, `job.stop_app`, and `job.search_apps`; jobs emit ordered lifecycle events and reject forged/reused identity.
- Picker provenance is window- and kind-bound; payloads cannot grant roots. Start/stop require `confirmed:true`; configuration, preferences, resource loading, and migration storage remain separated.
- Release contract: `Universe Manager` / `universe-manager-gui` / `com.sh.universe-manager`, updater disabled, local resources only. Current package and external-CWD proof are in [package.md](package.md) and [smoke.md](smoke.md); MSI/DMG are deferred/platform-unverified.
