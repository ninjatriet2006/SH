pub mod models;
pub mod storage;

use models::{AppDatabase, AppSettings, EmailAccount, RegistrationRecord, Website};
use std::fs;

#[tauri::command]
fn get_all_data() -> Result<AppDatabase, String> {
    let mut db = storage::load_db()?;
    // Chuẩn hóa lại mỗi lần đọc để xử lý app chạy qua ngày mới (cache trong phiên có thể cũ)
    storage::normalize_checkins(&mut db);
    Ok(db)
}

#[tauri::command]
fn save_email(email: EmailAccount) -> Result<EmailAccount, String> {
    let mut db = storage::load_db()?;
    let mut saved = email.clone();
    if saved.id.is_empty() {
        saved.id = format!("em-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        saved.created_at = chrono::Local::now().to_rfc3339();
        db.emails.push(saved.clone());
    } else {
        if let Some(pos) = db.emails.iter().position(|e| e.id == saved.id) {
            db.emails[pos] = saved.clone();
        } else {
            db.emails.push(saved.clone());
        }
    }
    storage::save_db(&db)?;
    Ok(saved)
}

#[tauri::command]
fn delete_email(id: String) -> Result<(), String> {
    let mut db = storage::load_db()?;
    db.emails.retain(|e| e.id != id);
    db.registrations.retain(|r| r.email_id != id);
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn save_website(website: Website) -> Result<Website, String> {
    let mut db = storage::load_db()?;
    let mut saved = website.clone();
    if saved.id.is_empty() {
        saved.id = format!("web-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
        saved.created_at = chrono::Local::now().to_rfc3339();
        db.websites.push(saved.clone());
    } else {
        if let Some(pos) = db.websites.iter().position(|w| w.id == saved.id) {
            db.websites[pos] = saved.clone();
        } else {
            db.websites.push(saved.clone());
        }
    }
    storage::save_db(&db)?;
    Ok(saved)
}

#[tauri::command]
fn delete_website(id: String) -> Result<(), String> {
    let mut db = storage::load_db()?;
    db.websites.retain(|w| w.id != id);
    db.registrations.retain(|r| r.website_id != id);
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn toggle_registration(email_id: String, website_id: String) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db()?;
    let now = chrono::Local::now().to_rfc3339();
    let record = if let Some(pos) = db.registrations.iter().position(|r| r.email_id == email_id && r.website_id == website_id) {
        let current = &mut db.registrations[pos];
        current.is_registered = !current.is_registered;
        if current.is_registered && current.registered_at.is_none() {
            current.registered_at = Some(now);
        }
        current.clone()
    } else {
        let new_rec = RegistrationRecord {
            id: format!("reg-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            email_id,
            website_id,
            is_registered: true,
            status: models::AccountStatus::Live,
            is_checked_in: false,
            registered_at: Some(now),
            checkin_streak: 0,
            last_checkin_at: None,
            notes: "".to_string(),
        };
        db.registrations.push(new_rec.clone());
        new_rec
    };
    storage::save_db(&db)?;
    Ok(record)
}

#[tauri::command]
fn set_registration_status(email_id: String, website_id: String, status: models::AccountStatus) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db()?;
    let now = chrono::Local::now().to_rfc3339();
    let record = if let Some(pos) = db.registrations.iter().position(|r| r.email_id == email_id && r.website_id == website_id) {
        let current = &mut db.registrations[pos];
        current.status = status;
        current.clone()
    } else {
        let new_rec = RegistrationRecord {
            id: format!("reg-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            email_id,
            website_id,
            is_registered: true,
            status,
            is_checked_in: false,
            registered_at: Some(now),
            checkin_streak: 0,
            last_checkin_at: None,
            notes: "".to_string(),
        };
        db.registrations.push(new_rec.clone());
        new_rec
    };
    storage::save_db(&db)?;
    Ok(record)
}

#[tauri::command]
fn toggle_checkin(email_id: String, website_id: String) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db()?;
    let now = chrono::Local::now().to_rfc3339();
    let today = chrono::Local::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let record = if let Some(pos) = db.registrations.iter().position(|r| r.email_id == email_id && r.website_id == website_id) {
        let current = &mut db.registrations[pos];
        let last_checkin_date = current.last_checkin_at.as_deref().and_then(storage::checkin_date);
        if current.is_checked_in && last_checkin_date == Some(today) {
            // Hủy điểm danh trong hôm nay: giữ last_checkin_at để bảo toàn lịch sử
            current.is_checked_in = false;
            current.checkin_streak = current.checkin_streak.saturating_sub(1);
        } else {
            // Điểm danh mới: nối streak nếu lần gần nhất là hôm qua (liên tục)
            // hoặc hôm nay (điểm lại sau khi vừa hủy); ngược lại streak tính lại từ 1
            current.checkin_streak = if last_checkin_date == Some(today) || last_checkin_date == Some(yesterday) {
                current.checkin_streak + 1
            } else {
                1
            };
            current.is_checked_in = true;
            current.last_checkin_at = Some(now);
        }
        current.clone()
    } else {
        let new_rec = RegistrationRecord {
            id: format!("reg-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            email_id,
            website_id,
            is_registered: true,
            status: models::AccountStatus::Live,
            is_checked_in: true,
            registered_at: Some(now.clone()),
            checkin_streak: 1,
            last_checkin_at: Some(now),
            notes: "".to_string(),
        };
        db.registrations.push(new_rec.clone());
        new_rec
    };
    storage::save_db(&db)?;
    Ok(record)
}

#[tauri::command]
fn unlink_registration(email_id: String, website_id: String) -> Result<(), String> {
    let mut db = storage::load_db()?;
    db.registrations.retain(|r| !(r.email_id == email_id && r.website_id == website_id));
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn update_registration(record: RegistrationRecord) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db()?;
    if let Some(pos) = db.registrations.iter().position(|r| r.id == record.id) {
        db.registrations[pos] = record.clone();
    } else {
        db.registrations.push(record.clone());
    }
    storage::save_db(&db)?;
    Ok(record)
}

#[tauri::command]
fn get_settings() -> Result<AppSettings, String> {
    let db = storage::load_db()?;
    Ok(db.settings)
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let mut db = storage::load_db()?;
    db.settings = settings;
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn get_available_languages() -> Result<Vec<serde_json::Value>, String> {
    let base = storage::resource_base();
    let langs_dir = base.join("langs");
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(langs_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        list.push(val);
                    }
                }
            }
        }
    }
    Ok(list)
}

#[tauri::command]
fn get_available_themes() -> Result<Vec<serde_json::Value>, String> {
    let base = storage::resource_base();
    let themes_dir = base.join("themes");
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        list.push(val);
                    }
                }
            }
        }
    }
    Ok(list)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            get_all_data,
            save_email,
            delete_email,
            save_website,
            delete_website,
            toggle_registration,
            set_registration_status,
            toggle_checkin,
            unlink_registration,
            update_registration,
            get_settings,
            save_settings,
            get_available_languages,
            get_available_themes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkin_streak_lifecycle() {
        let dir = std::env::temp_dir().join(format!("account_hub_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("langs")).unwrap();
        std::fs::write(dir.join("langs/vi.json"), "{}").unwrap();
        std::env::set_var("ACCOUNT_HUB_RESOURCE_DIR", &dir);

        // 1. Điểm danh lần đầu: streak = 1
        let rec = toggle_checkin("em-x".into(), "web-x".into()).unwrap();
        assert!(rec.is_checked_in && rec.checkin_streak == 1);

        // 2. Hủy trong cùng ngày: streak về 0, giữ last_checkin_at
        let rec = toggle_checkin("em-x".into(), "web-x".into()).unwrap();
        assert!(!rec.is_checked_in && rec.checkin_streak == 0);

        // 3. Điểm lại trong cùng ngày: streak khôi phục = 1
        let rec = toggle_checkin("em-x".into(), "web-x".into()).unwrap();
        assert!(rec.is_checked_in && rec.checkin_streak == 1);

        // 4. Giả lập app chạy qua đêm (last = hôm qua, streak 5, cờ cũ) → điểm danh nối streak = 6
        let yesterday = (chrono::Local::now() - chrono::Duration::days(1)).to_rfc3339();
        {
            let mut db = storage::load_db().unwrap();
            let r = db.registrations.iter_mut().find(|r| r.email_id == "em-x").unwrap();
            r.last_checkin_at = Some(yesterday);
            r.checkin_streak = 5;
            r.is_checked_in = true;
            storage::save_db(&db).unwrap();
        }
        let rec = toggle_checkin("em-x".into(), "web-x".into()).unwrap();
        assert!(rec.is_checked_in && rec.checkin_streak == 6);

        // 5. Bỏ sót vài ngày (last = 3 ngày trước) → streak tính lại từ 1
        let three_days_ago = (chrono::Local::now() - chrono::Duration::days(3)).to_rfc3339();
        {
            let mut db = storage::load_db().unwrap();
            let r = db.registrations.iter_mut().find(|r| r.email_id == "em-x").unwrap();
            r.last_checkin_at = Some(three_days_ago.clone());
            r.checkin_streak = 5;
            r.is_checked_in = false;
            storage::save_db(&db).unwrap();
        }
        let rec = toggle_checkin("em-x".into(), "web-x".into()).unwrap();
        assert!(rec.is_checked_in && rec.checkin_streak == 1);

        // 6. Cờ is_checked_in cũ (không phải hôm nay) → get_all_data tự reset
        {
            let mut db = storage::load_db().unwrap();
            let r = db.registrations.iter_mut().find(|r| r.email_id == "em-x").unwrap();
            r.last_checkin_at = Some(three_days_ago);
            r.is_checked_in = true;
            storage::save_db(&db).unwrap();
        }
        let db = get_all_data().unwrap();
        let r = db.registrations.iter().find(|r| r.email_id == "em-x").unwrap();
        assert!(!r.is_checked_in);

        // 7. File dữ liệu hỏng → báo lỗi và sao lưu .corrupt
        {
            let mut db = storage::load_db().unwrap();
            db.registrations.clear();
            storage::save_db(&db).unwrap();
        }
        drop(storage::test_reset_cache());
        std::fs::write(dir.join("storage/accounts_data.json"), "NOT JSON").unwrap();
        let err = storage::load_db().unwrap_err();
        assert!(err.contains("hỏng"));
        assert!(dir.join("storage/accounts_data.json.corrupt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
