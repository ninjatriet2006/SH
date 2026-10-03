//! Profile Data Models & Hardware Fingerprint Structures.
//!
//! Tách biệt và mô phỏng hoàn toàn môi trường của từng instance IDE độc lập.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareFingerprint {
    /// SHA-256 hash giả lập cho ID máy (`telemetry.machineId`)
    pub machine_id: String,
    /// SHA-256 hash giả lập cho card mạng MAC (`telemetry.macMachineId`)
    pub mac_machine_id: String,
    /// UUID v4 giả lập cho Device ID (`telemetry.devDeviceId`)
    pub dev_device_id: String,
    /// Windows SQM session identifier (`sqmId`)
    pub sqm_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceProfile {
    pub id: String,
    pub name: String,
    /// Định danh nền tảng IDE (ví dụ: "codebuddy_cn", "codebuddy_global", "cursor", "windsurf", "vscode", "zed")
    pub platform_id: String,
    /// Thư mục dữ liệu người dùng cô lập (`--user-data-dir`)
    pub user_data_dir: PathBuf,
    /// ID tài khoản được gán chết cho profile này (nếu có)
    pub bound_account_id: Option<String>,
    /// Các cờ dòng lệnh bổ sung (ví dụ: `--proxy-server=http://127.0.0.1:7890`)
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Bộ mã máy ảo riêng biệt để chống phát hiện multi-account
    pub hardware_fingerprint: HardwareFingerprint,
    /// Thời điểm khởi tạo
    pub created_at: String,
}

impl HardwareFingerprint {
    /// Sinh ngẫu nhiên một bộ định danh phần cứng giả lập mới hoàn toàn.
    pub fn random() -> Self {
        use rand::RngCore;
        use sha2::{Sha256, Digest};

        let mut rng = rand::thread_rng();

        // Sinh machineId ngẫu nhiên (64 hex characters)
        let mut b1 = [0u8; 32];
        rng.fill_bytes(&mut b1);
        let mut hasher1 = Sha256::new();
        hasher1.update(&b1);
        let machine_id = format!("{:x}", hasher1.finalize());

        // Sinh macMachineId ngẫu nhiên (64 hex characters)
        let mut b2 = [0u8; 32];
        rng.fill_bytes(&mut b2);
        let mut hasher2 = Sha256::new();
        hasher2.update(&b2);
        let mac_machine_id = format!("{:x}", hasher2.finalize());

        // Sinh devDeviceId (UUID v4 format)
        let mut u = [0u8; 16];
        rng.fill_bytes(&mut u);
        u[6] = (u[6] & 0x0f) | 0x40; // Version 4
        u[8] = (u[8] & 0x3f) | 0x80; // Variant 10
        let dev_device_id = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u[0], u[1], u[2], u[3], u[4], u[5], u[6], u[7], u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
        );

        // Sinh sqmId dạng {UUID}
        let sqm_id = format!("{{{}}}", dev_device_id.to_uppercase());

        Self {
            machine_id,
            mac_machine_id,
            dev_device_id,
            sqm_id,
        }
    }
}
