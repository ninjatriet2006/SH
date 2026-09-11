mod commands;
pub mod contract;
pub mod security;

use commands::BridgeState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    debug_assert_eq!(commands::INVOKE_REGISTRY.len(), 9);
    debug_assert_eq!(commands::TOPICS.len(), 4);
    tauri::Builder::default()
        .setup(|app| {
            app.manage(BridgeState::default());
            let resources = app.path().resource_dir()?;
            let script = commands::resource_initialization_script(&resources)
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
            commands::settings_load,
            commands::settings_save,
            commands::capabilities_check,
            commands::scan_images,
            commands::process_images,
            commands::distribute,
            commands::preferences_get,
            commands::preferences_set,
            commands::picker_select,
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::commands::{INVOKE_REGISTRY, TOPICS, resource_initialization_script};
    use super::contract::*;
    use super::security::PickerProvenance;
    use serde_json::json;
    use std::fs;
    use std::path::Path;

    #[test]
    fn registry_is_exact() {
        assert_eq!(INVOKE_REGISTRY.len(), 9);
        assert_eq!(TOPICS.len(), 4);
        assert_eq!(INVOKE_REGISTRY[8].name, "picker_select");
        assert_eq!(INVOKE_REGISTRY[2].topic, Some("job.capabilities_check"));
        assert_eq!(INVOKE_REGISTRY[8].errors.len(), 4);
    }

    #[test]
    fn packaged_resource_paths_are_injected_before_the_main_window_is_created() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("valid Tauri configuration");
        assert_eq!(config["app"]["windows"][0]["create"], false);

        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("application root");
        let script = resource_initialization_script(root).expect("bundled resources");
        assert!(script.starts_with("window.__IMG_SPLT_RESOURCES__="));
        for resource in [
            "langs/en.json",
            "langs/vi.json",
            "themes/system.json",
            "themes/light.json",
            "themes/dark.json",
        ] {
            assert!(script.contains(resource), "missing packaged resource {resource}");
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
    fn nullable_fields_are_present_and_missing_fields_are_rejected() {
        let request = Req {
            schema_version: 1,
            request_id: None,
            payload: Empty {},
        };
        assert_eq!(
            serde_json::to_value(request).expect("serialize"),
            json!({"schema_version":1,"request_id":null,"payload":{}})
        );
        assert!(serde_json::from_value::<Req<Empty>>(json!({"schema_version":1,"payload":{}})).is_err());
        assert!(serde_json::from_value::<ToolStatus>(json!({"available":false})).is_err());
    }

    #[test]
    fn provenance_is_per_window_and_kind() {
        let root = std::env::temp_dir().join(format!("img-splt-bridge-{}", std::process::id()));
        fs::create_dir_all(&root).expect("root");
        let file = root.join("image.png");
        fs::write(&file, b"fixture").expect("fixture");
        let provenance = PickerProvenance::default();
        provenance
            .replace_directory("main", ImagePickerKind::Input, &root)
            .expect("select root");
        assert!(
            provenance
                .resolve_existing("main", ImagePickerKind::Input, &file)
                .is_ok()
        );
        assert!(
            provenance
                .resolve_existing("other", ImagePickerKind::Input, &file)
                .is_err()
        );
        assert!(
            provenance
                .resolve_existing("main", ImagePickerKind::Output, &file)
                .is_err()
        );
    }
}
