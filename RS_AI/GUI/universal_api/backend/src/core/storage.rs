//! Local SQLite storage — replaces the old Upstash/Redis persistence layer.
//!
//! Tables: `accounts` (upstream CodeBuddy credentials, Token/Cookie columns
//! encrypted with the [`Vault`]), `access_keys` (local API keys handed out to
//! clients like IDEs — stored as SHA-256 hash, plaintext shown once),
//! `usage_logs` (per-key request/token accounting).
use std::path::Path;
use std::sync::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use base64::Engine;

use crate::core::crypto::Vault;

#[derive(Debug, Clone, Serialize)]
pub struct StoredAccount {
    pub uid: String,
    pub domain: String,
    pub nickname: String,
    pub enterprise_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Zed native account (khối riêng). Token mã hóa, không bao giờ trả về UI.
#[derive(Debug, Clone, Serialize)]
pub struct StoredZedAccount {
    pub id: String,
    pub label: String,
    pub access_token: String,
    pub org_id: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccessKey {
    pub id: i64,
    pub label: String,
    pub prefix: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageLog {
    pub id: i64,
    pub access_key_prefix: String,
    pub route: String,
    pub model: String,
    pub status: i64,
    pub tokens: i64,
    pub elapsed_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub access_key_prefix: String,
    pub requests: i64,
    pub tokens: i64,
    pub errors: i64,
}

pub struct Storage {
    conn: Mutex<Connection>,
    vault: Vault,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS accounts (
    uid TEXT PRIMARY KEY,
    domain TEXT NOT NULL DEFAULT '',
    nickname TEXT NOT NULL DEFAULT '',
    enterprise_id TEXT NOT NULL DEFAULT '',
    access_token_enc TEXT NOT NULL,
    refresh_token_enc TEXT NOT NULL DEFAULT '',
    expires_at INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS access_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL,
    prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    revoked INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS usage_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    access_key_prefix TEXT NOT NULL DEFAULT '',
    route TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    status INTEGER NOT NULL,
    tokens INTEGER NOT NULL DEFAULT 0,
    elapsed_ms INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_usage_logs_key ON usage_logs(access_key_prefix);
-- Native Zed accounts (khối riêng, theo mẫu CodeBuddy nhưng tách bảng).
CREATE TABLE IF NOT EXISTS zed_accounts (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL DEFAULT '',
    access_token_enc TEXT NOT NULL,
    org_id TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn hash_key(raw: &str) -> String {
    hex_encode(&Sha256::digest(raw.as_bytes()))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Storage {
    /// Open (or create) the database; `key_fallback` backs up the master key
    /// when the OS keyring is unavailable.
    pub fn open(db_path: &Path, key_fallback: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create db dir: {e}"))?;
        }
        let conn = Connection::open(db_path).map_err(|e| format!("open sqlite: {e}"))?;
        conn.execute_batch(SCHEMA).map_err(|e| format!("schema init: {e}"))?;
        let vault = Vault::open(key_fallback)?;
        Ok(Self { conn: Mutex::new(conn), vault })
    }

    #[cfg(test)]
    pub fn open_in_memory(vault: Vault) -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("open memory db: {e}"))?;
        conn.execute_batch(SCHEMA).map_err(|e| format!("schema init: {e}"))?;
        Ok(Self { conn: Mutex::new(conn), vault })
    }

    // ── Accounts ────────────────────────────────────────────────────────────

    pub fn upsert_account(
        &self,
        uid: &str,
        domain: &str,
        nickname: &str,
        enterprise_id: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at: i64,
    ) -> Result<(), String> {
        let at_enc = self.vault.encrypt(access_token)?;
        let rt_enc = if refresh_token.is_empty() {
            String::new()
        } else {
            self.vault.encrypt(refresh_token)?
        };
        let ts = now();
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO accounts (uid, domain, nickname, enterprise_id, access_token_enc, refresh_token_enc, expires_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(uid) DO UPDATE SET
               domain=excluded.domain, nickname=excluded.nickname, enterprise_id=excluded.enterprise_id,
               access_token_enc=excluded.access_token_enc, refresh_token_enc=excluded.refresh_token_enc,
               expires_at=excluded.expires_at, updated_at=excluded.updated_at",
            params![uid, domain, nickname, enterprise_id, at_enc, rt_enc, expires_at, ts],
        )
        .map_err(|e| format!("upsert account: {e}"))?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Result<Vec<StoredAccount>, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT uid, domain, nickname, enterprise_id, access_token_enc, refresh_token_enc, expires_at, created_at, updated_at FROM accounts ORDER BY uid")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?, row.get::<_, i64>(7)?, row.get::<_, i64>(8)?,
                ))
            })
            .map_err(|e| format!("query accounts: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            let (uid, domain, nickname, enterprise_id, at_enc, rt_enc, expires_at, created_at, updated_at) =
                row.map_err(|e| format!("row: {e}"))?;
            let access_token = self.vault.decrypt(&at_enc).map_err(|e| {
                format!("{e} (vault key mismatch — stored credentials unreadable; remove and re-import the account)")
            })?;
            let refresh_token = if rt_enc.is_empty() { String::new() } else { self.vault.decrypt(&rt_enc).map_err(|e| {
                format!("{e} (vault key mismatch — stored credentials unreadable; remove and re-import the account)")
            })? };
            out.push(StoredAccount { uid, domain, nickname, enterprise_id, access_token, refresh_token, expires_at, created_at, updated_at });
        }
        Ok(out)
    }

    pub fn delete_account(&self, uid: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute("DELETE FROM accounts WHERE uid = ?1", params![uid])
            .map_err(|e| format!("delete account: {e}"))?;
        Ok(())
    }

    /// Raw encrypted blob of one account's access token — used to prove
    /// at-rest encryption in verification.
    pub fn raw_access_token_blob(&self, uid: &str) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.query_row("SELECT access_token_enc FROM accounts WHERE uid = ?1", params![uid], |r| r.get(0))
            .map_err(|e| format!("raw blob: {e}"))
    }

    // ── Zed accounts (native block, tách bảng khỏi CodeBuddy) ────────────────

    pub fn upsert_zed_account(
        &self,
        id: &str,
        label: &str,
        access_token: &str,
        org_id: &str,
    ) -> Result<(), String> {
        let at_enc = self.vault.encrypt(access_token)?;
        let ts = now();
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO zed_accounts (id, label, access_token_enc, org_id, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET
               label=excluded.label, access_token_enc=excluded.access_token_enc,
               org_id=excluded.org_id, updated_at=excluded.updated_at",
            params![id, label, at_enc, org_id, ts],
        )
        .map_err(|e| format!("upsert zed account: {e}"))?;
        Ok(())
    }

    pub fn list_zed_accounts(&self) -> Result<Vec<StoredZedAccount>, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, label, access_token_enc, org_id, enabled, created_at, updated_at FROM zed_accounts ORDER BY id")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?, row.get::<_, i64>(6)?,
                ))
            })
            .map_err(|e| format!("query zed accounts: {e}"))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, label, at_enc, org_id, enabled, created_at, updated_at) =
                row.map_err(|e| format!("row: {e}"))?;
            let access_token = self.vault.decrypt(&at_enc).map_err(|e| {
                format!("{e} (vault key mismatch — stored credentials unreadable; remove and re-import the account)")
            })?;
            out.push(StoredZedAccount { id, label, access_token, org_id, enabled: enabled != 0, created_at, updated_at });
        }
        Ok(out)
    }

    pub fn set_zed_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let n = conn.execute(
            "UPDATE zed_accounts SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![if enabled { 1 } else { 0 }, now(), id],
        )
        .map_err(|e| format!("set zed enabled: {e}"))?;
        if n == 0 {
            return Err(format!("zed account not found: {id}"));
        }
        Ok(())
    }

    pub fn delete_zed_account(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute("DELETE FROM zed_accounts WHERE id = ?1", params![id])
            .map_err(|e| format!("delete zed account: {e}"))?;
        Ok(())
    }

    // ── Access keys ─────────────────────────────────────────────────────────

    /// Create a local access key. Returns (record, plaintext key) — plaintext
    /// is shown to the user once; only the SHA-256 hash is persisted.
    pub fn create_access_key(&self, label: &str) -> Result<(AccessKey, String), String> {
        let mut raw = [0u8; 24];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut raw);
        let plaintext = format!("wbk-{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw));
        let prefix: String = plaintext.chars().take(12).collect();
        let ts = now();
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO access_keys (label, prefix, key_hash, created_at, revoked) VALUES (?1, ?2, ?3, ?4, 0)",
            params![label, prefix, hash_key(&plaintext), ts],
        )
        .map_err(|e| format!("create access key: {e}"))?;
        let id = conn.last_insert_rowid();
        Ok((
            AccessKey { id, label: label.to_string(), prefix, created_at: ts, last_used_at: None, revoked: false },
            plaintext,
        ))
    }

    pub fn list_access_keys(&self) -> Result<Vec<AccessKey>, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, label, prefix, created_at, last_used_at, revoked FROM access_keys ORDER BY id")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AccessKey {
                    id: row.get(0)?, label: row.get(1)?, prefix: row.get(2)?,
                    created_at: row.get(3)?, last_used_at: row.get(4)?,
                    revoked: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| format!("list access keys: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("row: {e}"))
    }

    pub fn revoke_access_key(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute("UPDATE access_keys SET revoked = 1 WHERE id = ?1", params![id])
            .map_err(|e| format!("revoke key: {e}"))?;
        Ok(())
    }

    /// Validate a presented key against stored hashes; touches last_used_at.
    pub fn check_access_key(&self, presented: &str) -> Result<bool, String> {
        let hash = hash_key(presented);
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let found: Option<i64> = conn
            .query_row(
                "SELECT id FROM access_keys WHERE key_hash = ?1 AND revoked = 0",
                params![hash],
                |r| r.get(0),
            )
            .ok();
        if let Some(id) = found {
            let _ = conn.execute("UPDATE access_keys SET last_used_at = ?1 WHERE id = ?2", params![now(), id]);
        }
        Ok(found.is_some())
    }

    pub fn access_key_prefix(&self, presented: &str) -> Result<Option<String>, String> {
        let hash = hash_key(presented);
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        Ok(conn
            .query_row("SELECT prefix FROM access_keys WHERE key_hash = ?1", params![hash], |r| r.get(0))
            .ok())
    }

    // ── Usage logs ──────────────────────────────────────────────────────────

    pub fn log_usage(
        &self,
        access_key_prefix: &str,
        route: &str,
        model: &str,
        status: u16,
        tokens: u64,
        elapsed_ms: u64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO usage_logs (access_key_prefix, route, model, status, tokens, elapsed_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![access_key_prefix, route, model, status as i64, tokens as i64, elapsed_ms as i64, now()],
        )
        .map_err(|e| format!("log usage: {e}"))?;
        Ok(())
    }

    pub fn list_usage_logs(&self, limit: i64) -> Result<Vec<UsageLog>, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, access_key_prefix, route, model, status, tokens, elapsed_ms, created_at FROM usage_logs ORDER BY id DESC LIMIT ?1")
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(UsageLog {
                    id: row.get(0)?, access_key_prefix: row.get(1)?, route: row.get(2)?,
                    model: row.get(3)?, status: row.get(4)?, tokens: row.get(5)?,
                    elapsed_ms: row.get(6)?, created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("list usage: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("row: {e}"))
    }

    pub fn usage_summary(&self) -> Result<Vec<UsageSummary>, String> {
        let conn = self.conn.lock().map_err(|_| "storage lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT access_key_prefix, COUNT(*) AS req_count, COALESCE(SUM(tokens),0), COALESCE(SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END),0)
                 FROM usage_logs GROUP BY access_key_prefix ORDER BY req_count DESC",
            )
            .map_err(|e| format!("prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UsageSummary {
                    access_key_prefix: row.get(0)?, requests: row.get(1)?,
                    tokens: row.get(2)?, errors: row.get(3)?,
                })
            })
            .map_err(|e| format!("summary: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("row: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{Storage, Vault};

    fn storage() -> Storage {
        Storage::open_in_memory(Vault::with_key([9u8; 32])).unwrap()
    }

    #[test]
    fn account_insert_read_round_trip() {
        let s = storage();
        s.upsert_account("u1", "example.com", "nick", "e1", "access-token-abc", "refresh-token-xyz", 1234)
            .unwrap();
        let list = s.list_accounts().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].uid, "u1");
        assert_eq!(list[0].access_token, "access-token-abc");
        assert_eq!(list[0].refresh_token, "refresh-token-xyz");
        assert_eq!(list[0].expires_at, 1234);
    }

    #[test]
    fn tokens_are_encrypted_at_rest() {
        let s = storage();
        s.upsert_account("u2", "d", "n", "e", "super-secret-token", "refresh-secret", 0)
            .unwrap();
        let blob = s.raw_access_token_blob("u2").unwrap();
        assert_ne!(blob, "super-secret-token");
        assert!(!blob.contains("super-secret"));
        // decrypt path still yields the original value
        assert_eq!(s.list_accounts().unwrap()[0].access_token, "super-secret-token");
    }

    #[test]
    fn access_key_lifecycle() {
        let s = storage();
        let (record, plaintext) = s.create_access_key("cursor").unwrap();
        assert!(plaintext.starts_with("wbk-"));
        assert!(s.check_access_key(&plaintext).unwrap());
        assert!(!s.check_access_key("wbk-wrong-key").unwrap());
        s.revoke_access_key(record.id).unwrap();
        assert!(!s.check_access_key(&plaintext).unwrap());
        assert_eq!(s.list_access_keys().unwrap().len(), 1);
    }

    #[test]
    fn usage_logging_and_summary() {
        let s = storage();
        s.log_usage("wbk-abc", "/v1/chat/completions", "glm-5.2", 200, 150, 900).unwrap();
        s.log_usage("wbk-abc", "/v1/chat/completions", "glm-5.2", 429, 0, 120).unwrap();
        s.log_usage("wbk-xyz", "/v1/messages", "kimi-k2.7", 200, 80, 400).unwrap();
        let logs = s.list_usage_logs(10).unwrap();
        assert_eq!(logs.len(), 3);
        let summary = s.usage_summary().unwrap();
        let abc = summary.iter().find(|r| r.access_key_prefix == "wbk-abc").unwrap();
        assert_eq!(abc.requests, 2);
        assert_eq!(abc.tokens, 150);
        assert_eq!(abc.errors, 1);
    }
}
