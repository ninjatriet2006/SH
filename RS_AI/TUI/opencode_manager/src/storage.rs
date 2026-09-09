/*
[INTEGRITY NOTES]
- Mục đích: Hạ tầng lưu trữ DỮ LIỆU RIÊNG của manager (khác dữ liệu của
  opencode): thư mục chuẩn `.config/opencode-manager/`, ghi an toàn (atomic),
  backup xoay vòng (không xoá dữ liệu cũ), và migration từ vị trí cũ.
- Trách nhiệm: 3 hàm thuần + 1 hằng số — không biết gì về nội dung file.
- Tương tác: `config.rs` (CkeyConfig), `arbiter.rs` (ArbiterConfig), GUI
  `api/settings.rs` (GuiSettings).

PHÂN ĐỊNH QUYỀN SỞ HỮU FILE (nguyên tắc không vi phạm):
  - CỦA OPENCODE (đừng bao giờ di chuyển/xoá): `~/.config/opencode/opencode.json`,
    `~/.local/share/opencode/auth.json`, `~/.cache/opencode/models.json`.
  - CỦA MANAGER (chuyển sang `.config/opencode-manager/`): ckey.json,
    arbiter.json, settings.json (GUI).

CƠ CHẾ CHỐNG MẤT DỮ LIỆU (3 lớp):
  1. GHI ATOMIC: ghi file tạm cùng thư mục rồi rename — crash giữa chừng
     không bao giờ để lại file rỗng/nửa vời (rename cùng filesystem là
     nguyên tử trên Linux/macOS/Windows).
  2. BACKUP XOAY VÒNG: trước khi ghi đè, copy file cũ thành `.bak_<timestamp>`;
     giữ tối đa `BACKUP_KEEP` bản mới nhất — đủ lùi vài bước mà không phình
     vô hạn (bản cũ hơn BỊ XOÁ trong vòng xoay — đó là giới hạn bùn, không
     phải xoá dữ liệu người dùng: bản "gần nhất" luôn còn).
  3. MIGRATION KHÔNG XOÁ: chuyển từ vị trí cũ sang vị trí mới bằng copy sang
     mới + rename bản cũ thành `.legacy` (nguyên vẹn, chỉ đổi tên để không
     đọc lại). Không bao giờ `remove_file` dữ liệu nguồn.
*/

use std::fs;
use std::path::{Path, PathBuf};

/// Số bản backup giữ lại cho mỗi file (xoay vòng).
pub const BACKUP_KEEP: usize = 5;

/// Thư mục dữ liệu riêng của manager: `~/.config/opencode-manager/`.
pub fn manager_config_dir() -> PathBuf {
    let home = crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    home.join(".config").join("opencode-manager")
}

/// Đường dẫn file dữ liệu manager (nằm trong thư mục chuẩn).
pub fn manager_data_path(file_name: &str) -> PathBuf {
    manager_config_dir().join(file_name)
}

/// Ghi file ATOMIC: ghi `<path>.tmp` rồi rename đè.
///
/// File tạm phải CÙNG thư mục với đích — rename khác filesystem không nguyên
/// tử. Crash giữa chừng để lại `.tmp` (vô hại) chứ đích không bao giờ hỏng.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Không tạo được thư mục {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| format!("Không ghi được file tạm {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Không đặt được file {}: {e}", path.display())
    })
}

/// Backup xoay vòng: copy `path` → `<path>.bak_<timestamp>` rồi xoá các bản
/// cũ hơn `keep` bản mới nhất.
///
/// Hậu tố dùng nano-giây: hai lần save trong cùng giây vẫn ra tên khác nhau.
pub fn backup_rotate(path: &Path, keep: usize) {
    if !path.exists() {
        return;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let backup = path.with_extension(format!("json.bak_{ts}"));
    if fs::copy(path, &backup).is_err() {
        return; // backup lỗi thì vẫn ghi tiếp — an toàn hơn chặn cả save.
    }
    rotate_clean(path, keep);
}

/// Xoá các `.bak_*` cũ nhất sao cho chỉ còn `keep` bản.
fn rotate_clean(path: &Path, keep: usize) {
    let Some(dir) = path.parent() else { return };
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("json");
    let prefix = format!("{stem}.{ext}.bak_");

    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut backups: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();
    if backups.len() <= keep {
        return;
    }
    // Tên backup có sắp theo thời gian (nano tăng dần) → sort theo tên.
    backups.sort();
    let excess = backups.len() - keep;
    for old in backups.into_iter().take(excess) {
        let _ = fs::remove_file(old);
    }
}

/// Migration KHÔNG XOÁ: nếu `new` chưa có mà `old` có → copy old sang new,
/// rồi đổi tên old thành `<old>.legacy` (nguyên vẹn, không đọc lại nữa).
///
/// Trả về `true` nếu đã migrate. Bị lỗi ở bước nào cũng KHÔNG làm mất old.
pub fn migrate_legacy(old: &Path, new: &Path) -> bool {
    if new.exists() || !old.exists() {
        return false;
    }
    if let Some(parent) = new.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    if fs::copy(old, new).is_err() {
        return false; // old còn nguyên.
    }
    // Đổi tên old để lần sau không migrate lại; lỗi thì lần sau copy lại
    // (new đã có → bỏ qua) — vẫn an toàn.
    let _ = fs::rename(old, old.with_extension("json.legacy"));
    true
}

/// Đọc nội dung file nếu tồn tại (None nếu không có). Tiện ích cho các
/// config mới (GUI settings) — TUI chưa dùng.
#[allow(dead_code)]
pub fn read_if_exists(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lấy khoá env chung + tạo home test. Giá trị trả về là (guard, dir) —
    /// guard phải sống suốt thân test.
    fn locked_home(tag: &str) -> (std::sync::MutexGuard<'static, ()>, PathBuf) {
        let lock = crate::app::TEST_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("storage_test_homes")
            .join(tag);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", &dir);
        }
        (lock, dir)
    }

    /// Atomic write: file đích luôn nguyên vẹn hoặc không đổi — không bao giờ
    /// rỗng/nửa vời.
    #[test]
    fn atomic_write_tao_file_moi_va_de_cu() {
        let (_guard, home) = locked_home("atomic");
        let p = manager_data_path("t.json");
        atomic_write(&p, "v1").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "v1");
        atomic_write(&p, "v2").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "v2");
        // Không để lại file tạm.
        assert!(!p.with_extension("json.tmp").exists());
        let _ = fs::remove_dir_all(&home);
    }

    /// Backup xoay vòng: tạo đúng `keep` bản, bản cũ hơn bị dọn.
    #[test]
    fn backup_xoay_vong_giu_toi_da_keep() {
        let (_guard, home) = locked_home("rotate");
        let p = manager_data_path("r.json");
        atomic_write(&p, "v0").unwrap();

        for i in 1..=(BACKUP_KEEP + 3) {
            backup_rotate(&p, BACKUP_KEEP);
            atomic_write(&p, &format!("v{i}")).unwrap();
            // Ngủ rất ngắn để nano-giây tăng (đảm bảo tên khác nhau).
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let dir = p.parent().unwrap();
        let baks: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_str().map(|n| n.contains(".bak_")).unwrap_or(false))
            .collect();
        assert_eq!(
            baks.len(),
            BACKUP_KEEP,
            "phải giữ đúng {BACKUP_KEEP} bản (thực có {:?})",
            baks.iter().map(|e| e.file_name()).collect::<Vec<_>>()
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// Migration: copy sang mới + đổi tên cũ thành .legacy; không bao giờ
    /// xoá dữ liệu nguồn; chạy lần hai là no-op.
    #[test]
    fn migrate_khong_xoa_du_lieu() {
        let (_guard, home) = locked_home("migrate");
        let old = home.join(".config").join("opencode").join("ckey.json");
        let new = manager_data_path("ckey.json");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, "{\"accounts\":{}}").unwrap();

        // Lần 1: migrate — old thành .legacy, new có nội dung.
        assert!(migrate_legacy(&old, &new));
        assert_eq!(fs::read_to_string(&new).unwrap(), "{\"accounts\":{}}");
        assert!(old.with_extension("json.legacy").exists(), "dữ liệu cũ còn nguyên");
        assert!(!old.exists(), "old đã đổi tên");

        // Lần 2: no-op (new đã có).
        fs::write(&old, "nonsense").unwrap();
        assert!(!migrate_legacy(&old, &new));
        assert_eq!(
            fs::read_to_string(&new).unwrap(),
            "{\"accounts\":{}}",
            "new không bị đụng"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// New đã có sẵn → old không bị đụng (ưu tiên dữ liệu mới).
    #[test]
    fn migrate_new_co_san_thi_old_nguyen() {
        let (_guard, home) = locked_home("migrate2");
        let old = home.join(".config").join("opencode").join("ckey.json");
        let new = manager_data_path("ckey.json");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::create_dir_all(new.parent().unwrap()).unwrap();
        fs::write(&old, "OLD").unwrap();
        fs::write(&new, "NEW").unwrap();

        assert!(!migrate_legacy(&old, &new));
        assert_eq!(fs::read_to_string(&new).unwrap(), "NEW");
        assert_eq!(fs::read_to_string(&old).unwrap(), "OLD");
        let _ = fs::remove_dir_all(&home);
    }
}
