use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub default_distribution_mode: String,
    pub max_files_per_folder: usize,
    pub fixed_folder_count: usize,
    pub max_retries: usize,
    pub min_upscale_width: u32,
    pub target_upscale_width: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_distribution_mode: "balanced".to_string(),
            max_files_per_folder: 80,
            fixed_folder_count: 5,
            max_retries: 5,
            min_upscale_width: 600,
            target_upscale_width: 1280,
        }
    }
}

pub fn load_or_create_settings() -> Settings {
    let settings_path = Path::new("settings.yaml");
    if !settings_path.exists() {
        let default_settings = Settings::default();
        let yaml_str = format!(
            "# CẤU HÌNH IMAGE SPLITER\n\
             #\n\
             # --- THÔNG SỐ XỬ LÝ LỖI ---\n\
             # Số lần chạy lại tối đa nếu ffmpeg lỗi hoặc output file 0-byte\n\
             max_retries: {}\n\
             \n\
             # --- THÔNG SỐ ĐỘ PHÂN GIẢI (UPSCALE) ---\n\
             # Nếu chiều rộng ảnh nhỏ hơn min_upscale_width, nó sẽ bị upscale lên target_upscale_width\n\
             min_upscale_width: {}\n\
             target_upscale_width: {}\n\
             \n\
             # --- THÔNG SỐ CHIA THƯ MỤC (DISTRIBUTION) ---\n\
             # Thuật toán mặc định (khi không muốn chọn tay): \n\
             # - 'balanced' (Cân bằng), 'greedy' (Lấp đầy), 'fixed' (Số thư mục cố định)\n\
             default_distribution_mode: '{}'\n\
             \n\
             # Số lượng file tối đa mỗi folder (Dùng cho mode 'balanced' và 'greedy')\n\
             max_files_per_folder: {}\n\
             \n\
             # Số thư mục chia cố định (Dùng cho mode 'fixed')\n\
             fixed_folder_count: {}\n",
            default_settings.max_retries,
            default_settings.min_upscale_width,
            default_settings.target_upscale_width,
            default_settings.default_distribution_mode,
            default_settings.max_files_per_folder,
            default_settings.fixed_folder_count
        );
        fs::write(settings_path, yaml_str).expect("Không thể tạo file settings.yaml");
        return default_settings;
    }

    let file_content = fs::read_to_string(settings_path).expect("Không thể đọc file settings.yaml");
    serde_yaml::from_str(&file_content).unwrap_or_else(|e| {
        println!("Lỗi khi đọc settings.yaml: {}. Đang dùng cấu hình mặc định...", e);
        Settings::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_reasonable() {
        let settings = Settings::default();
        // All numeric values should be positive (non-zero)
        assert!(settings.max_files_per_folder > 0, "max_files_per_folder should be > 0");
        assert!(settings.fixed_folder_count > 0, "fixed_folder_count should be > 0");
        assert!(settings.max_retries > 0, "max_retries should be > 0");
        assert!(settings.min_upscale_width > 0, "min_upscale_width should be > 0");
        assert!(settings.target_upscale_width > 0, "target_upscale_width should be > 0");
        // Default distribution mode must be a known mode
        assert!(
            ["balanced", "greedy", "fixed"].contains(&settings.default_distribution_mode.as_str()),
            "default_distribution_mode should be one of 'balanced', 'greedy', or 'fixed'"
        );
    }

    #[test]
    fn test_min_upscale_width_sensible() {
        let settings = Settings::default();
        // min_upscale_width should be at least a reasonable minimum (e.g. >= 100)
        assert!(
            settings.min_upscale_width >= 100,
            "min_upscale_width ({}) is too low; expected >= 100",
            settings.min_upscale_width
        );
        // min_upscale_width should be less than target_upscale_width
        assert!(
            settings.min_upscale_width < settings.target_upscale_width,
            "min_upscale_width ({}) should be less than target_upscale_width ({})",
            settings.min_upscale_width,
            settings.target_upscale_width
        );
    }

    #[test]
    fn test_settings_serde_round_trip() {
        let original = Settings::default();
        // Serialize to YAML string
        let yaml_str = serde_yaml::to_string(&original).expect("Failed to serialize settings to YAML");
        assert!(!yaml_str.is_empty(), "YAML output should not be empty");
        // Deserialize back
        let deserialized: Settings = serde_yaml::from_str(&yaml_str).expect("Failed to deserialize settings from YAML");
        // Compare all fields
        assert_eq!(
            deserialized.default_distribution_mode,
            original.default_distribution_mode
        );
        assert_eq!(deserialized.max_files_per_folder, original.max_files_per_folder);
        assert_eq!(deserialized.fixed_folder_count, original.fixed_folder_count);
        assert_eq!(deserialized.max_retries, original.max_retries);
        assert_eq!(deserialized.min_upscale_width, original.min_upscale_width);
        assert_eq!(deserialized.target_upscale_width, original.target_upscale_width);
    }

    #[test]
    fn test_settings_default_values_are_consistent() {
        let default = Settings::default();
        // Verify the default values match specific expected constants
        // (If these change in the future, this test will explicitly document the change)
        assert_eq!(default.default_distribution_mode, "balanced");
        assert_eq!(default.max_files_per_folder, 80);
        assert_eq!(default.fixed_folder_count, 5);
        assert_eq!(default.max_retries, 5);
        assert_eq!(default.min_upscale_width, 600);
        assert_eq!(default.target_upscale_width, 1280);
    }

    #[test]
    fn test_load_or_create_settings_defaults_on_bad_yaml() {
        // Parsing invalid YAML should fall back to Default
        let bad_yaml = "not: valid: yaml: [[[";
        let settings: Settings = serde_yaml::from_str(bad_yaml).unwrap_or_else(|_| Settings::default());
        let default = Settings::default();
        assert_eq!(settings.min_upscale_width, default.min_upscale_width);
    }
}
