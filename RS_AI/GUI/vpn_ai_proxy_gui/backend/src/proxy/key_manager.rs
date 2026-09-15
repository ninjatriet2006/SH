use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndpointKeyManager {
    pub key_file_path: Option<String>,
    pub current_key_index: usize,
    pub total_keys: usize,
    pub current_key_preview: Option<String>,
    pub last_switched_at: Option<String>,
    #[serde(skip)]
    pub cached_keys: Vec<String>,
    #[serde(skip)]
    pub cached_mtime: Option<SystemTime>,
}

impl EndpointKeyManager {
    /// Đọc danh sách key từ file được gắn với endpoint này (hỗ trợ .txt mỗi dòng 1 key hoặc .json mảng string)
    pub fn load_keys_from_file(file_path: &str) -> Result<Vec<String>, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("Key file does not exist: {}", file_path));
        }

        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read key file: {}", e))?;

        if file_path.ends_with(".json") {
            if let Ok(keys) = serde_json::from_str::<Vec<String>>(&content) {
                return Ok(keys.into_iter().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()).collect());
            }
        }

        // Default txt lines
        let keys: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();

        Ok(keys)
    }

    /// Đảm bảo cache keys hợp lệ với mtime của file, tránh đọc đĩa trên mỗi request
    pub fn ensure_cached_keys(&mut self) -> Result<&[String], String> {
        let path_str = match self.key_file_path.as_deref() {
            Some(p) => p,
            None => {
                self.cached_keys.clear();
                self.total_keys = 0;
                self.current_key_preview = None;
                return Ok(&self.cached_keys);
            }
        };

        let path = Path::new(path_str);
        if !path.exists() {
            self.cached_keys.clear();
            self.total_keys = 0;
            self.current_key_preview = None;
            return Err(format!("Key file does not exist: {}", path_str));
        }

        let current_mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
        if self.cached_keys.is_empty() || self.cached_mtime != current_mtime {
            let loaded = Self::load_keys_from_file(path_str)?;
            self.cached_keys = loaded;
            self.cached_mtime = current_mtime;
            self.total_keys = self.cached_keys.len();
        }

        Ok(&self.cached_keys)
    }

    /// Lấy key hiện tại theo current_key_index (sử dụng in-memory cache)
    pub fn get_active_key(&self) -> Option<String> {
        if !self.cached_keys.is_empty() {
            let idx = self.current_key_index % self.cached_keys.len();
            return Some(self.cached_keys[idx].clone());
        }

        // Fallback nếu cache chưa được load
        let path_str = self.key_file_path.as_ref()?;
        let keys = Self::load_keys_from_file(path_str).ok()?;
        if keys.is_empty() {
            return None;
        }
        let idx = self.current_key_index % keys.len();
        Some(keys[idx].clone())
    }

    /// Chuyển con trỏ sang key tiếp theo trong file của endpoint này
    pub fn advance_to_next_key(&mut self) -> Option<String> {
        let _ = self.ensure_cached_keys();
        if self.cached_keys.is_empty() {
            return None;
        }

        self.current_key_index = (self.current_key_index + 1) % self.cached_keys.len();
        self.total_keys = self.cached_keys.len();
        let next_key = self.cached_keys[self.current_key_index].clone();
        
        self.current_key_preview = if next_key.len() > 10 {
            Some(format!("{}...{}", &next_key[..6], &next_key[next_key.len() - 4..]))
        } else {
            Some(next_key.clone())
        };
        self.last_switched_at = Some(chrono::Local::now().format("%H:%M:%S").to_string());

        Some(next_key)
    }

    /// Cập nhật preview và tổng số key
    pub fn refresh_metadata(&mut self) {
        let snapshot: Result<Vec<String>, String> =
            self.ensure_cached_keys().map(|k| k.to_vec());
        match snapshot {
            Ok(keys) => {
                self.total_keys = keys.len();
                if !keys.is_empty() {
                    let idx = self.current_key_index % keys.len();
                    let k = &keys[idx];
                    self.current_key_preview = if k.len() > 10 {
                        Some(format!("{}...{}", &k[..6], &k[k.len() - 4..]))
                    } else {
                        Some(k.clone())
                    };
                } else {
                    self.current_key_preview = None;
                }
            }
            Err(_) => {
                self.total_keys = 0;
                self.current_key_preview = None;
            }
        }
    }
}
