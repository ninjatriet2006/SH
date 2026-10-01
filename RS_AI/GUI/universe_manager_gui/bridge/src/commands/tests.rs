use super::*;
use crate::contract::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use universe_manager_backend as backend;

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "universe-bridge-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn running_fixture(root: &Path) -> (PathBuf, PathBuf, ManagerConfig) {
    let config_dir = root.join("config");
    let managed = root.join("managed");
    let app_dir = managed.join("yes-app");
    fs::create_dir_all(&config_dir).expect("config directory");
    fs::create_dir_all(&app_dir).expect("application directory");
    let executable = app_dir.join("yes");
    fs::copy("/usr/bin/yes", &executable).expect("copy fixture executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    let config = ManagerConfig {
        settings: backend::ManagerSettings {
            managed_dir: managed.to_string_lossy().into_owned(),
        },
        apps: vec![backend::AppEntry {
            id: "yes-app".into(),
            name: "Yes App".into(),
            install_path: app_dir.to_string_lossy().into_owned(),
            exec_path: executable.to_string_lossy().into_owned(),
            package_type: Some("Local".into()),
            ..backend::AppEntry::default()
        }],
    };
    (config_dir, managed, config)
}

fn start_fixture(
    state: &BridgeState,
    window: &str,
    config_dir: &Path,
    managed: &Path,
    config: &ManagerConfig,
) -> (Arc<WindowLifecycle>, u32) {
    state
        .picker
        .replace_directory(window, UniversePickerKind::Managed, managed)
        .expect("picker root");
    let action = state
        .lifecycle_action(window, config_dir.to_owned(), managed.to_owned())
        .expect("lifecycle action");
    action
        .run(
            true,
            config,
            "yes-app",
            &backend::CancellationToken::new(),
            |_| {},
        )
        .expect("start fixture");
    let lifecycle = Arc::clone(&action.lifecycle);
    let pid = lifecycle
        .manager
        .identity("yes-app")
        .expect("identity lookup")
        .expect("tracked process")
        .pid;
    drop(action);
    (lifecycle, pid)
}

#[test]
fn managed_picker_change_is_rejected_while_process_is_active() {
    let root = TestDir::new("picker-active");
    let (config_dir, managed, config) = running_fixture(&root.0);
    let replacement = root.0.join("replacement");
    fs::create_dir_all(&replacement).expect("replacement root");
    let state = BridgeState::default();
    let (lifecycle, _) = start_fixture(&state, "main", &config_dir, &managed, &config);

    let failure = state
        .replace_picker_directory("main", UniversePickerKind::Managed, &replacement)
        .expect_err("active process must retain its root authority");
    assert_eq!(failure.code, IpcErrorCode::Conflict);
    assert_eq!(
        state
            .picker
            .root("main", UniversePickerKind::Managed)
            .expect("original root"),
        fs::canonicalize(&managed).expect("canonical root")
    );

    let stop = state
        .lifecycle_action("main", config_dir, managed)
        .expect("stop authority");
    stop.run(
        false,
        &config,
        "yes-app",
        &backend::CancellationToken::new(),
        |_| {},
    )
    .expect("stop fixture");
    drop(stop);
    assert!(
        lifecycle
            .manager
            .identity("yes-app")
            .expect("identity")
            .is_none()
    );
}

#[test]
fn window_teardown_stops_and_reaps_active_process_without_cross_window_authority() {
    let root = TestDir::new("teardown-active");
    let (config_dir, managed, config) = running_fixture(&root.0);
    let state = BridgeState::default();
    let (lifecycle, pid) = start_fixture(&state, "main", &config_dir, &managed, &config);

    assert!(state
        .lifecycle
        .lock()
        .expect("registry")
        .get("other")
        .is_none());
    state.clear_window("other");
    assert!(
        lifecycle
            .manager
            .identity("yes-app")
            .expect("identity")
            .is_some()
    );

    state.clear_window("main");
    assert!(
        lifecycle
            .manager
            .identity("yes-app")
            .expect("identity")
            .is_none()
    );
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    assert!(state
        .lifecycle
        .lock()
        .expect("registry")
        .get("main")
        .is_none());
}
