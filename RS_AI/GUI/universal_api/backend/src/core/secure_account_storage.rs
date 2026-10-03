//! AES-256-GCM envelopes for provider account detail files.
//! Directly ported from Cockpit Tools (src-tauri/src/modules/secure_account_storage.rs)
//! for 100% format and cryptographic parity.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const KEY_FILE: &str = "secure-account-storage.key";
pub const VERSION: u32 = 1;
pub const ROTATION_SECONDS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureAccountEnvelope {
    pub version: u32,
    pub kind: String,
    pub algorithm: String,
    pub key_id: String,
    pub nonce: String,
    pub ciphertext: String,
    pub encrypted_at: i64,
}

pub fn get_cockpit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cockpit_tools")
}

pub fn key_path() -> PathBuf {
    get_cockpit_dir().join(KEY_FILE)
}

pub fn read_key(path: &Path) -> Result<[u8; 32], String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("Đọc khóa mã hóa tài khoản thất bại: {}", e))?;
    let bytes = general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|e| format!("Giải mã Base64 khóa thất bại: {}", e))?;
    bytes
        .try_into()
        .map_err(|_| "Độ dài khóa mã hóa không hợp lệ (cần đúng 32 bytes)".to_string())
}

pub fn read_or_create_key_at(path: &Path) -> Result<[u8; 32], String> {
    if path.exists() {
        return read_key(path);
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let encoded = general_purpose::STANDARD.encode(key);
    fs::write(path, encoded).map_err(|e| format!("Ghi khóa mã hóa thất bại: {}", e))?;
    Ok(key)
}

pub fn cipher() -> Result<Aes256Gcm, String> {
    let key = read_or_create_key_at(&key_path())?;
    Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Khởi tạo mã hóa AES-256-GCM thất bại: {}", e))
}

pub fn read_account_file_readonly<T: DeserializeOwned>(
    path: &Path,
    key_path: &Path,
) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Đọc tệp tài khoản thất bại: {}", e))?;
    if let Ok(envelope) = serde_json::from_str::<SecureAccountEnvelope>(&content) {
        let key = read_key(key_path)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Khởi tạo mã hóa thất bại: {}", e))?;
        return decrypt_envelope(&envelope, cipher);
    }
    serde_json::from_str(&content).map_err(|e| format!("Phân tích cú pháp JSON thất bại: {}", e))
}

pub fn decrypt_envelope<T: DeserializeOwned>(
    envelope: &SecureAccountEnvelope,
    cipher: Aes256Gcm,
) -> Result<T, String> {
    if envelope.version != VERSION || envelope.algorithm != "AES-256-GCM" {
        return Err("Phiên bản hoặc thuật toán mã hóa không được hỗ trợ".to_string());
    }
    let nonce = general_purpose::STANDARD
        .decode(envelope.nonce.trim())
        .map_err(|e| format!("Giải mã nonce thất bại: {}", e))?;
    if nonce.len() != 12 {
        return Err("Độ dài nonce không hợp lệ (cần đúng 12 bytes)".to_string());
    }
    let ciphertext = general_purpose::STANDARD
        .decode(envelope.ciphertext.trim())
        .map_err(|e| format!("Giải mã ciphertext thất bại: {}", e))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|e| format!("Giải mã phong bì AES-256-GCM thất bại: {:?}", e))?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("Phân tích JSON giải mã thất bại: {}", e))
}

pub fn serialize_account_file<T: Serialize>(kind: &str, account: &T) -> Result<String, String> {
    let plaintext = serde_json::to_vec(account).map_err(|e| format!("Serialize thất bại: {}", e))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher()?
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_ref())
        .map_err(|e| format!("Mã hóa AES-256-GCM thất bại: {:?}", e))?;
    let envelope = SecureAccountEnvelope {
        version: VERSION,
        kind: kind.to_string(),
        algorithm: "AES-256-GCM".to_string(),
        key_id: "local-secure-account-storage-v1".to_string(),
        nonce: general_purpose::STANDARD.encode(nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
        encrypted_at: chrono::Utc::now().timestamp(),
    };
    serde_json::to_string_pretty(&envelope).map_err(|e| format!("Format JSON thất bại: {}", e))
}

pub fn write_string_atomic(path: &Path, content: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let parent = path.parent().ok_or_else(|| "Không tìm thấy thư mục cha".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Tạo thư mục thất bại: {}", e))?;

    if path.exists() {
        let bak = path.with_extension("bak");
        let _ = fs::copy(path, &bak);
    }

    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temp_name = format!(
        ".{}.tmp.{}.{}.atomic",
        path.file_name().and_then(|x| x.to_str()).unwrap_or("file"),
        std::process::id(),
        nanos
    );
    let temp_path = parent.join(temp_name);

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|e| format!("Mở file tạm {} thất bại: {}", temp_path.display(), e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Ghi file tạm {} thất bại: {}", temp_path.display(), e))?;
    file.sync_all()
        .map_err(|e| format!("Đồng bộ file tạm {} thất bại: {}", temp_path.display(), e))?;
    drop(file);

    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        format!("Đổi tên file tạm sang {} thất bại: {}", path.display(), e)
    })?;

    Ok(())
}

pub fn save_account_envelope_atomic<T: Serialize>(path: &Path, kind: &str, account: &T) -> Result<(), String> {
    let content = serialize_account_file(kind, account)?;
    write_string_atomic(path, &content)
}

pub fn deserialize_account_file<T: DeserializeOwned>(
    _path: &Path,
    content: &str,
) -> Result<(T, bool), String> {
    if let Ok(envelope) = serde_json::from_str::<SecureAccountEnvelope>(content) {
        let value = decrypt_envelope(&envelope, cipher()?)?;
        let needs_rotation = chrono::Utc::now().timestamp() - envelope.encrypted_at > ROTATION_SECONDS;
        return Ok((value, needs_rotation));
    }
    let value = serde_json::from_str::<T>(content).map_err(|e| format!("Phân tích cú pháp JSON thất bại: {}", e))?;
    Ok((value, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct DemoAccount {
        id: String,
        secret: String,
    }

    #[test]
    fn test_cockpit_secure_storage_roundtrip() {
        let dir = std::env::temp_dir().join(format!("cockpit-roundtrip-{}", uuid_simple()));
        let _ = fs::create_dir_all(&dir);
        let key_p = dir.join(KEY_FILE);
        let key = read_or_create_key_at(&key_p).expect("key init");

        let acc = DemoAccount {
            id: "test-user".into(),
            secret: "super-secret-token".into(),
        };

        let cipher_inst = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = [5u8; 12];
        let plain_bytes = serde_json::to_vec(&acc).unwrap();
        let ct = cipher_inst.encrypt(Nonce::from_slice(&nonce), plain_bytes.as_ref()).unwrap();
        let env = SecureAccountEnvelope {
            version: VERSION,
            kind: "demo".into(),
            algorithm: "AES-256-GCM".into(),
            key_id: "local-secure-account-storage-v1".into(),
            nonce: general_purpose::STANDARD.encode(nonce),
            ciphertext: general_purpose::STANDARD.encode(ct),
            encrypted_at: chrono::Utc::now().timestamp(),
        };

        let file_p = dir.join("acc.json");
        fs::write(&file_p, serde_json::to_string(&env).unwrap()).unwrap();

        let loaded: DemoAccount = read_account_file_readonly(&file_p, &key_p).expect("read readonly");
        assert_eq!(loaded, acc);

        let _ = fs::remove_dir_all(dir);
    }

    fn uuid_simple() -> String {
        let mut bytes = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut bytes);
        format!("{:016x}", u64::from_ne_bytes(bytes))
    }
}
