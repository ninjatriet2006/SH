use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndpointKeyManager {
    pub key_file_path: Option<String>,
    pub failed_key_file_path: Option<String>,
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
    /// Xóa 1 key lỗi (ví dụ 401 Unauthorized - key chết/thu hồi) khỏi file chính.
    /// Trả về `Ok(true)` nếu key thực sự tồn tại trong file và đã bị xóa thành công (mảng bị rút ngắn).
    pub fn remove_key_from_main_file(&mut self, key_to_remove: &str) -> Result<bool, String> {
        let path_str = match self.key_file_path.as_deref() {
            Some(p) if !p.trim().is_empty() => p.trim(),
            _ => return Ok(false),
        };
        let path = Path::new(path_str);
        if !path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read key file: {}", e))?;
        let removed: bool;
        
        // Handle JSON format explicitly
        if path_str.ends_with(".json") {
            if let Ok(keys) = serde_json::from_str::<Vec<String>>(&content) {
                let initial_len = keys.len();
                let filtered: Vec<String> = keys.into_iter().filter(|k| k.trim() != key_to_remove).collect();
                removed = filtered.len() < initial_len;
                if removed {
                    let new_content = serde_json::to_string_pretty(&filtered).unwrap_or_else(|_| "[]".to_string());
                    fs::write(path, new_content).map_err(|e| format!("Failed to write JSON key file: {}", e))?;
                }
            } else {
                removed = false;
            }
        } else {
            // Handle TXT format
            let initial_lines: Vec<&str> = content.lines().collect();
            let filtered_lines: Vec<&str> = initial_lines
                .iter()
                .cloned()
                .filter(|l| l.trim() != key_to_remove)
                .collect();
            removed = filtered_lines.len() < initial_lines.len();
            if removed {
                let new_content = if filtered_lines.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", filtered_lines.join("\n"))
                };
                fs::write(path, new_content).map_err(|e| format!("Failed to write TXT key file: {}", e))?;
            }
        }

        if removed {
            // Invalidate cache
            self.cached_mtime = None;
            self.refresh_metadata();
            // KHÔNG tự ý trừ current_key_index nữa:
            // Khi key tại vị trí hiện tại bị xóa, mảng bị co lại, key kế tiếp tự động trượt lên lấp vào vị trí này!
        }

        Ok(removed)
    }

    /// Di chuyển 1 key lỗi 403 (Forbidden / Insufficient Quota - hết tiền) sang file phụ.
    /// Trả về `Ok(true)` nếu key thực sự đã bị gỡ khỏi file chính.
    pub fn move_key_to_failed_file(&mut self, key_to_move: &str) -> Result<bool, String> {
        // 1. Ghi vào file phụ nếu có cấu hình hợp lệ (không rỗng)
        if let Some(ref failed_path_str) = self.failed_key_file_path {
            let clean_failed_path = failed_path_str.trim();
            if !clean_failed_path.is_empty() {
                let failed_path = Path::new(clean_failed_path);
                if let Some(parent) = failed_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(failed_path)
                    .map_err(|e| format!("Failed to open failed key file: {}", e))?;
                writeln!(file, "{}", key_to_move)
                    .map_err(|e| format!("Failed to append to failed key file: {}", e))?;
            }
        }

        // 2. Xóa khỏi file chính dù có hoặc không có file phụ
        self.remove_key_from_main_file(key_to_move)
    }

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

    /// Resolve key hiện tại, refresh cache từ đĩa trước nếu file đổi mtime.
    /// Dùng trong request path (cần &mut) để key bị sửa ngoài file có hiệu lực ngay,
    /// tránh dùng key stale đã cache từ trước.
    pub fn resolve_active_key(&mut self) -> Option<String> {
        let _ = self.ensure_cached_keys();
        self.get_active_key()
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
                    self.current_key_index %= keys.len();
                    let k = &keys[self.current_key_index];
                    self.current_key_preview = if k.len() > 10 {
                        Some(format!("{}...{}", &k[..6], &k[k.len() - 4..]))
                    } else {
                        Some(k.clone())
                    };
                } else {
                    self.current_key_index = 0;
                    self.current_key_preview = None;
                }
            }
            Err(_) => {
                self.total_keys = 0;
                self.current_key_index = 0;
                self.current_key_preview = None;
            }
        }
    }
}
