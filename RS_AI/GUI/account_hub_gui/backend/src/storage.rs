use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use crate::models::{AppDatabase, EmailAccount, Website, RegistrationRecord, AppSettings};

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
            category: "Crypto / Exchange".to_string(),
            has_daily_checkin: true,
            can_cheat_account: false,
            requires_kyc: true,
            requires_proxy: false,
            custom_criteria: vec![
                crate::models::CustomCriterion {
                    key: "tier".to_string(),
                    label: "Hạng tài khoản".to_string(),
                    value_type: "text".to_string(),
                    value: "VIP 1".to_string(),
                }
            ],
            notes: "Điểm danh Task Center hàng ngày".to_string(),
            created_at: now.clone(),
        },
        Website {
            id: "web-2".to_string(),
            name: "Grass Network".to_string(),
            url: "https://app.getgrass.io".to_string(),
            category: "DePIN / Bandwidth".to_string(),
            has_daily_checkin: true,
            can_cheat_account: true,
            requires_kyc: false,
            requires_proxy: true,
            custom_criteria: vec![
                crate::models::CustomCriterion {
                    key: "node_count".to_string(),
                    label: "Số node tối đa".to_string(),
                    value_type: "text".to_string(),
                    value: "5".to_string(),
                }
            ],
            notes: "Treo máy lấy point, cần proxy sạch".to_string(),
            created_at: now.clone(),
        },
        Website {
            id: "web-3".to_string(),
            name: "Discord".to_string(),
            url: "https://discord.com".to_string(),
            category: "Social / Community".to_string(),
            has_daily_checkin: false,
            can_cheat_account: true,
            requires_kyc: false,
            requires_proxy: false,
            custom_criteria: vec![],
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

pub fn load_db() -> Result<AppDatabase, String> {
    let mut guard = DB_INSTANCE.lock().unwrap();
    if let Some(db) = &*guard {
        return Ok(db.clone());
    }

    let file_path = db_file_path();
    let db = if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Không đọc được file dữ liệu {}: {e}", file_path.display()))?;
        match serde_json::from_str::<AppDatabase>(&content) {
            Ok(mut parsed) => {
                normalize_checkins(&mut parsed);
                parsed
            }
            Err(e) => {
                // Giữ lại bản gốc để người dùng khôi phục thay vì mất dữ liệu âm thầm
                let backup = file_path.with_extension("json.corrupt");
                let _ = fs::copy(&file_path, &backup);
                log::error!("Database file is corrupt, backed up to {}", backup.display());
                return Err(format!(
                    "File dữ liệu bị hỏng ({e}). Bản gốc đã được sao lưu sang {}. Hãy khôi phục hoặc xóa file này rồi khởi động lại app.",
                    backup.display()
                ));
            }
        }
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
    let file_path = db_file_path();
    let json = serde_json::to_string_pretty(db).map_err(|e| e.to_string())?;
    // Ghi file tạm rồi rename để tránh hỏng file dữ liệu nếu app crash giữa chừng
    let tmp_path = file_path.with_extension("json.tmp");
    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Không ghi được file tạm {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Không lưu được {}: {e}", file_path.display()))?;
    *guard = Some(db.clone());
    Ok(())
}

#[cfg(test)]
pub fn test_reset_cache() -> std::sync::MutexGuard<'static, Option<AppDatabase>> {
    let mut guard = DB_INSTANCE.lock().unwrap();
    *guard = None;
    guard
}
