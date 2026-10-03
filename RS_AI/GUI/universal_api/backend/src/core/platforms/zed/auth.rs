//! Zed Cloud Authentication & Credential Parsing.

use serde_json::Value;

pub const MAX_CREDENTIAL_FILE_BYTES: u64 = 64 * 1024;
const MAX_FIELD_LEN: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct ZedCredential {
    pub id: String,
    pub access_token: String,
}

fn bad_cred(_msg: &str) -> String {
    "Set credential file to an absolute, owner-only Zed JSON file {\"type\":\"zed\",\"id\":\"...\",\"access_token\":\"...\"}.".to_string()
}

pub fn parse_credential_json(raw: &str) -> Result<ZedCredential, String> {
    if raw.len() as u64 > MAX_CREDENTIAL_FILE_BYTES {
        return Err(bad_cred("size"));
    }
    let v: Value = serde_json::from_str(raw).map_err(|_| bad_cred("json"))?;
    let obj = v.as_object().ok_or_else(|| bad_cred("shape"))?;
    let typ = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let id = obj.get("id").and_then(|t| t.as_str()).unwrap_or("").trim();
    let token = obj.get("access_token").and_then(|t| t.as_str()).unwrap_or("").trim();
    if typ != "zed" || id.is_empty() || token.is_empty() {
        return Err(bad_cred("fields"));
    }
    if id.len() > MAX_FIELD_LEN || token.len() > MAX_FIELD_LEN {
        return Err(bad_cred("length"));
    }
    Ok(ZedCredential {
        id: id.to_string(),
        access_token: token.to_string(),
    })
}

pub fn read_credential_file(path: &str) -> Result<ZedCredential, String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(bad_cred("relative"));
    }
    let meta = std::fs::symlink_metadata(p).map_err(|_| bad_cred("stat"))?;
    if meta.file_type().is_symlink() {
        return Err(bad_cred("symlink"));
    }
    if !meta.is_file() || meta.len() > MAX_CREDENTIAL_FILE_BYTES {
        return Err(bad_cred("type/size"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(bad_cred("perms"));
        }
    }
    let raw = std::fs::read(p).map_err(|_| bad_cred("read"))?;
    let text = std::str::from_utf8(&raw).map_err(|_| bad_cred("utf8"))?;
    parse_credential_json(text)
}
