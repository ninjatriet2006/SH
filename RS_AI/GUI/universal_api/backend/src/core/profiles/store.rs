//! Instance Profile Store.
//!
//! Lưu trữ danh sách profiles vào `instances.json` và quản lý thư mục `--user-data-dir`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::model::{HardwareFingerprint, InstanceProfile};
use super::spoofing::apply_hardware_spoofing;
use crate::core::paths;

const PROFILES_FILE: &str = "instances.json";
static STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStoreData {
    pub profiles: Vec<InstanceProfile>,
}

pub fn get_instances_root_dir() -> PathBuf {
    paths::data_base().join("instances")
}

pub fn get_store_file_path() -> PathBuf {
    paths::data_base().join(PROFILES_FILE)
}

pub fn load_profiles() -> Result<Vec<InstanceProfile>, String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let path = get_store_file_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let data: ProfileStoreData = serde_json::from_str(&content).unwrap_or_default();
    Ok(data.profiles)
}

pub fn save_profiles(profiles: &[InstanceProfile]) -> Result<(), String> {
    let _lock = STORE_LOCK.lock().map_err(|_| "Failed to lock profile store".to_string())?;
    let path = get_store_file_path();
    paths::ensure_parent(&path)?;
    let data = ProfileStoreData {
        profiles: profiles.to_vec(),
    };
    let json = serde_json::to_string_pretty(&data).map_err(|e| format!("serialize error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

/// Tạo một profile mới với thư mục cô lập và ID phần cứng giả lập.
pub fn create_profile(
    name: &str,
    platform_id: &str,
    bound_account_id: Option<String>,
    extra_args: Vec<String>,
) -> Result<InstanceProfile, String> {
    let id = format!("inst_{}", &uuid_v4()[..12]);
    let root = get_instances_root_dir().join(platform_id);
    let user_data_dir = root.join(&id);

    fs::create_dir_all(&user_data_dir).map_err(|e| format!("create instance dir: {e}"))?;

    // Sinh hardware fingerprint giả lập
    let fingerprint = HardwareFingerprint::random();
    apply_hardware_spoofing(&user_data_dir, &fingerprint)?;

    let profile = InstanceProfile {
        id: id.clone(),
        name: if name.trim().is_empty() { format!("{platform_id}-{id}") } else { name.trim().to_string() },
        platform_id: platform_id.to_string(),
        user_data_dir,
        bound_account_id,
        extra_args,
        hardware_fingerprint: fingerprint,
        created_at: Utc::now().to_rfc3339(),
    };

    let mut all = load_profiles()?;
    all.push(profile.clone());
    save_profiles(&all)?;

    Ok(profile)
}

/// Xóa profile và chuyển thư mục sang thùng rác hoặc xóa an toàn.
pub fn delete_profile(id: &str) -> Result<(), String> {
    let mut all = load_profiles()?;
    let idx = all.iter().position(|p| p.id == id).ok_or_else(|| "profile not found".to_string())?;
    let profile = all.remove(idx);

    // Xóa thư mục dữ liệu instance
    if profile.user_data_dir.exists() {
        let _ = fs::remove_dir_all(&profile.user_data_dir);
    }

    save_profiles(&all)?;
    Ok(())
}

/// Nhân bản một profile hiện có với một bộ Hardware Fingerprint mới hoàn toàn.
pub fn clone_profile(id: &str, new_name: &str) -> Result<InstanceProfile, String> {
    let all = load_profiles()?;
    let source = all.iter().find(|p| p.id == id).ok_or_else(|| "source profile not found".to_string())?;

    let new_id = format!("inst_{}", &uuid_v4()[..12]);
    let root = get_instances_root_dir().join(&source.platform_id);
    let new_user_data_dir = root.join(&new_id);

    fs::create_dir_all(&new_user_data_dir).map_err(|e| format!("create instance dir: {e}"))?;

    // Sao chép cài đặt người dùng (User/settings.json, keybindings...) nếu có
    let src_user_dir = source.user_data_dir.join("User");
    let dest_user_dir = new_user_data_dir.join("User");
    if src_user_dir.exists() {
        let _ = copy_dir_shallow(&src_user_dir, &dest_user_dir);
    }

    // Sinh hardware fingerprint mới toanh cho bản clone
    let new_fingerprint = HardwareFingerprint::random();
    apply_hardware_spoofing(&new_user_data_dir, &new_fingerprint)?;

    let cloned = InstanceProfile {
        id: new_id,
        name: new_name.trim().to_string(),
        platform_id: source.platform_id.clone(),
        user_data_dir: new_user_data_dir,
        bound_account_id: source.bound_account_id.clone(),
        extra_args: source.extra_args.clone(),
        hardware_fingerprint: new_fingerprint,
        created_at: Utc::now().to_rfc3339(),
    };

    let mut updated = all;
    updated.push(cloned.clone());
    save_profiles(&updated)?;

    Ok(cloned)
}

/// Cập nhật thông tin profile (đổi tên, đổi account gắn, đổi args).
pub fn update_profile(
    id: &str,
    name: Option<String>,
    bound_account_id: Option<Option<String>>,
    extra_args: Option<Vec<String>>,
) -> Result<InstanceProfile, String> {
    let mut all = load_profiles()?;
    let profile = all.iter_mut().find(|p| p.id == id).ok_or_else(|| "profile not found".to_string())?;

    if let Some(n) = name {
        profile.name = n;
    }
    if let Some(b) = bound_account_id {
        profile.bound_account_id = b;
    }
    if let Some(a) = extra_args {
        profile.extra_args = a;
    }

    let updated = profile.clone();
    save_profiles(&all)?;
    Ok(updated)
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

fn copy_dir_shallow(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_file() {
            let target = dst.join(entry.file_name());
            let _ = fs::copy(entry.path(), target);
        }
    }
    Ok(())
}
