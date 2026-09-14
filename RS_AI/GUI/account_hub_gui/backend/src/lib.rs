pub mod models;
pub mod storage;

use models::{AppDatabase, AppSettings, EmailAccount, RegistrationRecord, Website};
use std::fs;

#[tauri::command]
fn get_all_data() -> Result<AppDatabase, String> {
    Ok(storage::load_db())
}

#[tauri::command]
fn save_email(email: EmailAccount) -> Result<EmailAccount, String> {
    let mut db = storage::load_db();
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
    let mut db = storage::load_db();
    db.emails.retain(|e| e.id != id);
    db.registrations.retain(|r| r.email_id != id);
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn save_website(website: Website) -> Result<Website, String> {
    let mut db = storage::load_db();
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
    let mut db = storage::load_db();
    db.websites.retain(|w| w.id != id);
    db.registrations.retain(|r| r.website_id != id);
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn toggle_registration(email_id: String, website_id: String) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db();
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
    let mut db = storage::load_db();
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
fn unlink_registration(email_id: String, website_id: String) -> Result<(), String> {
    let mut db = storage::load_db();
    db.registrations.retain(|r| !(r.email_id == email_id && r.website_id == website_id));
    storage::save_db(&db)?;
    Ok(())
}

#[tauri::command]
fn update_registration(record: RegistrationRecord) -> Result<RegistrationRecord, String> {
    let mut db = storage::load_db();
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
    let db = storage::load_db();
    Ok(db.settings)
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let mut db = storage::load_db();
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
