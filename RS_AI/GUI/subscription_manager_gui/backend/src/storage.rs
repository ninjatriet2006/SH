/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp cơ chế lưu trữ dữ liệu đơn giản bằng JSON file.
- Trách nhiệm: Đọc và ghi các danh sách User, Package, và Subscription ra file `data.json` tại thư mục gốc.
- Tương tác: Được gọi bởi các API modules để lưu trữ hoặc lấy dữ liệu bền vững (persistent data).
*/

// Import thư viện File và xử lý đường dẫn
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
// Import Serialize/Deserialize và các Models đã tạo
use crate::models::{Package, PaymentRef, Subscription, Transaction, User};
use serde::{Deserialize, Serialize};

/// Thư mục gốc chứa tài nguyên app (`langs/`, `themes/`, `fonts/`).
/// Dò MỘT LẦN rồi cache: mọi resource phải cùng một base, nếu không sẽ xảy ra
/// tình trạng `themes` lấy ở repo root mà `langs` lại không thấy (đã gặp).
static RESOURCE_BASE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// `langs/` là mốc neo (anchor) để nhận ra một thư mục có phải app root không:
/// đây là tài nguyên BẮT BUỘC (app không có nó thì UI hiện raw key), và luôn
/// được builder copy kèm. Không neo bằng `themes/`/`fonts/` vì hai thứ đó được
/// tự tạo rỗng khi thiếu nên có thể tồn tại ở thư mục bất kỳ.
const RESOURCE_ANCHOR: &str = "langs";

/// Dò thư mục gốc tài nguyên theo thứ tự:
///   1. CWD (`cargo tauri dev` với CWD đúng, hoặc user chạy từ thư mục app);
///   2. Thư mục chứa binary (layout `release/<app>/` mà GUI Builder xuất);
///   3. Các cấp cha của thư mục binary (bắt trường hợp bundle lồng thư mục);
///   4. Chỉ ở bản debug: `CARGO_MANIFEST_DIR/..` — tài nguyên nằm ở
///      `GUI/<app>/langs` còn CWD lúc `cargo tauri dev` là `GUI/<app>/backend`,
///      nên hai nhánh trên đều MISS và toàn bộ UI hiện raw key.
fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &std::path::Path| p.join(RESOURCE_ANCHOR).is_dir();

    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        // Duyệt exe_dir rồi lần lượt các cấp cha.
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            if has_anchor(dir) {
                return dir.to_path_buf();
            }
            cur = dir.parent();
        }
    }

    // Bản debug: neo theo vị trí source để `cargo tauri dev` chạy được ngay.
    #[cfg(debug_assertions)]
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    // Không tìm được: dùng CWD để hành vi ghi file vẫn như cũ (không panic).
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Thư mục gốc tài nguyên đã cache.
pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(|| {
        std::env::var_os("SUBSCRIPTION_MANAGER_RESOURCE_DIR")
            .map(PathBuf::from)
            .filter(|path| path.join(RESOURCE_ANCHOR).is_dir())
            .unwrap_or_else(detect_resource_base)
    })
}

// Tìm file `<name>` trong thư mục storage theo thứ tự ưu tiên:
//   1. `./storage/<name>` tính từ thư mục làm việc (tương thích data của
//      bản cũ — user cũ mở app từ đâu thì data vẫn ở đó, không mất);
//   2. `<gốc tài nguyên>/storage/<name>` — cùng base với langs/themes/fonts
//      nên data luôn đi kèm app, kể cả khi CWD trỏ chỗ khác.
// Trước đây chỉ dùng CWD nên double-click binary từ chỗ khác là "mất" data.
pub fn storage_file_path(name: &str) -> PathBuf {
    let rel = format!("storage/{name}");
    let cwd_path = PathBuf::from(&rel);
    if cwd_path.is_file() {
        return cwd_path;
    }
    let base = resource_base();
    // Chỉ chuyển sang base khi base thực sự là app root (có anchor); nếu không
    // thì giữ CWD như cũ để không tạo thư mục rác ở chỗ ngẫu nhiên.
    if base.join(RESOURCE_ANCHOR).is_dir() {
        return base.join(&rel);
    }
    cwd_path
}

/// Thư mục cấu hình riêng của người dùng. Có thể đổi gốc chuẩn bằng
/// `XDG_CONFIG_HOME`; nếu không có, Linux dùng `~/.config`.
///
/// Đây là nguồn dữ liệu chính, tách khỏi thư mục cài đặt/bản build để thay app,
/// xoá checkout repository hoặc mở binary từ CWD khác không làm mất dữ liệu.
pub fn config_dir() -> PathBuf {
    config_dir_from_env(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        resource_base().join("storage"),
    )
}

/// Đường dẫn canonical cho một file cấu hình do app sở hữu.
pub fn config_file_path(name: &str) -> PathBuf {
    config_dir().join(name)
}

fn config_dir_from_env(
    xdg_config_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    fallback: PathBuf,
) -> PathBuf {
    if let Some(path) = xdg_config_home {
        return PathBuf::from(path).join("subscription_manager_gui");
    }
    if let Some(home) = home {
        return PathBuf::from(home).join(".config").join("subscription_manager_gui");
    }
    // Fallback hiếm (môi trường không có HOME): vẫn ưu tiên thư mục app, không
    // tạo file ở CWD ngẫu nhiên.
    fallback
}

fn canonical_data_file_path() -> PathBuf {
    config_dir().join("data.json")
}

/// Bản sao có thể mang theo cùng app. Giữ rule resolve cũ để một `storage/`
/// hiện hữu được dùng làm nguồn migration thay vì bị bỏ quên.
fn mirror_data_file_path() -> PathBuf {
    storage_file_path("data.json")
}

/// Tìm THƯ MỤC tài nguyên `<name>` (langs/themes/fonts) — luôn lấy từ CÙNG một
/// gốc đã dò (`resource_base`). Trước đây mỗi API tự dò riêng nên dev mode miss
/// hết (UI hiện raw key) và có thể lẫn tài nguyên giữa các thư mục khác nhau.
pub fn resource_dir(name: &str) -> PathBuf {
    resource_base().join(name)
}

// Khóa toàn cục serialize mọi chuỗi đọc-sửa-ghi. Mỗi lệnh ghi giữ guard này
// suốt thời gian chạy (`let _guard = lock_store();` ở đầu command) nên hai
// `invoke` song song không thể load cùng bản cũ rồi ghi đè lẫn nhau.
// (Khóa trong tiến trình; chạy 2 bản app cùng lúc vẫn cần file-lock OS.)
static DATA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Giữ khóa storage cho toàn bộ lệnh ghi. Guard phải được bind vào biến
/// (`let _guard = ...`) để khóa tồn tại đến hết scope.
pub fn lock_store() -> std::sync::MutexGuard<'static, ()> {
    DATA_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// Cấu trúc DataStore chứa toàn bộ dữ liệu của ứng dụng.
// `Default` derive thay cho `impl` tay (clippy::derivable_impls).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataStore {
    // Danh sách người dùng
    pub users: Vec<User>,
    // Danh sách các gói dịch vụ
    pub packages: Vec<Package>,
    // Danh sách các đăng ký dịch vụ
    pub subscriptions: Vec<Subscription>,
    // Lịch sử giao dịch (có serde default để tương thích file cũ)
    #[serde(default)]
    pub transactions: Vec<Transaction>,
    // Mã tra cứu thanh toán đã phát hành khi in hóa đơn (serde default để
    // data.json của bản cũ vẫn đọc được).
    #[serde(default)]
    pub payment_refs: Vec<PaymentRef>,
}

/// Định dạng export có version riêng để sau này có thể import an toàn khi mô
/// hình dữ liệu của app thay đổi.
#[derive(Serialize)]
struct ExportBackup<'a> {
    format_version: u8,
    exported_at: u128,
    data: &'a DataStore,
    settings: crate::settings_api::Settings,
}

// Dùng `DataStore::default()` từ derive ở trên để khởi tạo rỗng.

// Hàm đọc dữ liệu từ file JSON
fn read_data_file(path: &std::path::Path) -> Option<DataStore> {
    let mut contents = String::new();
    match File::open(path).and_then(|mut file| file.read_to_string(&mut contents)) {
        Ok(_) => match serde_json::from_str(&contents) {
            Ok(data) => Some(data),
            Err(error) => {
                eprintln!("[storage] {} hỏng: {error}", path.display());
                None
            }
        },
        Err(error) => {
            eprintln!("[storage] không đọc được {}: {error}", path.display());
            None
        }
    }
}

fn load_from_backups(data_file: &std::path::Path) -> Option<DataStore> {
    let (path, data) = load_newest_backup(data_file)?;
    eprintln!(
        "[storage] đã phục hồi từ {} ({} user, {} đăng ký)",
        path.display(),
        data.users.len(),
        data.subscriptions.len()
    );
    Some(data)
}

/// Nạp theo thứ tự canonical -> mirror -> backup hai nơi. Khi dữ liệu chỉ còn
/// ở mirror (bản app cũ), ghi ngay một bản canonical để lần sau không phụ thuộc
/// vị trí binary. Không tự merge: giao dịch/số dư cần toàn vẹn hơn là đoán.
pub fn load_data() -> DataStore {
    let canonical = canonical_data_file_path();
    let mirror = mirror_data_file_path();
    load_data_from_paths(&canonical, &mirror)
}

fn sync_mirror_data(mirror: &std::path::Path, data: &DataStore) {
    let expected = match serde_json::to_string_pretty(data) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("[storage] không serialize được dữ liệu mirror: {error}");
            return;
        }
    };
    if fs::read_to_string(mirror).ok().as_deref() == Some(expected.as_str()) {
        return;
    }
    if let Err(error) = write_data_file(mirror, data) {
        eprintln!("[storage] không đồng bộ được mirror {}: {error}", mirror.display());
    }
}

fn load_data_from_paths(canonical: &std::path::Path, mirror: &std::path::Path) -> DataStore {
    if canonical.is_file() {
        if let Some(data) = read_data_file(canonical).or_else(|| load_from_backups(canonical)) {
            if mirror != canonical {
                sync_mirror_data(mirror, &data);
            }
            return data;
        }
    }

    if mirror.is_file() {
        if let Some(data) = read_data_file(mirror).or_else(|| load_from_backups(mirror)) {
            eprintln!("[storage] migrate {} -> {}", mirror.display(), canonical.display());
            if let Err(error) = write_data_file(canonical, &data) {
                eprintln!("[storage] không tạo được canonical data: {error}");
            }
            return data;
        }
    }

    DataStore::default()
}

/// Tìm bản sao lưu MỚI NHẤT còn parse được trong `storage/backups/`.
/// Duyệt từ mới về cũ vì bản mới nhất có thể cũng đã hỏng.
fn load_newest_backup(data_file: &std::path::Path) -> Option<(PathBuf, DataStore)> {
    let backup_dir = data_file.parent()?.join("backups");
    let entries = std::fs::read_dir(&backup_dir).ok()?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("data-") && n.ends_with(".json"))
        })
        .collect();
    files.sort();
    for path in files.into_iter().rev() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<DataStore>(&text) {
                return Some((path, data));
            }
            eprintln!("[storage] bản sao lưu {} cũng hỏng, thử bản cũ hơn", path.display());
        }
    }
    None
}

/// Số bản sao lưu `data.json` được giữ lại. Đủ để lùi vài bước khi phát hiện
/// sai sót muộn, nhưng không phình vô hạn.
const BACKUP_KEEP: usize = 10;

/// Sao lưu `data.json` hiện tại vào `storage/backups/` trước khi ghi bản mới.
///
/// Sao lưu ĐẶT CẠNH data (không dùng git): repo này công khai còn data.json
/// chứa thông tin khách hàng, số dư, giao dịch. `.gitignore` chặn cả
/// `storage/` lẫn `**/data.json`.
///
/// Bỏ qua im lặng nếu chưa có file (lần chạy đầu) hoặc backup thất bại — sao
/// lưu không được phép làm hỏng luồng ghi chính.
fn backup_data_file(data_file: &std::path::Path) {
    if !data_file.is_file() {
        return;
    }
    // File rỗng/0 byte không đáng sao lưu, và sao lưu nó có thể đẩy bản tốt ra
    // khỏi hạn mức giữ lại.
    if data_file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        return;
    }

    let Some(dir) = data_file.parent() else { return };
    let backup_dir = dir.join("backups");
    if fs::create_dir_all(&backup_dir).is_err() {
        return;
    }

    // Tên theo timestamp mili-giây: sắp xếp theo tên = sắp theo thời gian, và
    // hai lần ghi trong cùng giây không ghi đè nhau.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest = backup_dir.join(format!("data-{stamp}.json"));
    if fs::copy(data_file, &dest).is_err() {
        return;
    }

    prune_backups(&backup_dir);
}

/// Xoá bản sao lưu cũ, chỉ giữ `BACKUP_KEEP` bản mới nhất.
fn prune_backups(backup_dir: &std::path::Path) {
    let Ok(entries) = fs::read_dir(backup_dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("data-") && n.ends_with(".json"))
        })
        .collect();
    if files.len() <= BACKUP_KEEP {
        return;
    }
    // Sắp theo tên = theo timestamp (tên có độ dài cố định trong cùng thời đại).
    files.sort();
    let excess = files.len() - BACKUP_KEEP;
    for old in files.into_iter().take(excess) {
        let _ = fs::remove_file(old);
    }
}

// Hàm ghi dữ liệu xuống file JSON
fn write_data_file(data_file: &std::path::Path, data: &DataStore) -> Result<(), String> {
    // Chuyển đối tượng DataStore thành chuỗi JSON với định dạng dễ đọc (pretty)
    let json = match serde_json::to_string_pretty(data) {
        Ok(j) => j,
        Err(e) => return Err(format!("Lỗi chuyển đổi dữ liệu thành JSON: {}", e)),
    };

    // Đảm bảo thư mục cha tồn tại
    if let Some(parent) = data_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Lỗi tạo thư mục lưu trữ {}: {e}", parent.display()))?;
    }

    // Sao lưu bản hiện tại TRƯỚC khi ghi đè.
    backup_data_file(data_file);

    // Ghi qua file tạm rồi `rename`: `File::create` truncate ngay lập tức, nên
    // nếu tiến trình chết giữa lúc ghi (hoặc hết đĩa) thì data.json còn lại là
    // file rỗng/một nửa → mất trắng dữ liệu khách. `rename` trong cùng thư mục
    // là nguyên tử trên POSIX, đọc song song luôn thấy bản cũ hoặc bản mới trọn vẹn.
    let tmp_file = data_file.with_extension("json.tmp");
    {
        let mut file = match File::create(&tmp_file) {
            Ok(f) => f,
            Err(e) => return Err(format!("Lỗi tạo file lưu trữ tạm: {}", e)),
        };
        if let Err(e) = file.write_all(json.as_bytes()) {
            let _ = fs::remove_file(&tmp_file);
            return Err(format!("Lỗi ghi dữ liệu vào file: {}", e));
        }
        // `sync_all` để dữ liệu thực sự xuống đĩa trước khi rename — mất điện
        // ngay sau rename vẫn còn nội dung, không phải file rỗng.
        if let Err(e) = file.sync_all() {
            let _ = fs::remove_file(&tmp_file);
            return Err(format!("Lỗi đồng bộ dữ liệu xuống đĩa: {}", e));
        }
    }

    if let Err(e) = fs::rename(&tmp_file, data_file) {
        let _ = fs::remove_file(&tmp_file);
        return Err(format!("Lỗi thay thế file lưu trữ: {}", e));
    }

    Ok(())
}

/// Ghi canonical trước, sau đó mirror vào `storage/`. Mỗi đích tự backup trước
/// khi thay file, nên dữ liệu vẫn an toàn khi máy tắt đột ngột giữa hai lượt ghi.
/// Lỗi mirror được log nhưng không báo thao tác nghiệp vụ thất bại sau khi bản
/// canonical đã ghi bền vững: retry cùng thao tác có thể tạo giao dịch trùng.
pub fn save_data(data: &DataStore) -> Result<(), String> {
    let canonical = canonical_data_file_path();
    let mirror = mirror_data_file_path();
    save_data_to_paths(&canonical, &mirror, data)
}

/// Xuất snapshot dữ liệu và cài đặt tại đường dẫn người dùng đã chọn.
/// Không thay đổi data ứng dụng hoặc tạo backup xoay vòng nội bộ.
#[tauri::command(rename_all = "snake_case")]
pub fn export_backup(destination: String) -> Result<(), String> {
    let destination = PathBuf::from(destination);
    if destination.as_os_str().is_empty() || destination.is_dir() {
        return Err("Vui lòng chọn một file backup hợp lệ".to_string());
    }

    let _guard = lock_store();
    let data = load_data();
    let backup = ExportBackup {
        format_version: 1,
        exported_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0),
        data: &data,
        settings: crate::settings_api::get_settings()?,
    };
    let json =
        serde_json::to_string_pretty(&backup).map_err(|error| format!("Không thể tạo nội dung backup: {error}"))?;
    write_export_file(&destination, &json)
}

fn write_export_file(path: &std::path::Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Đường dẫn backup không hợp lệ: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Không thể tạo thư mục backup {}: {error}", parent.display()))?;

    let temporary = path.with_extension("backup.tmp");
    {
        let mut file = File::create(&temporary).map_err(|error| format!("Không thể tạo file backup tạm: {error}"))?;
        file.write_all(content.as_bytes())
            .map_err(|error| format!("Không thể ghi backup: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Không thể đồng bộ backup xuống đĩa: {error}"))?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("Không thể hoàn tất backup: {error}")
    })
}

fn save_data_to_paths(canonical: &std::path::Path, mirror: &std::path::Path, data: &DataStore) -> Result<(), String> {
    write_data_file(canonical, data)?;
    if mirror != canonical {
        if let Err(error) = write_data_file(mirror, data) {
            eprintln!(
                "[storage] canonical đã lưu nhưng không mirror được {}: {error}",
                mirror.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bug đã gặp: `cargo tauri dev` đặt CWD = `backend/` và binary nằm ở
    /// `target/debug/`, cả hai đều không có `langs/` nên từ điển rỗng và toàn
    /// bộ UI hiện raw key ("sidebar.dashboard" thay vì "Tổng quan").
    /// Test này chốt: base dò được PHẢI chứa anchor `langs/`.
    #[test]
    fn resource_base_co_chua_langs() {
        let base = resource_base();
        assert!(
            base.join(RESOURCE_ANCHOR).is_dir(),
            "resource_base() = {} không chứa {}/ — langs sẽ rỗng và UI hiện raw key",
            base.display(),
            RESOURCE_ANCHOR
        );
    }

    /// Các file ngôn ngữ thực tế phải đọc được qua đường dẫn đã dò.
    /// Không cố định `vi.json`: quét file `.json` đầu tiên có trong `langs/` để
    /// test không phụ thuộc một bộ ngôn ngữ cụ thể.
    #[test]
    fn doc_duoc_file_ngon_ngu_dau_tien() {
        let langs = resource_dir("langs");
        let mut files: Vec<_> = std::fs::read_dir(&langs)
            .unwrap_or_else(|e| panic!("đọc {}: {e}", langs.display()))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect();
        files.sort();
        let first = files
            .first()
            .unwrap_or_else(|| panic!("không có file .json nào trong {}", langs.display()));

        let content = std::fs::read_to_string(first).expect("đọc file ngôn ngữ");
        let json: serde_json::Value = serde_json::from_str(&content).expect("file ngôn ngữ phải là JSON hợp lệ");
        assert!(
            json.is_object() && json.as_object().is_some_and(|m| !m.is_empty()),
            "{} phải là object không rỗng",
            first.display()
        );
    }

    /// Mọi resource phải cùng một gốc — trước đây `themes` lấy ở repo root mà
    /// `langs` lại MISS, gây trạng thái nửa vời khó lần ra.
    #[test]
    fn moi_resource_dung_chung_mot_goc() {
        let base = resource_base();
        for name in ["langs", "themes", "fonts"] {
            assert_eq!(
                resource_dir(name),
                base.join(name),
                "{name} không cùng gốc với các resource khác"
            );
        }
    }

    /// `storage/` phải nằm cùng gốc với tài nguyên, không rơi ra thư mục ngẫu
    /// nhiên (bug cũ: double-click từ $HOME sinh ra `~/storage`).
    #[test]
    fn storage_di_kem_app_root() {
        let p = storage_file_path("data.json");
        let base = resource_base();
        assert!(
            p == std::path::Path::new("storage/data.json") || p.starts_with(base),
            "storage path {} không thuộc app root {}",
            p.display(),
            base.display()
        );
    }

    #[test]
    fn config_dir_uu_tien_xdg_roi_den_home() {
        let fallback = PathBuf::from("/app/storage");
        assert_eq!(
            config_dir_from_env(Some("/xdg".into()), Some("/home/user".into()), fallback.clone()),
            PathBuf::from("/xdg/subscription_manager_gui")
        );
        assert_eq!(
            config_dir_from_env(None, Some("/home/user".into()), fallback.clone()),
            PathBuf::from("/home/user/.config/subscription_manager_gui")
        );
        assert_eq!(config_dir_from_env(None, None, fallback.clone()), fallback);
    }

    #[test]
    fn mirror_cu_duoc_migrate_sang_canonical() {
        let root = std::env::temp_dir().join(format!("subscription-manager-storage-migrate-{}", std::process::id()));
        let canonical = root.join("config/data.json");
        let mirror = root.join("app/storage/data.json");
        let expected = DataStore {
            users: vec![User {
                id: "usr_1".into(),
                username: "Khach".into(),
                email: None,
                phone: None,
                contact_url: None,
                created_at: 1,
                balance: 50_000,
            }],
            ..Default::default()
        };
        write_data_file(&mirror, &expected).unwrap();

        let loaded = load_data_from_paths(&canonical, &mirror);
        assert_eq!(loaded.users[0].balance, 50_000);
        let canonical_data = read_data_file(&canonical).expect("canonical phải được tạo");
        assert_eq!(canonical_data.users[0].username, "Khach");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_luon_thang_va_dong_bo_lai_mirror() {
        let root = std::env::temp_dir().join(format!("subscription-manager-storage-sync-{}", std::process::id()));
        let canonical = root.join("config/data.json");
        let mirror = root.join("app/storage/data.json");
        let canonical_data = DataStore {
            users: vec![User {
                id: "usr_primary".into(),
                username: "Nguon chinh".into(),
                email: None,
                phone: None,
                contact_url: None,
                created_at: 2,
                balance: 10,
            }],
            ..Default::default()
        };
        let stale_mirror = DataStore::default();
        write_data_file(&canonical, &canonical_data).unwrap();
        write_data_file(&mirror, &stale_mirror).unwrap();

        let loaded = load_data_from_paths(&canonical, &mirror);
        assert_eq!(loaded.users[0].id, "usr_primary");
        let mirrored = read_data_file(&mirror).expect("mirror phải được đồng bộ lại");
        assert_eq!(mirrored.users[0].id, "usr_primary");
        let _ = fs::remove_dir_all(root);
    }

    /// Tương thích ngược: `data.json` của bản CŨ không có `balance`,
    /// `auto_renew`, `last_auto_renew_at`. Thiếu `serde(default)` ở các field
    /// mới sẽ làm parse thất bại → `load_data` trả rỗng → mất TOÀN BỘ dữ liệu
    /// khách hàng. Test này chốt: file cũ vẫn đọc được, field mới nhận giá trị
    /// mặc định an toàn (số dư 0, auto-renew TẮT).
    #[test]
    fn doc_duoc_data_json_cua_ban_cu() {
        let json_cu = r#"{
            "users": [
                {
                    "id": "usr_1",
                    "username": "Khach cu",
                    "email": null,
                    "phone": null,
                    "contact_url": null,
                    "created_at": 1700000000000
                }
            ],
            "packages": [
                {
                    "id": "pkg_1",
                    "name": "Goi thang",
                    "description": null,
                    "duration_days": 30,
                    "price": 200000
                }
            ],
            "subscriptions": [
                {
                    "id": "sub_1",
                    "user_id": "usr_1",
                    "package_id": "pkg_1",
                    "expiration_date": 1800000000000,
                    "is_active": true
                }
            ],
            "transactions": []
        }"#;

        let data: DataStore = serde_json::from_str(json_cu).expect("data.json bản cũ phải đọc được");

        assert_eq!(data.users.len(), 1, "không được mất user");
        assert_eq!(data.users[0].balance, 0, "số dư mặc định phải là 0");
        assert_eq!(data.subscriptions.len(), 1, "không được mất đăng ký");
        assert!(
            !data.subscriptions[0].auto_renew,
            "auto_renew mặc định phải TẮT — không tự trừ tiền khách khi họ chưa đồng ý"
        );
        assert_eq!(data.subscriptions[0].last_auto_renew_at, None);
        // Dữ liệu cũ vẫn nguyên vẹn.
        assert_eq!(data.subscriptions[0].expiration_date, 1_800_000_000_000);
        assert!(data.subscriptions[0].is_active);
    }
}
