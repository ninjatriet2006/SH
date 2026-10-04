//! Instance Profile Store.
//!
//! Lưu trữ danh sách profiles/instances theo từng nền tảng tương thích 1:1 với Cockpit Tools.
//! Hỗ trợ nạp Default Instance, kiểm tra tiến trình PID còn sống và cô lập thư mục theo platform.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Utc;

use super::model::{DefaultInstanceSettings, ExtraArgsVal, HardwareFingerprint, InstanceProfile, InstanceStore};
use super::spoofing::apply_hardware_spoofing;
use crate::core::paths;

static STORE_LOCK: Mutex<()> = Mutex::new(());

pub fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(windows)]
    {
        false
    }
}

pub fn get_platform_file_name(platform_id: &str) -> String {
    match platform_id.to_lowercase().as_str() {
        "antigravity" | "antigravity_ide" | "antigravity_desktop" => "instances.json".to_string(),
        "codex" => "codex_instances.json".to_string(),
        "claude" => "claude_instances.json".to_string(),
        "windsurf" | "devin" => "windsurf_instances.json".to_string(),
        "cursor" => "cursor_instances.json".to_string(),
        "kiro" => "kiro_instances.json".to_string(),
        "codebuddy" => "codebuddy_instances.json".to_string(),
        "codebuddy_cn" => "codebuddy_cn_instances.json".to_string(),
        "workbuddy" => "workbuddy_instances.json".to_string(),
        "trae" => "trae_instances.json".to_string(),
        "trae_solo" => "trae_solo_instances.json".to_string(),
        "trae_cn" => "trae_cn_instances.json".to_string(),
        "trae_solo_cn" => "trae_solo_cn_instances.json".to_string(),
        "qoder" => "qoder_instances.json".to_string(),
        "zcode" => "zcode_instances.json".to_string(),
        "grok" => "grok_instances.json".to_string(),
        "vscode" | "github_copilot" | "ghcp" => "github_copilot_instances.json".to_string(),
        "zed" | "zed_cloud" => "zed_instances.json".to_string(),
        other => {
            let clean = other.replace('/', "_").replace('\\', "_");
            format!("{}_instances.json", clean)
        }
    }
}

pub fn get_instances_root_dir(platform_id: &str) -> PathBuf {
    if let Some(cp_dir) = paths::cockpit_dir() {
        let cp_inst = cp_dir.join("instances").join(platform_id);
        if cp_inst.exists() {
            return cp_inst;
        }
    }
    paths::data_base().join("instances").join(platform_id)
}

/// Tìm file cấu hình instance ưu tiên data_base rồi đến ~/.cockpit_tools
fn find_instance_file(platform_id: &str) -> (PathBuf, Option<PathBuf>) {
    let filename = get_platform_file_name(platform_id);
    let primary = paths::data_base().join(&filename);
    let mut fallback = paths::cockpit_dir().map(|d| d.join(&filename));

    // Đặc thù Antigravity: nếu không thấy instances.json có thể fallback sang antigravity_legacy_instances.json
    if (platform_id.starts_with("antigravity")) && !primary.exists() {
        if let Some(ref fb) = fallback {
            if !fb.exists() {
                if let Some(cp_dir) = paths::cockpit_dir() {
                    let leg = cp_dir.join("antigravity_legacy_instances.json");
                    if leg.exists() {
                        fallback = Some(leg);
                    }
                }
            }
        }
    }

    (primary, fallback)
}

fn read_instance_store(platform_id: &str) -> InstanceStore {
    let (primary, fallback) = find_instance_file(platform_id);

    for p in [Some(primary), fallback].into_iter().flatten() {
        if p.is_file() {
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(parsed) = serde_json::from_str::<InstanceStore>(&content) {
                    return parsed;
                }
            }
        }
    }

    // Default instance store nếu chưa có file
    InstanceStore {
        instances: Vec::new(),
        default_settings: Some(DefaultInstanceSettings {
            bind_account_id: None,
            extra_args: Some(String::new()),
            working_dir: None,
            launch_mode: Some("app".to_string()),
            model_routing: None,
            app_speed: None,
            follow_local_account: Some(false),
            auto_sync_threads: Some(false),
            last_pid: None,
        }),
    }
}

fn write_instance_store(platform_id: &str, store: &InstanceStore) -> Result<(), String> {
    let (primary, fallback) = find_instance_file(platform_id);
    // Nếu tệp cockpit đang tồn tại, ghi vào tệp cockpit để đồng bộ hoàn toàn
    let target = if let Some(fb) = fallback {
        if fb.is_file() {
            fb
        } else {
            primary
        }
    } else {
        primary
    };

    paths::ensure_parent(&target)?;
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("serialize instance store error: {e}"))?;
    crate::core::secure_account_storage::write_string_atomic(&target, &json)?;
    Ok(())
}

/// Nạp toàn bộ instances của một platform (kèm Default Instance và kiểm tra trạng thái sống của PID).
pub fn load_platform_instances(platform_id: &str) -> Result<Vec<InstanceProfile>, String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let store = read_instance_store(platform_id);
    let mut result = Vec::new();

    // 1. Thêm Default Instance nếu có
    if let Some(def) = store.default_settings {
        let is_running = def.last_pid.map(is_pid_alive).unwrap_or(false);
        let bound_account_id = if !platform_id.starts_with("antigravity")
            && (platform_id == "zed" || platform_id == "zed_cloud")
            && def.bind_account_id.as_deref().map(|id| id.contains("@gmail.com")).unwrap_or(false)
        {
            None
        } else {
            def.bind_account_id
        };

        result.push(InstanceProfile {
            id: "default".to_string(),
            name: "Default Instance".to_string(),
            platform_id: platform_id.to_string(),
            user_data_dir: PathBuf::from("default"),
            working_dir: def.working_dir.as_ref().map(PathBuf::from),
            bound_account_id,
            extra_args: def.extra_args.map(ExtraArgsVal::Text).unwrap_or_default(),
            hardware_fingerprint: None,
            created_at: 0,
            last_launched_at: None,
            last_pid: def.last_pid,
            is_default: true,
            is_running,
            launch_mode: def.launch_mode,
            model_routing: def.model_routing,
            app_speed: def.app_speed,
        });
    }

    // 2. Thêm các custom instances
    for mut inst in store.instances {
        inst.platform_id = platform_id.to_string();
        inst.is_default = false;
        inst.is_running = inst.last_pid.map(is_pid_alive).unwrap_or(false);
        result.push(inst);
    }

    Ok(result)
}

/// Tạo một instance mới cho một nền tảng cụ thể.
pub fn create_platform_instance(
    platform_id: &str,
    name: &str,
    init_mode: &str,
    _source_instance_id: Option<&str>,
    existing_dir: Option<&str>,
    bound_account_id: Option<String>,
    extra_args: Option<String>,
) -> Result<InstanceProfile, String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let mut store = read_instance_store(platform_id);

    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err("Tên Instance không được để trống".to_string());
    }

    let id = format!("inst_{}", &uuid_v4()[..12]);
    let user_data_dir = if init_mode == "existing_dir" && existing_dir.is_some() {
        let p = PathBuf::from(existing_dir.unwrap());
        if !p.exists() {
            fs::create_dir_all(&p).map_err(|e| format!("Tạo thư mục existing: {e}"))?;
        }
        p
    } else {
        let base_instances_dir = if let Some(cp_dir) = paths::cockpit_dir() {
            cp_dir.join("instances").join(platform_id)
        } else {
            paths::data_base().join("instances").join(platform_id)
        };
        let p = base_instances_dir.join(clean_name);
        fs::create_dir_all(&p).map_err(|e| format!("Tạo thư mục instance: {e}"))?;
        p
    };

    let fingerprint = HardwareFingerprint::random();
    let _ = apply_hardware_spoofing(&user_data_dir, &fingerprint);

    let profile = InstanceProfile {
        id: id.clone(),
        name: clean_name.to_string(),
        platform_id: platform_id.to_string(),
        user_data_dir,
        working_dir: None,
        bound_account_id,
        extra_args: extra_args.map(ExtraArgsVal::Text).unwrap_or_default(),
        hardware_fingerprint: Some(fingerprint),
        created_at: Utc::now().timestamp(),
        last_launched_at: None,
        last_pid: None,
        is_default: false,
        is_running: false,
        launch_mode: Some("app".to_string()),
        model_routing: None,
        app_speed: None,
    };

    store.instances.push(profile.clone());
    write_instance_store(platform_id, &store)?;

    Ok(profile)
}

/// Cập nhật thông tin của một instance
pub fn update_platform_instance(
    platform_id: &str,
    instance_id: &str,
    name: Option<String>,
    bound_account_id: Option<Option<String>>,
    extra_args: Option<String>,
) -> Result<(), String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let mut store = read_instance_store(platform_id);

    if instance_id == "default" {
        if let Some(ref mut def) = store.default_settings {
            if let Some(b) = bound_account_id {
                def.bind_account_id = b;
            }
            if let Some(a) = extra_args {
                def.extra_args = Some(a);
            }
        }
    } else {
        let inst = store
            .instances
            .iter_mut()
            .find(|p| p.id == instance_id)
            .ok_or_else(|| "Instance not found".to_string())?;

        if let Some(n) = name {
            inst.name = n;
        }
        if let Some(b) = bound_account_id {
            inst.bound_account_id = b;
        }
        if let Some(a) = extra_args {
            inst.extra_args = ExtraArgsVal::Text(a);
        }
    }

    write_instance_store(platform_id, &store)?;
    Ok(())
}

/// Ghi nhận PID khi khởi chạy một instance
pub fn record_platform_instance_launch(
    platform_id: &str,
    instance_id: &str,
    pid: u32,
) -> Result<(), String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let mut store = read_instance_store(platform_id);

    if instance_id == "default" {
        if let Some(ref mut def) = store.default_settings {
            def.last_pid = Some(pid);
        }
    } else if let Some(inst) = store.instances.iter_mut().find(|p| p.id == instance_id) {
        inst.last_pid = Some(pid);
        inst.last_launched_at = Some(Utc::now().timestamp());
    }

    write_instance_store(platform_id, &store)?;
    Ok(())
}

/// Ghi nhận dừng tiến trình instance
pub fn record_platform_instance_stop(
    platform_id: &str,
    instance_id: &str,
) -> Result<(), String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let mut store = read_instance_store(platform_id);

    if instance_id == "default" {
        if let Some(ref mut def) = store.default_settings {
            def.last_pid = None;
        }
    } else if let Some(inst) = store.instances.iter_mut().find(|p| p.id == instance_id) {
        inst.last_pid = None;
    }

    write_instance_store(platform_id, &store)?;
    Ok(())
}

/// Xóa một custom instance
pub fn delete_platform_instance(platform_id: &str, instance_id: &str) -> Result<(), String> {
    if instance_id == "default" {
        return Err("Không thể xóa Default Instance của ứng dụng".to_string());
    }

    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let mut store = read_instance_store(platform_id);

    let idx = store
        .instances
        .iter()
        .position(|p| p.id == instance_id)
        .ok_or_else(|| "Instance not found".to_string())?;

    let removed = store.instances.remove(idx);
    if removed.user_data_dir.is_dir() {
        let _ = fs::remove_dir_all(&removed.user_data_dir);
    }

    write_instance_store(platform_id, &store)?;
    Ok(())
}

/// Nạp tất cả instances trên toàn bộ hệ sinh thái (phục vụ Global Overview)
pub fn load_all_instances() -> Result<Vec<InstanceProfile>, String> {
    let platforms = [
        "antigravity",
        "codex",
        "claude",
        "windsurf",
        "cursor",
        "kiro",
        "codebuddy",
        "codebuddy_cn",
        "workbuddy",
        "trae",
        "zed",
        "qoder",
        "zcode",
        "grok",
        "vscode",
    ];

    let mut all = Vec::new();
    for p in platforms {
        if let Ok(list) = load_platform_instances(p) {
            all.extend(list);
        }
    }

    Ok(all)
}

// Backward compatibility legacy methods
pub fn load_profiles() -> Result<Vec<InstanceProfile>, String> {
    load_all_instances()
}

pub fn create_profile(
    name: &str,
    platform_id: &str,
    bound_account_id: Option<String>,
    extra_args: Vec<String>,
) -> Result<InstanceProfile, String> {
    create_platform_instance(
        platform_id,
        name,
        "copy_source",
        None,
        None,
        bound_account_id,
        Some(extra_args.join(" ")),
    )
}

pub fn delete_profile(id: &str) -> Result<(), String> {
    let all = load_all_instances()?;
    if let Some(target) = all.iter().find(|p| p.id == id) {
        delete_platform_instance(&target.platform_id, id)
    } else {
        Err("Profile not found".to_string())
    }
}

pub fn clone_profile(id: &str, new_name: &str) -> Result<InstanceProfile, String> {
    let all = load_all_instances()?;
    let target = all
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "Source profile not found".to_string())?;

    create_platform_instance(
        &target.platform_id,
        new_name,
        "copy_source",
        Some(id),
        None,
        target.bound_account_id.clone(),
        Some(target.extra_args.to_string_lossy()),
    )
}

pub fn update_profile(
    id: &str,
    name: Option<String>,
    bound_account_id: Option<Option<String>>,
    extra_args: Option<Vec<String>>,
) -> Result<InstanceProfile, String> {
    let all = load_all_instances()?;
    let target = all
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "Profile not found".to_string())?;

    let p_id = target.platform_id.clone();
    update_platform_instance(
        &p_id,
        id,
        name,
        bound_account_id,
        extra_args.map(|v| v.join(" ")),
    )?;

    let refreshed = load_platform_instances(&p_id)?;
    refreshed
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "Profile not found after update".to_string())
}

fn uuid_v4() -> String {
    use rand::RngCore;
    let mut u = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut u);
    u[6] = (u[6] & 0x0f) | 0x40;
    u[8] = (u[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u[0], u[1], u[2], u[3], u[4], u[5], u[6], u[7], u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_antigravity_instances() {
        let list = load_platform_instances("antigravity").expect("Should load instances");
        assert!(!list.is_empty(), "Should load at least default instance");
        let def = list.iter().find(|i| i.is_default).expect("Default instance must exist");
        assert_eq!(def.name, "Default Instance");
    }

    #[test]
    fn test_load_zed_instances_isolation() {
        let list = load_platform_instances("zed").expect("Should load instances");
        assert!(!list.is_empty(), "Should load default instance for zed");
        let def = list.iter().find(|i| i.is_default).expect("Default instance must exist");
        assert_eq!(def.name, "Default Instance");
        assert_ne!(def.bound_account_id.as_deref(), Some("vuk560269@gmail.com"), "Zed must never inherit Antigravity bound account");
    }
}
