pub mod models;

use std::fs;
use std::path::PathBuf;
use crate::config::models::*;

pub struct ConfigManager {
    pub config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let dir = get_config_dir();
        let _ = fs::create_dir_all(&dir);
        Self { config_dir: dir }
    }

    pub fn init_all_configs(&self) -> anyhow::Result<()> {
        self.load_video_config()?;
        self.load_audio_config()?;
        self.load_image_config()?;
        self.load_doc_config()?;
        self.load_archive_config()?;
        Ok(())
    }

    pub fn load_video_config(&self) -> anyhow::Result<VideoConfig> {
        let path = self.config_dir.join("config_video.yaml");
        let config: VideoConfig = self.load_or_create(&path)?;
        Ok(config)
    }

    pub fn load_audio_config(&self) -> anyhow::Result<AudioConfig> {
        let path = self.config_dir.join("config_audio.yaml");
        let config: AudioConfig = self.load_or_create(&path)?;
        Ok(config)
    }

    pub fn load_image_config(&self) -> anyhow::Result<ImageConfig> {
        let path = self.config_dir.join("config_img.yaml");
        let config: ImageConfig = self.load_or_create(&path)?;
        Ok(config)
    }

    pub fn load_doc_config(&self) -> anyhow::Result<DocConfig> {
        let path = self.config_dir.join("config_doc.yaml");
        let config: DocConfig = self.load_or_create(&path)?;
        Ok(config)
    }

    pub fn load_archive_config(&self) -> anyhow::Result<ArchiveConfig> {
        let path = self.config_dir.join("config_archive.yaml");
        let config: ArchiveConfig = self.load_or_create(&path)?;
        Ok(config)
    }

    fn load_or_create<T>(&self, path: &PathBuf) -> anyhow::Result<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Default,
    {
        if !path.exists() {
            let default_val = T::default();
            let yaml = serde_yml::to_string(&default_val)?;
            fs::write(path, yaml)?;
            return Ok(default_val);
        }

        let content = fs::read_to_string(path)?;
        
        // Auto-healing: If deserialization fails or missing fields exist,
        // Serde default will fallback, and we resave it.
        let config: T = match serde_yml::from_str(&content) {
            Ok(cfg) => cfg,
            Err(_) => {
                // Fallback to default
                let fallback = T::default();
                let yaml = serde_yml::to_string(&fallback)?;
                let _ = fs::write(path, yaml);
                fallback
            }
        };

        // Resave to keep the file fully populated with default fields (Auto-healing missing fields)
        let yaml = serde_yml::to_string(&config)?;
        let _ = fs::write(path, yaml);

        Ok(config)
    }

    pub fn delete_all_configs(&self) -> anyhow::Result<()> {
        let _ = fs::remove_dir_all(&self.config_dir);
        println!("[✅] Đã xóa toàn bộ thư mục cấu hình!");
        Ok(())
    }
}

fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(appdata).join("universal_converter")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".config").join("universal_converter")
    }
}
