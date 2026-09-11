mod commands;
pub mod contract;
pub mod security;

use commands::BridgeState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    debug_assert_eq!(
        commands::INVOKE_REGISTRY.len(),
        9,
        "invoke handler and command contract registry must stay in sync"
    );
    tauri::Builder::default()
        .setup(|app| {
            let state = BridgeState::default();
            #[cfg(target_os = "linux")]
            {
                let records_path = commands::native_records_path(app.handle())
                    .map_err(|error| -> Box<dyn std::error::Error> { error.message.into() })?;
                state
                    .initialize_native_records(records_path)
                    .map_err(|error| -> Box<dyn std::error::Error> { error.message.into() })?;
            }
            app.manage(state);
            let root = app.path().resource_dir()?;
            let script = commands::resource_initialization_script(&root)
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            app.webview_windows()
                .values()
                .try_for_each(|window| window.eval(&script))?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let state = window.state::<BridgeState>();
            if let WindowEvent::Destroyed = event {
                state.clear_window(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::dependencies_check,
            commands::classify_file,
            commands::scan_directory,
            commands::batch_convert,
            commands::native_install,
            commands::native_uninstall,
            commands::preferences_get,
            commands::preferences_set,
            commands::picker_select,
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::contract::*;
    use super::security::{PickerKind, PickerProvenance};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    struct TestTree(PathBuf);

    impl TestTree {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "universal-bridge-{name}-{}-{}",
                std::process::id(),
                TEST_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("test root");
            Self(path)
        }
    }

    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn packaged_json_fetch_origins_are_allowed_by_csp() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid Tauri configuration");
        let csp = config["app"]["security"]["csp"].as_str().expect("CSP string");
        let connect_src = csp
            .split(';')
            .map(str::trim)
            .find(|directive| directive.starts_with("connect-src "))
            .expect("connect-src directive");

        assert_eq!(
            connect_src,
            "connect-src 'self' ipc: http://ipc.localhost asset: http://asset.localhost"
        );
    }

    #[test]
    fn envelope_serializes_nullable_fields_and_exact_error_code() {
        let request = Req {
            schema_version: 1,
            request_id: None,
            payload: Empty {},
        };
        assert_eq!(
            serde_json::to_value(request).expect("serialize"),
            json!({"schema_version": 1, "request_id": null, "payload": {}})
        );
        let error = IpcError {
            code: IpcErrorCode::InvalidArgument,
            message: "bad".to_owned(),
            retryable: false,
            details: None,
        };
        assert_eq!(
            serde_json::to_value(error).expect("serialize"),
            json!({"code":"invalid_argument","message":"bad","retryable":false,"details":null})
        );
    }

    #[test]
    fn envelope_rejects_missing_request_id_but_accepts_present_null() {
        assert!(
            serde_json::from_value::<Req<Empty>>(json!({
                "schema_version": 1,
                "payload": {}
            }))
            .is_err()
        );
        assert_eq!(
            serde_json::from_value::<Req<Empty>>(json!({
                "schema_version": 1,
                "request_id": null,
                "payload": {}
            }))
            .expect("present nullable request_id")
            .request_id,
            None
        );
    }

    #[test]
    fn every_a1_a7_nullable_field_rejects_missing_but_accepts_null() {
        let response = json!({"schema_version":1,"request_id":null,"data":{}});
        assert!(serde_json::from_value::<Res<Empty>>(response.clone()).is_ok());
        let mut missing_response = response;
        missing_response.as_object_mut().expect("object").remove("request_id");
        assert!(serde_json::from_value::<Res<Empty>>(missing_response).is_err());

        let error = json!({"code":"internal","message":"fixture","retryable":false,"details":null});
        assert!(serde_json::from_value::<IpcError>(error.clone()).is_ok());
        let mut missing_details = error;
        missing_details.as_object_mut().expect("object").remove("details");
        assert!(serde_json::from_value::<IpcError>(missing_details).is_err());

        let progress = json!({
            "request_id":"req-1","completed":null,"total":null,"message":null,"result":null
        });
        assert!(serde_json::from_value::<JobProgress<Empty>>(progress.clone()).is_ok());
        for field in ["completed", "total", "message", "result"] {
            let mut missing = progress.clone();
            missing.as_object_mut().expect("object").remove(field);
            assert!(
                serde_json::from_value::<JobProgress<Empty>>(missing).is_err(),
                "missing {field} must fail"
            );
        }
    }

    #[test]
    fn job_result_and_progress_match_discriminated_contract() {
        let event = JobEvent {
            schema_version: 1,
            job_id: "req-1".to_owned(),
            seq: 2,
            state: JobState::Completed,
            payload: JobProgress {
                request_id: "req-1".to_owned(),
                completed: None,
                total: None,
                message: None,
                result: Some(JobResult::Completed {
                    request_id: "req-1".to_owned(),
                    value: DependencyReport {
                        is_ok: true,
                        missing: vec![],
                    },
                }),
            },
        };
        let value = serde_json::to_value(event).expect("serialize");
        assert_eq!(value["state"], "completed");
        assert_eq!(value["payload"]["result"]["status"], "completed");
        assert!(value["payload"]["result"].get("error").is_none());
    }

    #[test]
    fn picker_provenance_is_per_window_and_empty_roots_deny() {
        let tree = TestTree::new("windows");
        let file = tree.0.join("input.txt");
        fs::write(&file, b"data").expect("fixture");
        let provenance = PickerProvenance::default();

        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &file)
                .expect_err("empty roots deny")
                .code,
            IpcErrorCode::Forbidden
        );
        provenance
            .record_paths("main", PickerKind::Input, std::slice::from_ref(&file))
            .expect("native picker record");
        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &file)
                .expect("same window"),
            fs::canonicalize(&file).expect("canonical")
        );
        assert_eq!(
            provenance
                .resolve_existing("other", PickerKind::Input, &file)
                .expect_err("cross-window provenance denied")
                .code,
            IpcErrorCode::Forbidden
        );
        for kind in [PickerKind::Output, PickerKind::Artifact] {
            assert_eq!(
                provenance
                    .resolve_existing("main", kind, &file)
                    .expect_err("picker kinds must not grant each other")
                    .code,
                IpcErrorCode::Forbidden
            );
        }
    }

    #[test]
    fn picker_provenance_allows_only_the_exact_selected_root_or_its_descendants() {
        let tree = TestTree::new("exact-roots");
        let selected = tree.0.join("selected");
        let child = selected.join("child.txt");
        let sibling = tree.0.join("sibling.txt");
        fs::create_dir(&selected).expect("selected directory");
        fs::write(&child, b"child").expect("child fixture");
        fs::write(&sibling, b"sibling").expect("sibling fixture");
        let provenance = PickerProvenance::default();

        provenance
            .replace_paths("main", PickerKind::Input, std::slice::from_ref(&selected))
            .expect("selected root");
        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &selected)
                .expect("exact root allowed"),
            fs::canonicalize(&selected).expect("canonical root")
        );
        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &child)
                .expect("descendant allowed"),
            fs::canonicalize(&child).expect("canonical child")
        );
        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &sibling)
                .expect_err("adjacent path denied")
                .code,
            IpcErrorCode::Forbidden
        );

        provenance
            .replace_paths("main", PickerKind::Input, std::slice::from_ref(&child))
            .expect("selected file root");
        assert_eq!(
            provenance
                .resolve_existing("main", PickerKind::Input, &sibling)
                .expect_err("a selected file must not grant its siblings")
                .code,
            IpcErrorCode::Forbidden
        );
    }

    #[cfg(unix)]
    #[test]
    fn containment_rejects_traversal_and_symlink_escape() {
        use std::os::unix::fs::symlink;

        let tree = TestTree::new("containment");
        let allowed = tree.0.join("allowed");
        let outside = tree.0.join("outside.txt");
        fs::create_dir(&allowed).expect("allowed");
        fs::write(&outside, b"outside").expect("outside");
        let link = allowed.join("escape.txt");
        symlink(&outside, &link).expect("symlink");
        let outside_directory = tree.0.join("outside-directory");
        fs::create_dir(&outside_directory).expect("outside directory");
        fs::write(outside_directory.join("nested.txt"), b"outside").expect("nested outside file");
        let intermediate_link = allowed.join("linked-directory");
        symlink(&outside_directory, &intermediate_link).expect("intermediate symlink");
        let provenance = PickerProvenance::default();
        provenance
            .record_paths("main", PickerKind::Input, std::slice::from_ref(&allowed))
            .expect("picker root");

        for path in [
            allowed.join("../outside.txt"),
            link,
            intermediate_link.join("nested.txt"),
        ] {
            assert_eq!(
                provenance
                    .resolve_existing("main", PickerKind::Input, &path)
                    .expect_err("escape denied")
                    .code,
                IpcErrorCode::Forbidden
            );
        }
    }
}
