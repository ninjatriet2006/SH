use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use crate::models::{legacy_v1, AppDatabase, AppSettings, Criterion, EmailAccount, LoginMethod, RegistrationRecord, Website, CURRENT_SCHEMA_VERSION};

static DB_INSTANCE: Mutex<Option<AppDatabase>> = Mutex::new(None);
static RESOURCE_BASE: OnceLock<PathBuf> = OnceLock::new();
const RESOURCE_ANCHOR: &str = "langs";

fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &Path| p.join(RESOURCE_ANCHOR).is_dir();

    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            if has_anchor(dir) {
                return dir.to_path_buf();
            }
            cur = dir.parent();
        }
    }

    #[cfg(debug_assertions)]
    {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(|| {
        std::env::var_os("ACCOUNT_HUB_RESOURCE_DIR")
            .map(PathBuf::from)
            .filter(|path| path.join(RESOURCE_ANCHOR).is_dir())
            .unwrap_or_else(detect_resource_base)
    })
}

fn db_file_path() -> PathBuf {
    let base = resource_base();
    let storage_dir = base.join("storage");
    if !storage_dir.exists() {
        let _ = fs::create_dir_all(&storage_dir);
    }
    storage_dir.join("accounts_data.json")
}

fn seed_criteria(now: &str) -> Vec<Criterion> {
    vec![
        Criterion { id: "crit-cheat".to_string(), name: "Cheat / Multi-acc".to_string(), description: None, created_at: now.to_string() },
        Criterion { id: "crit-kyc".to_string(), name: "KYC".to_string(), description: None, created_at: now.to_string() },
        Criterion { id: "crit-proxy".to_string(), name: "Proxy".to_string(), description: None, created_at: now.to_string() },
    ]
}

fn seed_login_methods(now: &str) -> Vec<LoginMethod> {
    vec![
        LoginMethod { id: "lm-email".to_string(), name: "Email + Password".to_string(), description: None, created_at: now.to_string() },
        LoginMethod { id: "lm-wallet".to_string(), name: "Wallet Connect".to_string(), description: None, created_at: now.to_string() },
    ]
}

fn init_sample_data() -> AppDatabase {
    let now = chrono::Local::now().to_rfc3339();
    let yesterday = (chrono::Local::now() - chrono::Duration::days(1)).to_rfc3339();
    let sample_emails = vec![
        EmailAccount {
            id: "em-1".to_string(),
            email: "dev.alpha@gmail.com".to_string(),
            owner: "Team Main".to_string(),
            recovery_email: "backup1@gmail.com".to_string(),
            phone: "0901234567".to_string(),
            tags: vec!["Main".to_string(), "Dev".to_string()],
            notes: "Tài khoản dev chính".to_string(),
            created_at: now.clone(),
        },
        EmailAccount {
            id: "em-2".to_string(),
            email: "airdrop.hunter01@gmail.com".to_string(),
            owner: "Team Airdrop".to_string(),
            recovery_email: "backup2@gmail.com".to_string(),
            phone: "".to_string(),
            tags: vec!["Airdrop".to_string(), "Multi".to_string()],
            notes: "Dùng để cheat / claim airdrop".to_string(),
            created_at: now.clone(),
        },
        EmailAccount {
            id: "em-3".to_string(),
            email: "test.bot02@gmail.com".to_string(),
            owner: "Team Bot".to_string(),
            recovery_email: "".to_string(),
            phone: "".to_string(),
            tags: vec!["Bot".to_string()],
            notes: "Tài khoản test phụ".to_string(),
            created_at: now.clone(),
        },
    ];

    let sample_websites = vec![
        Website {
            id: "web-1".to_string(),
            name: "Binance Web3".to_string(),
            url: "https://binance.com".to_string(),
            tags: vec!["Crypto / Exchange".to_string(), "Hạng tài khoản: VIP 1".to_string()],
            has_daily_checkin: true,
            criterion_ids: vec!["crit-kyc".to_string()],
            login_method_ids: vec![],
            notes: "Điểm danh Task Center hàng ngày".to_string(),
            created_at: now.clone(),
        },
        Website {
            id: "web-2".to_string(),
            name: "Grass Network".to_string(),
            url: "https://app.getgrass.io".to_string(),
            tags: vec!["DePIN / Bandwidth".to_string(), "Số node tối đa: 5".to_string()],
            has_daily_checkin: true,
            criterion_ids: vec!["crit-cheat".to_string(), "crit-proxy".to_string()],
            login_method_ids: vec![],
            notes: "Treo máy lấy point, cần proxy sạch".to_string(),
            created_at: now.clone(),
        },
        Website {
            id: "web-3".to_string(),
            name: "Discord".to_string(),
            url: "https://discord.com".to_string(),
            tags: vec!["Social / Community".to_string()],
            has_daily_checkin: false,
            criterion_ids: vec!["crit-cheat".to_string()],
            login_method_ids: vec![],
            notes: "Tham gia các server dự án".to_string(),
            created_at: now.clone(),
        },
    ];

    let sample_regs = vec![
        RegistrationRecord {
            id: "reg-1".to_string(),
            email_id: "em-1".to_string(),
            website_id: "web-1".to_string(),
            is_registered: true,
            status: crate::models::AccountStatus::Live,
            is_checked_in: true,
            registered_at: Some(now.clone()),
            checkin_streak: 12,
            last_checkin_at: Some(now.clone()),
            notes: "Đã KYC xong".to_string(),
        },
        RegistrationRecord {
            id: "reg-2".to_string(),
            email_id: "em-2".to_string(),
            website_id: "web-2".to_string(),
            is_registered: true,
            status: crate::models::AccountStatus::Live,
            is_checked_in: false,
            registered_at: Some(now.clone()),
            checkin_streak: 44,
            last_checkin_at: Some(yesterday),
            notes: "Chạy proxy US".to_string(),
        },
    ];

    AppDatabase {
        emails: sample_emails,
        websites: sample_websites,
        registrations: sample_regs,
        settings: AppSettings::default(),
        criteria: seed_criteria(&now),
        login_methods: seed_login_methods(&now),
        schema_version: CURRENT_SCHEMA_VERSION,
    }
}

// Ngày điểm danh (theo giờ local) từ timestamp RFC3339
pub fn checkin_date(ts: &str) -> Option<chrono::NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Local).date_naive())
}

// Reset cờ is_checked_in nếu lần điểm danh gần nhất không còn là hôm nay
// (đảm bảo trạng thái "điểm danh hôm nay" tự sai khi sang ngày mới)
pub fn normalize_checkins(db: &mut AppDatabase) {
    let today = chrono::Local::now().date_naive();
    for rec in db.registrations.iter_mut() {
        if rec.is_checked_in
            && rec.last_checkin_at.as_deref().and_then(checkin_date) != Some(today)
        {
            rec.is_checked_in = false;
        }
    }
}

/// Version declared in file; missing `schema_version` means v1 (back-compat).
pub fn schema_version_of(raw: &serde_json::Value) -> u32 {
    raw.get("schema_version")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(1)
}

/// Enforce loader window [CURRENT-2, CURRENT]; newer/older → explicit Err.
pub fn check_version_window(version: u32) -> Result<(), String> {
    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "File dữ liệu schema v{version} mới hơn app (hỗ trợ tới v{CURRENT_SCHEMA_VERSION}). Hãy cập nhật app rồi mở lại."
        ));
    }
    if version + 2 < CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "File dữ liệu schema v{version} quá cũ (app hỗ trợ từ v{}). Hãy migrate thủ công hoặc tạo dữ liệu mới.",
            CURRENT_SCHEMA_VERSION - 2
        ));
    }
    Ok(())
}

/// Migrate a parsed v1 document to v2 (pure, no FS side effects).
/// Transform: category→tag; custom entry→tag "label: value";
/// cheat/kyc/proxy flags → seed criteria + assignment; login_method_ids=[].
pub fn migrate_v1_value(raw: &serde_json::Value) -> Result<AppDatabase, String> {
    let v1: legacy_v1::DatabaseV1 =
        serde_json::from_value(raw.clone()).map_err(|e| format!("File v1 không đúng định dạng: {e}"))?;
    let now = chrono::Local::now().to_rfc3339();
    let mut criteria = seed_criteria(&now);
    // Only keep seeds actually referenced? Contract seeds all 3; keep all for stable ids.
    let _ = &mut criteria;
    let websites: Vec<Website> = v1
        .websites
        .into_iter()
        .map(|w| {
            let mut tags: Vec<String> = Vec::new();
            if !w.category.trim().is_empty() {
                tags.push(w.category.trim().to_string());
            }
            for c in &w.custom_criteria {
                let label = c.label.trim();
                let value = c.value.trim();
                if label.is_empty() && value.is_empty() {
                    continue;
                }
                tags.push(format!("{label}: {value}"));
            }
            let mut criterion_ids: Vec<String> = Vec::new();
            if w.can_cheat_account {
                criterion_ids.push("crit-cheat".to_string());
            }
            if w.requires_kyc {
                criterion_ids.push("crit-kyc".to_string());
            }
            if w.requires_proxy {
                criterion_ids.push("crit-proxy".to_string());
            }
            Website {
                id: w.id,
                name: w.name,
                url: w.url,
                tags,
                has_daily_checkin: w.has_daily_checkin,
                criterion_ids,
                login_method_ids: vec![],
                notes: w.notes,
                created_at: w.created_at,
            }
        })
        .collect();
    Ok(AppDatabase {
        emails: v1.emails,
        websites,
        registrations: v1.registrations,
        settings: v1.settings.unwrap_or_default(),
        criteria,
        login_methods: seed_login_methods(&now),
        schema_version: CURRENT_SCHEMA_VERSION,
    })
}

/// Copy `file_path` to `accounts_data.json.bak.YYYYMMDD-HHMMSS` BEFORE migrate.
/// Returns the backup path. Pure FS helper — unit-testable without global cache.
pub fn backup_file_before_migrate(file_path: &Path) -> Result<PathBuf, String> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let file_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("accounts_data.json");
    let backup = file_path.with_file_name(format!("{file_name}.bak.{stamp}"));
    fs::copy(file_path, &backup)
        .map_err(|e| format!("Không tạo được backup {}: {e}", backup.display()))?;
    Ok(backup)
}

fn parse_versioned_content(content: &str, file_path: &Path) -> Result<AppDatabase, String> {
    // Re-parse with proper error path for corrupt files (keep .corrupt flow).
    let raw: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            let backup = file_path.with_extension("json.corrupt");
            let _ = fs::copy(file_path, &backup);
            log::error!("Database file is corrupt, backed up to {}", backup.display());
            return Err(format!(
                "File dữ liệu bị hỏng ({e}). Bản gốc đã được sao lưu sang {}. Hãy khôi phục hoặc xóa file này rồi khởi động lại app.",
                backup.display()
            ));
        }
    };
    let version = schema_version_of(&raw);
    check_version_window(version)?;
    if version == CURRENT_SCHEMA_VERSION {
        let mut parsed: AppDatabase =
            serde_json::from_value(raw).map_err(|e| format!("File dữ liệu v{version} không đúng định dạng: {e}"))?;
        parsed.schema_version = CURRENT_SCHEMA_VERSION;
        normalize_checkins(&mut parsed);
        return Ok(parsed);
    }
    // v1 → v2 migration (backup BEFORE write).
    backup_file_before_migrate(file_path)?;
    let mut migrated = migrate_v1_value(&raw)?;
    normalize_checkins(&mut migrated);
    let json = serde_json::to_string_pretty(&migrated).map_err(|e| e.to_string())?;
    let tmp_path = file_path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Không ghi được file tạm {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, file_path)
        .map_err(|e| format!("Không lưu được {}: {e}", file_path.display()))?;
    Ok(migrated)
}

pub fn load_db() -> Result<AppDatabase, String> {
    let mut guard = DB_INSTANCE.lock().unwrap();
    if let Some(db) = &*guard {
        return Ok(db.clone());
    }

    let file_path = db_file_path();
    let db = if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Không đọc được file dữ liệu {}: {e}", file_path.display()))?;
        parse_versioned_content(&content, &file_path)?
    } else {
        let default_db = init_sample_data();
        let _ = fs::write(&file_path, serde_json::to_string_pretty(&default_db).unwrap_or_default());
        default_db
    };

    *guard = Some(db.clone());
    Ok(db)
}

pub fn save_db(db: &AppDatabase) -> Result<(), String> {
    let mut guard = DB_INSTANCE.lock().unwrap();
    let mut to_save = db.clone();
    to_save.schema_version = CURRENT_SCHEMA_VERSION;
    let file_path = db_file_path();
    let json = serde_json::to_string_pretty(&to_save).map_err(|e| e.to_string())?;
    // Ghi file tạm rồi rename để tránh hỏng file dữ liệu nếu app crash giữa chừng
    let tmp_path = file_path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Không ghi được file tạm {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Không lưu được {}: {e}", file_path.display()))?;
    *guard = Some(to_save);
    Ok(())
}

#[cfg(test)]
pub fn test_reset_cache() -> std::sync::MutexGuard<'static, Option<AppDatabase>> {
    let mut guard = DB_INSTANCE.lock().unwrap();
    *guard = None;
    guard
}
