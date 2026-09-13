mod commands;
pub mod contract;
mod preferences;
mod resources;
pub mod security;

use commands::BridgeState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    debug_assert_eq!(commands::INVOKE_REGISTRY.len(), 10);
    debug_assert_eq!(commands::TOPICS.len(), 5);
    tauri::Builder::default()
        .setup(|app| {
            app.manage(BridgeState::default());
            let script = app.path().resource_dir().map_or_else(
                |_| resources::fallback_initialization_script(),
                |resource_root| resources::initialization_script(&resource_root),
            );
            let app_data = app.path().app_data_dir()?;
            preferences::load_or_migrate(
                &app_data.join("preferences.json"),
                preferences::legacy_path().as_deref(),
            )
            .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            let window = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or("main window configuration is missing")?;
            tauri::WebviewWindowBuilder::from_config(app, &window)?
                .initialization_script(script)
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                window.state::<BridgeState>().clear_window(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::config_load,
            commands::config_save,
            commands::scan_apps,
            commands::detect_app,
            commands::start_app,
            commands::stop_app,
            commands::search_apps,
            commands::preferences_get,
            commands::preferences_set,
            commands::picker_select,
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::commands::{EventSequence, INVOKE_REGISTRY, TOPICS};
    use super::contract::*;
    use super::resources;
    use super::security::PickerProvenance;
    use std::fs;
    use std::path::Path;

    #[test]
    fn exact_metadata_capability_and_local_resources() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).expect("config");
        assert_eq!(config["productName"], "Universe Manager");
        assert_eq!(config["identifier"], "com.sh.universe-manager");
        assert_eq!(config["app"]["windows"][0]["label"], "main");
        assert_eq!(config["app"]["windows"][0]["title"], "Universe Manager — GUI");
        assert_eq!(config["app"]["windows"][0]["width"], 1000);
        assert_eq!(config["app"]["windows"][0]["height"], 680);
        assert_eq!(config["app"]["windows"][0]["create"], false);
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/main.json")).expect("capability");
        assert_eq!(capability["windows"], serde_json::json!(["main"]));
        assert_eq!(
            capability["permissions"],
            serde_json::json!(["core:event:allow-listen", "core:event:allow-unlisten"])
        );
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("app root");
        assert!(resources::initialization_script(root).contains("DejaVuSans.ttf"));
    }

    #[test]
    fn registry_and_topics_are_exact() {
        assert_eq!(
            INVOKE_REGISTRY.map(|item| item.name),
            [
                "config_load",
                "config_save",
                "scan_apps",
                "detect_app",
                "start_app",
                "stop_app",
                "search_apps",
                "preferences_get",
                "preferences_set",
                "picker_select",
            ]
        );
        assert_eq!(
            TOPICS,
            [
                "job.scan_apps",
                "job.detect_app",
                "job.start_app",
                "job.stop_app",
                "job.search_apps",
            ]
        );
        assert_eq!(INVOKE_REGISTRY.iter().filter(|item| item.topic.is_some()).count(), 5);
    }

    #[test]
    fn envelope_requires_nullable_fields_and_has_typed_error() {
        let request = Req {
            schema_version: 1,
            request_id: None,
            payload: Empty {},
        };
        assert_eq!(
            serde_json::to_value(request).expect("serialize"),
            serde_json::json!({
                "schema_version": 1, "request_id": null, "payload": {}
            })
        );
        assert!(serde_json::from_value::<Req<Empty>>(serde_json::json!({"schema_version":1,"payload":{}})).is_err());
        let failure = IpcError {
            code: IpcErrorCode::Forbidden,
            message: "denied".into(),
            retryable: false,
            details: None,
        };
        assert_eq!(serde_json::to_value(failure).expect("serialize")["code"], "forbidden");
    }

    #[test]
    fn picker_provenance_is_per_window_and_kind_and_payload_grants_nothing() {
        let root = std::env::temp_dir().join(format!("universe-bridge-{}", std::process::id()));
        fs::create_dir_all(&root).expect("root");
        let child = root.join("app");
        fs::create_dir_all(&child).expect("child");
        let provenance = PickerProvenance::default();
        assert!(
            provenance
                .resolve_existing("main", UniversePickerKind::Managed, &child)
                .is_err()
        );
        provenance
            .replace_directory("main", UniversePickerKind::Managed, &root)
            .expect("picker grant");
        assert!(
            provenance
                .resolve_existing("main", UniversePickerKind::Managed, &child)
                .is_ok()
        );
        assert!(
            provenance
                .resolve_existing("other", UniversePickerKind::Managed, &child)
                .is_err()
        );
        assert!(
            provenance
                .resolve_existing("main", UniversePickerKind::Source, &child)
                .is_err()
        );
        provenance.clear_window("main").expect("clear window provenance");
        assert!(
            provenance
                .resolve_existing("main", UniversePickerKind::Managed, &child)
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn picker_canonical_recheck_denies_symlink_and_replaced_path_escape() {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir().join(format!("universe-bridge-recheck-{}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        let candidate = root.join("candidate");
        fs::create_dir_all(&candidate).expect("candidate");
        let provenance = PickerProvenance::default();
        provenance
            .replace_directory("main", UniversePickerKind::Source, &root)
            .expect("picker grant");
        assert!(
            provenance
                .resolve_existing("main", UniversePickerKind::Source, &candidate)
                .is_ok()
        );
        fs::remove_dir(&candidate).expect("replace candidate");
        symlink(&outside, &candidate).expect("escape symlink");
        let error = provenance
            .resolve_existing("main", UniversePickerKind::Source, &candidate)
            .expect_err("open-time canonical recheck must deny replacement");
        assert_eq!(error.code, IpcErrorCode::Forbidden);
    }

    #[test]
    fn sequence_is_monotonic_and_allows_one_terminal_event() {
        let mut sequence = EventSequence::<Empty>::new("req-7".into());
        let started = sequence
            .event(JobState::Started, None, None, None, None)
            .expect("started");
        let progress = sequence
            .event(JobState::Progress, Some(1), Some(2), None, None)
            .expect("progress");
        let terminal = sequence
            .event(
                JobState::Completed,
                None,
                None,
                None,
                Some(JobResult::Completed {
                    request_id: "req-7".into(),
                    value: Empty {},
                }),
            )
            .expect("terminal");
        assert_eq!([started.seq, progress.seq, terminal.seq], [0, 1, 2]);
        assert_eq!(terminal.job_id, "req-7");
        assert!(
            sequence
                .event(
                    JobState::Failed,
                    None,
                    None,
                    None,
                    Some(JobResult::Failed {
                        request_id: "req-7".into(),
                        error: IpcError {
                            code: IpcErrorCode::Internal,
                            message: "late".into(),
                            retryable: false,
                            details: None,
                        },
                    })
                )
                .is_err()
        );
    }

    #[test]
    fn action_request_has_no_path_field_and_requires_confirmation_field() {
        let value = serde_json::to_value(AppActionRequest {
            app_id: "demo".into(),
            confirmed: true,
        })
        .expect("serialize");
        assert_eq!(value, serde_json::json!({"app_id":"demo","confirmed":true}));
        assert!(value.get("path").is_none());
        assert!(serde_json::from_value::<AppActionRequest>(serde_json::json!({"app_id":"demo"})).is_err());
        assert!(
            serde_json::from_value::<AppActionRequest>(serde_json::json!({
                "app_id":"demo", "confirmed":true, "root":"/forged"
            }))
            .is_ok(),
            "unknown payload fields must not grant authority"
        );
    }

    #[test]
    fn config_and_preferences_storage_are_separate_by_contract() {
        let commands = include_str!("commands.rs");
        assert!(commands.contains("app_config_dir(&app)"));
        assert!(commands.contains("app_data_dir(app)?.join(\"preferences.json\")"));
        assert!(!commands.contains("app_config_dir(app)?.join(\"preferences.json\")"));
    }
}
