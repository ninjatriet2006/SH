//! AEAD vault for encrypting sensitive fields at rest (Tokens/Cookies).
//!
//! The 256-bit master key lives in the OS keyring (Linux Secret Service,
//! macOS Keychain, Windows Credential Manager) — never in `.env` or disk files.
//! When the keyring is unavailable (headless/CI), falls back to a 0600-perm
//! file next to the database so local dev keeps working.
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use rand::RngCore;

const KEYRING_SERVICE: &str = "com.universal.api";
/// Service keyring cũ (tên WorkBuddy) — chỉ đọc để migration, không ghi mới.
const KEYRING_SERVICE_LEGACY: &str = "com.workbuddy.gui";
const KEYRING_USER: &str = "storage_master_key";

#[derive(Clone)]
pub struct Vault {
    key: [u8; 32],
}

impl Vault {
    /// Load the master key from the OS keyring, creating a random one on first
    /// run. `fallback_path` is used only when the keyring is unreachable.
    pub fn open(fallback_path: &std::path::Path) -> Result<Self, String> {
        let key = load_or_create_key(fallback_path)?;
        Ok(Self { key })
    }

    /// Test-only vault with an explicit key.
    #[cfg(test)]
    pub fn with_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key))
    }

    /// Encrypt plaintext -> base64(nonce || ciphertext).
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ct = self
            .cipher()
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| format!("encrypt error: {e}"))?;
        let mut blob = nonce.to_vec();
        blob.extend_from_slice(&ct);
        Ok(base64::engine::general_purpose::STANDARD.encode(blob))
    }

    /// Decrypt base64(nonce || ciphertext) -> plaintext.
    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let blob = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| format!("b64 decode error: {e}"))?;
        if blob.len() < 12 {
            return Err("ciphertext too short".into());
        }
        let (nonce_bytes, ct) = blob.split_at(12);
        let pt = self
            .cipher()
            .decrypt(Nonce::from_slice(nonce_bytes), ct)
            .map_err(|e| format!("decrypt error: {e}"))?;
        String::from_utf8(pt).map_err(|e| format!("utf8 error: {e}"))
    }
}

fn load_or_create_key(fallback_path: &std::path::Path) -> Result<[u8; 32], String> {
    // 1. Service mới.
    if let Some(key) = read_keyring_key(KEYRING_SERVICE) {
        return Ok(key);
    }
    // 2. Migration: đọc từ service cũ (WorkBuddy), rồi ghi sang service mới để
    // lần sau khỏi fallback. Token cũ không mất sau đổi tên.
    if let Some(key) = read_keyring_key(KEYRING_SERVICE_LEGACY) {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            let _ = entry.set_password(&base64::engine::general_purpose::STANDARD.encode(key));
        }
        return Ok(key);
    }
    // 3. Chưa có key nào: tạo mới ở service mới (như cũ) + VERIFY đọc lại được.
    // Bài học thực tế: key "tạo xong nhưng không persist ở đâu" khiến mọi row
    // mã hóa trong phiên đó thành rác sau restart — thà fail LOUD ngay tại boot
    // còn hơn im lặng chạy rồi mất toàn bộ credentials.
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let b64 = base64::engine::general_purpose::STANDARD.encode(key);
        if entry.set_password(&b64).is_ok() && read_keyring_key(KEYRING_SERVICE) == Some(key) {
            return Ok(key);
        }
        // keyring write failed/unverifiable -> fall through to file fallback
    }
    let key = load_or_create_key_file(fallback_path)?;
    // Verify file round-trip (đọc lại đúng key vừa ghi).
    match std::fs::read(fallback_path) {
        Ok(raw) => {
            let ok = std::str::from_utf8(&raw)
                .ok()
                .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b.trim()).ok())
                .map(|d| d.len() == 32 && d == key.to_vec())
                .unwrap_or(false);
            if !ok {
                return Err("vault key created but not readable back — refusing ephemeral key (credentials would be lost on restart)".into());
            }
            Ok(key)
        }
        Err(e) => Err(format!("vault key file unreadable after write: {e}")),
    }
}

/// Đọc key thô từ một keyring service. None = không có/khóa/sai định dạng.
fn read_keyring_key(service: &str) -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(service, KEYRING_USER).ok()?;
    let b64 = entry.get_password().ok()?;
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if raw.len() != 32 {
        return None;
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&raw);
    Some(key)
}

fn load_or_create_key_file(path: &std::path::Path) -> Result<[u8; 32], String> {
    if let Ok(raw) = std::fs::read(path) {
        if let Ok(b64) = std::str::from_utf8(&raw) {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
                if decoded.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&decoded);
                    return Ok(key);
                }
            }
        }
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let b64 = base64::engine::general_purpose::STANDARD.encode(key);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, b64).map_err(|e| format!("write key file: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::Vault;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let vault = Vault::with_key([42u8; 32]);
        let secret = "eyJhbGciOiJIUzI1NiJ9.secret-token-cookie-value";
        let encrypted = vault.encrypt(secret).unwrap();
        assert_ne!(encrypted, secret);
        assert!(!encrypted.contains("secret-token"));
        assert_eq!(vault.decrypt(&encrypted).unwrap(), secret);
    }

    #[test]
    fn encrypt_uses_random_nonce() {
        let vault = Vault::with_key([7u8; 32]);
        let a = vault.encrypt("same-plaintext").unwrap();
        let b = vault.encrypt("same-plaintext").unwrap();
        assert_ne!(a, b, "random nonce must make ciphertexts differ");
    }

    #[test]
    fn decrypt_rejects_garbage() {
        let vault = Vault::with_key([1u8; 32]);
        assert!(vault.decrypt("!!!not-base64!!!").is_err());
        assert!(vault.decrypt("c2hvcnQ=").is_err());
    }
}
