use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use universe_manager_backend::{
    AppEntry, CancellationToken, ConfigStore, DiscoveryService, ErrorKind, LifecycleManager, ManagerConfig,
    ManagerSettings, search_apps, stable_app_id,
};

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "universe-manager-backend-{label}-{}-{}",
            std::process::id(),
            unique_sequence()
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

fn unique_sequence() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn config(managed: &Path, apps: Vec<AppEntry>) -> ManagerConfig {
    ManagerConfig {
        settings: ManagerSettings {
            managed_dir: managed.to_string_lossy().into_owned(),
        },
        apps,
    }
}

#[test]
fn app_entry_serializes_nullable_fields_and_empty_arrays() {
    let value = serde_json::to_value(AppEntry::default()).expect("serialize app DTO");
    for field in [
        "source_path",
        "icon_path",
        "symlink_file",
        "is_custom",
        "start_cmd",
        "stop_cmd",
        "category",
        "package_type",
        "registry_key",
        "product_code",
        "about_url",
        "publisher",
        "version",
        "uninstall_cmd",
    ] {
        assert!(value.get(field).is_some_and(serde_json::Value::is_null), "{field}");
    }
    assert_eq!(value.get("inventory_sources"), Some(&serde_json::json!([])));
}

#[test]
fn config_round_trip_is_atomic_and_rejects_outside_managed_root() {
    let root = TestDir::new("config");
    let config_dir = root.0.join("config");
    let managed = root.0.join("managed");
    let outside = root.0.join("outside");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&managed).expect("managed dir");
    fs::create_dir_all(&outside).expect("outside dir");
    let store = ConfigStore::new(config_dir, vec![managed.clone()]).expect("store");
    let value = config(&managed, Vec::new());
    store.save(&value).expect("save");
    assert_eq!(store.load().expect("load"), value);

    let error = store
        .save(&config(&outside, Vec::new()))
        .expect_err("outside root must fail");
    assert_eq!(error.kind(), ErrorKind::Forbidden);
}

#[cfg(unix)]
#[test]
fn config_load_rechecks_canonical_parent_before_open() {
    use std::os::unix::fs::symlink;

    let root = TestDir::new("config-parent-recheck");
    let config_dir = root.0.join("config");
    let original_dir = root.0.join("config-original");
    let outside = root.0.join("outside");
    let managed = root.0.join("managed");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&outside).expect("outside dir");
    fs::create_dir_all(&managed).expect("managed dir");
    let store = ConfigStore::new(config_dir.clone(), vec![managed.clone()]).expect("store");
    let value = config(&managed, Vec::new());
    fs::write(
        outside.join("config.json"),
        serde_json::to_vec(&value).expect("serialize fixture"),
    )
    .expect("outside config fixture");
    fs::rename(&config_dir, &original_dir).expect("move approved parent");
    symlink(&outside, &config_dir).expect("replace parent with symlink");

    let error = store.load().expect_err("replaced canonical parent must fail closed");
    assert_eq!(error.kind(), ErrorKind::Forbidden);
}

#[test]
fn detection_is_contained_cancellable_and_has_stable_identity() {
    let root = TestDir::new("detect");
    let source = root.0.join("source");
    let app = source.join("Demo-App");
    fs::create_dir_all(&app).expect("app dir");
    let executable = app.join("demo");
    fs::write(&executable, b"fixture").expect("executable fixture");
    #[cfg(unix)]
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    fs::write(app.join("icon.svg"), b"svg").expect("icon fixture");
    let service = DiscoveryService::new(vec![source.clone()]).expect("service");
    let report = service.detect(&app, &CancellationToken::new(), |_| {}).expect("detect");
    assert_eq!(
        report.executables,
        vec![fs::canonicalize(&executable).expect("canonical executable")]
    );
    assert_eq!(stable_app_id(&app), stable_app_id(&app));

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let error = service.detect(&app, &cancellation, |_| {}).expect_err("cancelled");
    assert_eq!(error.kind(), ErrorKind::Cancelled);

    let outside = root.0.join("outside");
    fs::create_dir_all(&outside).expect("outside");
    let error = service
        .detect(&outside, &CancellationToken::new(), |_| {})
        .expect_err("outside source must fail");
    assert_eq!(error.kind(), ErrorKind::Forbidden);
}

#[test]
fn search_uses_only_registered_apps_beneath_managed_directory() {
    let root = TestDir::new("search");
    let managed = root.0.join("managed");
    let app_dir = managed.join("alpha");
    fs::create_dir_all(&app_dir).expect("app dir");
    let app = AppEntry {
        id: "alpha-stable".to_string(),
        name: "Alpha Editor".to_string(),
        install_path: app_dir.to_string_lossy().into_owned(),
        version: Some("1.2.3".to_string()),
        inventory_sources: vec!["Applications".to_string()],
        ..AppEntry::default()
    };
    let report = search_apps(
        &config(&managed, vec![app]),
        vec![managed],
        "editor",
        &CancellationToken::new(),
        |_| {},
    )
    .expect("search");
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].id, "alpha-stable");
}

#[cfg(target_os = "linux")]
#[test]
fn lifecycle_stops_only_the_exact_spawned_process_identity() {
    let root = TestDir::new("lifecycle");
    let config_dir = root.0.join("config");
    let managed = root.0.join("managed");
    let app_dir = managed.join("yes-app");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&app_dir).expect("app dir");
    let executable = app_dir.join("yes");
    fs::copy("/usr/bin/yes", &executable).expect("copy fixture executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    let app = AppEntry {
        id: "yes-app-stable".to_string(),
        name: "Yes App".to_string(),
        install_path: app_dir.to_string_lossy().into_owned(),
        exec_path: executable.to_string_lossy().into_owned(),
        package_type: Some("Local".to_string()),
        ..AppEntry::default()
    };
    let value = config(&managed, vec![app]);
    let store = ConfigStore::new(config_dir, vec![managed]).expect("store");
    let lifecycle = LifecycleManager::new(store);
    lifecycle
        .start(&value, "yes-app-stable", true, &CancellationToken::new(), |_| {})
        .expect("start");
    let identity = lifecycle
        .identity("yes-app-stable")
        .expect("identity")
        .expect("tracked");
    assert!(Path::new(&format!("/proc/{}", identity.pid)).exists());
    lifecycle
        .stop(&value, "yes-app-stable", true, &CancellationToken::new(), |_| {})
        .expect("stop");
    assert!(lifecycle.identity("yes-app-stable").expect("identity").is_none());
}

#[test]
fn lifecycle_rejects_unconfirmed_action_before_spawn() {
    let root = TestDir::new("confirmation");
    let config_dir = root.0.join("config");
    let managed = root.0.join("managed");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&managed).expect("managed dir");
    let store = ConfigStore::new(config_dir, vec![managed.clone()]).expect("store");
    let lifecycle = LifecycleManager::new(store);
    let error = lifecycle
        .start(
            &config(&managed, Vec::new()),
            "missing",
            false,
            &CancellationToken::new(),
            |_| {},
        )
        .expect_err("confirmation required");
    #[cfg(target_os = "linux")]
    assert_eq!(error.kind(), ErrorKind::Forbidden);
    #[cfg(not(target_os = "linux"))]
    assert_eq!(error.kind(), ErrorKind::Unavailable);

    let error = lifecycle
        .stop(
            &config(&managed, Vec::new()),
            "missing",
            false,
            &CancellationToken::new(),
            |_| {},
        )
        .expect_err("stop confirmation required before lookup");
    #[cfg(target_os = "linux")]
    assert_eq!(error.kind(), ErrorKind::Forbidden);
    #[cfg(not(target_os = "linux"))]
    assert_eq!(error.kind(), ErrorKind::Unavailable);
}

#[cfg(target_os = "linux")]
#[test]
fn lifecycle_rejects_shell_command_payload_without_execution() {
    let root = TestDir::new("shell-injection");
    let config_dir = root.0.join("config");
    let managed = root.0.join("managed");
    let app_dir = managed.join("app");
    let marker = root.0.join("injected");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&app_dir).expect("app dir");
    let executable = app_dir.join("true");
    fs::copy("/usr/bin/true", &executable).expect("fixture executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    let app = AppEntry {
        id: "shell-fixture".into(),
        name: "Shell Fixture".into(),
        install_path: app_dir.to_string_lossy().into_owned(),
        exec_path: executable.to_string_lossy().into_owned(),
        start_cmd: Some(format!("touch {}", marker.display())),
        ..AppEntry::default()
    };
    let value = config(&managed, vec![app]);
    let manager = LifecycleManager::new(ConfigStore::new(config_dir, vec![managed]).expect("store"));
    let error = manager
        .start(&value, "shell-fixture", true, &CancellationToken::new(), |_| {})
        .expect_err("shell command must be denied");
    assert_eq!(error.kind(), ErrorKind::Forbidden);
    assert!(!marker.exists(), "shell payload must not execute");
}

#[cfg(target_os = "linux")]
#[test]
fn lifecycle_rejects_symlink_executable_without_spawning_target() {
    use std::os::unix::fs::symlink;

    let root = TestDir::new("executable-symlink");
    let config_dir = root.0.join("config");
    let managed = root.0.join("managed");
    let app_dir = managed.join("app");
    let marker = root.0.join("spawned");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&app_dir).expect("app dir");
    let target = app_dir.join("target");
    fs::write(&target, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).expect("target");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("chmod target");
    let executable = app_dir.join("launcher");
    symlink(&target, &executable).expect("symlink executable");
    let app = AppEntry {
        id: "symlink-fixture".into(),
        name: "Symlink Fixture".into(),
        install_path: app_dir.to_string_lossy().into_owned(),
        exec_path: executable.to_string_lossy().into_owned(),
        ..AppEntry::default()
    };
    let manager = LifecycleManager::new(ConfigStore::new(config_dir, vec![managed.clone()]).expect("store"));
    let error = manager
        .start(
            &config(&managed, vec![app]),
            "symlink-fixture",
            true,
            &CancellationToken::new(),
            |_| {},
        )
        .expect_err("symlink executable must fail closed");
    assert_eq!(error.kind(), ErrorKind::Forbidden);
    assert!(!marker.exists(), "symlink target must not run");
}
