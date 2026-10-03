//! Hardware & Telemetry Spoofing Module.
//!
//! Ghi các giá trị giả lập danh tính phần cứng vào file `User/globalStorage/storage.json`
//! của profile IDE, đánh lừa hệ thống thu thập telemetry của hãng (Cursor, CodeBuddy, Windsurf, v.v.).

use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use super::model::HardwareFingerprint;

/// Ghi đè hoặc cập nhật mã phần cứng giả lập vào file `storage.json` trong profile.
pub fn apply_hardware_spoofing(user_data_dir: &Path, fingerprint: &HardwareFingerprint) -> Result<(), String> {
    let global_storage = user_data_dir.join("User").join("globalStorage");
    if !global_storage.exists() {
        fs::create_dir_all(&global_storage).map_err(|e| format!("cannot create globalStorage dir: {e}"))?;
    }

    let storage_json_path = global_storage.join("storage.json");
    let mut current_obj = if storage_json_path.exists() {
        let content = fs::read_to_string(&storage_json_path).unwrap_or_default();
        serde_json::from_str::<Value>(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if !current_obj.is_object() {
        current_obj = json!({});
    }

    if let Some(map) = current_obj.as_object_mut() {
        map.insert("telemetry.machineId".to_string(), json!(fingerprint.machine_id));
        map.insert("telemetry.macMachineId".to_string(), json!(fingerprint.mac_machine_id));
        map.insert("telemetry.devDeviceId".to_string(), json!(fingerprint.dev_device_id));
        map.insert("telemetry.sqmId".to_string(), json!(fingerprint.sqm_id));
    }

    let formatted = serde_json::to_string_pretty(&current_obj)
        .map_err(|e| format!("failed to serialize storage.json: {e}"))?;

    fs::write(&storage_json_path, formatted)
        .map_err(|e| format!("failed to write storage.json: {e}"))?;

    Ok(())
}
