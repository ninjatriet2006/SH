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
