/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các nghiệp vụ gán và quản lý Subscription (Đăng ký gói) cho User.
- Trách nhiệm: Tính toán ngày hết hạn, tạo mới Subscription, cập nhật thời gian và kiểm tra trạng thái kích hoạt.
- Tương tác: Gọi `storage` để lấy và lưu dữ liệu. Cần đọc `packages` để tính toán `duration_days` nếu không truyền `custom_expiration_date`.
*/

// Import struct Subscription và Transaction từ module models
use crate::models::{tx_action, Subscription, Transaction};
// Import hàm đọc/ghi file lưu trữ
use crate::storage::{load_data, save_data};
use crate::utils::{current_timestamp, generate_id};

/// Một ngày tính bằng mili-giây. Trước đây hằng này được viết lại ở nhiều chỗ.
const MS_IN_DAY: i64 = 86_400_000;

/// Giới hạn số chu kỳ bù cho một lần rà auto-renew.
/// Nếu app không mở trong thời gian dài, gói có thể đã hết hạn qua nhiều chu kỳ;
/// bù dần từng chu kỳ là đúng về số tiền, nhưng phải có trần để một gói cấu hình
/// sai (duration_days rất nhỏ) không tạo ra hàng nghìn giao dịch trong một lần rà.
const MAX_AUTO_RENEW_CYCLES: u32 = 60;

// Quét toàn bộ subscription và đồng bộ `is_active` theo thời gian hiện tại.
// Trả về true nếu có thay đổi (để caller quyết định có ghi file không).
// Tách helper vì block này bị copy-paste ở 3 command (trước đây lệch nhau:
// hai chỗ nuốt lỗi save, một chỗ lại `?` cho thao tác chỉ đọc).
fn refresh_statuses(data: &mut crate::storage::DataStore, now: i64) -> bool {
    let mut changed = false;
    for sub in data.subscriptions.iter_mut() {
        let should_be_active = sub.expiration_date > now;
        if sub.is_active != should_be_active {
            sub.is_active = should_be_active;
            changed = true;
        }
    }
    changed
}

/// Kết quả một lần rà tự động gia hạn — trả về để UI báo cho người dùng biết
/// điều gì đã xảy ra sau lưng họ (tiền đã bị trừ, cờ auto-renew bị tắt), thay
/// vì thay đổi âm thầm.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AutoRenewReport {
    /// Số đăng ký đã được gia hạn thành công.
    pub renewed: u32,
    /// Tổng tiền đã trừ vào số dư (VNĐ).
    pub total_charged: u64,
    /// Các đăng ký bật auto-renew nhưng KHÔNG gia hạn được, kèm lý do và cờ
    /// cho biết auto-renew đã bị tắt hay chưa.
    pub skipped: Vec<AutoRenewSkip>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoRenewSkip {
    pub subscription_id: String,
    pub user_id: String,
    pub package_id: String,
    pub reason: String,
    /// `true` nếu cờ `auto_renew` đã bị TẮT do lần rà này thất bại.
    /// Không tự tắt khi nguyên nhân là trần chu kỳ (lỗi kỹ thuật tạm thời,
    /// không phải do khách thiếu tiền) — lần rà sau sẽ tiếp tục.
    pub auto_renew_disabled: bool,
}

/// Rà và thực hiện tự động gia hạn cho các đăng ký đã hết hạn có `auto_renew`.
///
/// Quy tắc (theo yêu cầu nghiệp vụ):
///   - CHỈ gia hạn khi `auto_renew = true` — người dùng phải tự tick, hệ thống
///     không bao giờ tự ý trừ tiền của khách.
///   - CHỈ gia hạn khi số dư ĐỦ trả trọn giá gói. Số dư âm (đang nợ) hoặc
///     không đủ thì bỏ qua, để gói hết hạn — KHÔNG cho nợ thêm, KHÔNG trừ một
///     phần.
///   - Thiếu tiền thì TẮT LUÔN cờ `auto_renew`: tránh rà lại vô ích mỗi lần mở
///     app, và buộc người dùng chủ động bật lại sau khi khách nạp thêm (không
///     âm thầm trừ tiền vào lúc họ không ngờ tới).
///   - Bù nhiều chu kỳ nếu app lâu không mở, mỗi chu kỳ trừ tiền và ghi một
///     giao dịch riêng để lịch sử khớp với số tiền thực trừ.
///
/// Trả về `(report, changed)`; `changed` cho caller biết có cần ghi file không.
fn run_auto_renew(data: &mut crate::storage::DataStore, now: i64) -> (AutoRenewReport, bool) {
    let mut report = AutoRenewReport::default();
    let mut changed = false;

    // Thu thập trước danh sách cần xử lý để không giữ borrow khi sửa users.
    let candidates: Vec<(usize, String, String)> = data
        .subscriptions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.auto_renew && s.expiration_date <= now)
        .map(|(i, s)| (i, s.user_id.clone(), s.package_id.clone()))
        .collect();

    for (idx, user_id, package_id) in candidates {
        // Gói có thể đã bị xoá sau khi đăng ký được tạo.
        let Some(package) = data.packages.iter().find(|p| p.id == package_id).cloned() else {
            // Gói không còn thì auto-renew vĩnh viễn không chạy được → tắt cờ
            // để không rà lại vô ích mỗi lần mở app.
            data.subscriptions[idx].auto_renew = false;
            changed = true;
            report.skipped.push(AutoRenewSkip {
                subscription_id: data.subscriptions[idx].id.clone(),
                user_id,
                package_id,
                reason: "Gói dịch vụ không còn tồn tại".to_string(),
                auto_renew_disabled: true,
            });
            continue;
        };

        // Gói có duration 0 sẽ không bao giờ vượt `now` → vòng lặp vô hạn.
        if package.duration_days == 0 {
            data.subscriptions[idx].auto_renew = false;
            changed = true;
            report.skipped.push(AutoRenewSkip {
                subscription_id: data.subscriptions[idx].id.clone(),
                user_id,
                package_id,
                reason: "Gói có thời hạn 0 ngày, không thể tự động gia hạn".to_string(),
                auto_renew_disabled: true,
            });
            continue;
        }

        let Some(user_idx) = data.users.iter().position(|u| u.id == user_id) else {
            data.subscriptions[idx].auto_renew = false;
            changed = true;
            report.skipped.push(AutoRenewSkip {
                subscription_id: data.subscriptions[idx].id.clone(),
                user_id,
                package_id,
                reason: "Người dùng không còn tồn tại".to_string(),
                auto_renew_disabled: true,
            });
            continue;
        };

        let price = package.price;
        let cycle_ms = package.duration_days as i64 * MS_IN_DAY;
        let mut cycles_done = 0u32;
        let mut new_txs: Vec<Transaction> = Vec::new();

        // Bù từng chu kỳ cho tới khi hạn vượt `now` hoặc hết tiền.
        while data.subscriptions[idx].expiration_date <= now && cycles_done < MAX_AUTO_RENEW_CYCLES
        {
            let balance = data.users[user_idx].balance;
            // Số dư âm (đang nợ) hoặc không đủ → dừng, KHÔNG cho nợ thêm.
            // So sánh trên i64 nên nợ tự động fail phép kiểm tra này.
            if balance < price as i64 {
                break;
            }

            data.users[user_idx].balance = balance - price as i64;
            data.subscriptions[idx].expiration_date += cycle_ms;
            data.subscriptions[idx].last_auto_renew_at = Some(now);

            new_txs.push(Transaction {
                id: generate_id("tx"),
                user_id: user_id.clone(),
                package_id: package_id.clone(),
                amount: price,
                action: tx_action::AUTO_RENEW.to_string(),
                created_at: now,
            });

            cycles_done += 1;
            report.total_charged = report.total_charged.saturating_add(price);
        }

        if cycles_done > 0 {
            data.subscriptions[idx].is_active = data.subscriptions[idx].expiration_date > now;
            data.transactions.extend(new_txs);
            report.renewed += 1;
            changed = true;
        }

        // Vẫn còn hết hạn sau khi thử → không đủ tiền (hoặc chạm trần chu kỳ).
        if data.subscriptions[idx].expiration_date <= now {
            let cham_tran = cycles_done >= MAX_AUTO_RENEW_CYCLES;
            let reason = if cham_tran {
                format!("Đã gia hạn tối đa {MAX_AUTO_RENEW_CYCLES} chu kỳ trong một lần rà")
            } else {
                format!(
                    "Số dư không đủ (cần {} VNĐ, còn {} VNĐ) — đã tắt tự động gia hạn",
                    price, data.users[user_idx].balance
                )
            };

            // TẮT cờ auto_renew khi thiếu tiền: không thử lại mỗi lần mở app,
            // và người dùng phải chủ động bật lại sau khi khách nạp thêm. Chạm
            // trần chu kỳ thì GIỮ cờ — đó là giới hạn kỹ thuật của một lần rà,
            // lần sau vẫn nên tiếp tục bù.
            if !cham_tran {
                data.subscriptions[idx].auto_renew = false;
                changed = true;
            }

            report.skipped.push(AutoRenewSkip {
                subscription_id: data.subscriptions[idx].id.clone(),
                user_id,
                package_id,
                reason,
                auto_renew_disabled: !cham_tran,
            });
        }
    }

    (report, changed)
}

// Lệnh Tauri để gán một gói dịch vụ cho một người dùng
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn add_subscription_to_user(
    user_id: String,
    package_id: String,
    custom_expiration_date: Option<i64>,
    amount: Option<u64>,
    auto_renew: Option<bool>,
) -> Result<Subscription, String> {
    let _store_guard = crate::storage::lock_store();
    // Tải toàn bộ cơ sở dữ liệu
    let mut data = load_data();
    
    // Kiểm tra xem user có tồn tại không
    if !data.users.iter().any(|u| u.id == *user_id) {
        return Err(format!("Người dùng không tồn tại: {}", user_id));
    }
    
    // Tìm gói dịch vụ theo ID
    let package = match data.packages.iter().find(|p| p.id == *package_id) {
        Some(pkg) => pkg.clone(), // Copy dữ liệu gói
        None => return Err(format!("Gói dịch vụ không tồn tại: {}", package_id)),
    };

    let auto_renew = auto_renew.unwrap_or(false);
    // Gói 0 ngày không thể tự gia hạn (hạn mới không bao giờ vượt hiện tại).
    // Chặn ngay lúc gán để người dùng biết, thay vì im lặng bỏ qua khi rà.
    if auto_renew && package.duration_days == 0 {
        return Err("Không thể bật tự động gia hạn cho gói có thời hạn 0 ngày".to_string());
    }
    
    // Tính toán ngày hết hạn (expiration_date), đơn vị mili-giây epoch.
    let now = current_timestamp();
    let expiration_date = match custom_expiration_date {
        // Ngày tùy chỉnh phải trong tương lai — trước đây nhận cả 0/âm/quá khứ
        // rồi vẫn ghi log ASSIGN, tạo subscription chết ngay từ đầu.
        Some(custom_date) if custom_date <= now => {
            return Err("Ngày hết hạn tùy chỉnh phải trong tương lai".to_string());
        }
        Some(custom_date) => custom_date,
        // Nếu không có, tự động tính bằng cách cộng số ngày của gói vào thời gian hiện tại
        None => now + (package.duration_days as i64 * MS_IN_DAY),
    };

    // Tính trạng thái is_active
    let is_active = expiration_date > now;
    
    // Khởi tạo đối tượng Subscription
    let new_subscription = Subscription {
        id: generate_id("sub"), // ID ngẫu nhiên tự tạo
        user_id: user_id.clone(),               // ID người dùng
        package_id: package_id.clone(),            // ID gói dịch vụ
        expiration_date,       // Ngày hết hạn đã tính toán
        is_active,             // Bật/tắt tùy theo thời hạn
        auto_renew,            // Tự động gia hạn (mặc định tắt)
        last_auto_renew_at: None,
    };
    
    // Thêm vào danh sách subscriptions
    data.subscriptions.push(new_subscription.clone());

    // Chỉ ghi log giao dịch khi có thu tiền thật (amount = Some).
    // Trước đây `unwrap_or(0)` biến "không thu" thành log ASSIGN 0đ giả.
    if let Some(paid) = amount {
        let new_tx = Transaction {
            id: generate_id("tx"),
            user_id: user_id.clone(),
            package_id: package_id.clone(),
            amount: paid,
            action: tx_action::ASSIGN.to_string(),
            created_at: current_timestamp(),
        };
        data.transactions.push(new_tx);
    }
    
    // Ghi dữ liệu
    save_data(&data)?;
    
    // Trả về gói đăng ký vừa tạo
    Ok(new_subscription)
}

// Lệnh Tauri để thay đổi ngày hết hạn của một subscription
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn update_subscription_expiry(
    subscription_id: String,
    new_expiration_date: i64,
    amount: Option<u64>,
    auto_renew: Option<bool>,
) -> Result<Subscription, String> {
    let _store_guard = crate::storage::lock_store();
    // Tải dữ liệu
    let mut data = load_data();

    // Kiểm tra điều kiện bật auto-renew TRƯỚC khi sửa (cần đọc packages, không
    // thể làm khi đang giữ mutable borrow vào subscriptions).
    if auto_renew == Some(true) {
        let pkg_id = data
            .subscriptions
            .iter()
            .find(|s| s.id == subscription_id)
            .map(|s| s.package_id.clone());
        if let Some(pkg_id) = pkg_id {
            match data.packages.iter().find(|p| p.id == pkg_id) {
                Some(p) if p.duration_days == 0 => {
                    return Err(
                        "Không thể bật tự động gia hạn cho gói có thời hạn 0 ngày".to_string()
                    );
                }
                None => {
                    return Err(
                        "Không thể bật tự động gia hạn: gói dịch vụ không còn tồn tại".to_string()
                    );
                }
                _ => {}
            }
        }
    }

    // Tìm kiếm đăng ký theo ID
    if let Some(sub) = data.subscriptions.iter_mut().find(|s| s.id == subscription_id) {
        // Đặt lại ngày hết hạn
        sub.expiration_date = new_expiration_date;
        
        // Nếu hạn sử dụng mới lớn hơn thời điểm hiện tại, có thể kích hoạt lại gói
        if new_expiration_date > current_timestamp() {
            sub.is_active = true;
        } else {
            // Ngược lại, nếu set ngày quá khứ, thì vô hiệu hóa
            sub.is_active = false;
        }

        // `None` = giữ nguyên lựa chọn hiện tại (quy ước clear-field như email).
        if let Some(flag) = auto_renew {
            sub.auto_renew = flag;
        }
        
        // Ghi log giao dịch gia hạn — chỉ khi có thu tiền thật.
        if let Some(paid) = amount {
            let new_tx = Transaction {
                id: generate_id("tx"),
                user_id: sub.user_id.clone(),
                package_id: sub.package_id.clone(),
                amount: paid,
                action: tx_action::RENEW.to_string(),
                created_at: current_timestamp(),
            };
            data.transactions.push(new_tx);
        }
        
        // Tạo bản copy để trả về
        let updated_sub = sub.clone();
        
        // Lưu dữ liệu
        save_data(&data)?;
        
        // Trả về thành công
        return Ok(updated_sub);
    }
    
    // Báo lỗi nếu không tìm thấy ID
    Err(format!("Không tìm thấy gói đăng ký: {}", subscription_id))
}

// Lệnh Tauri để gỡ bỏ / xóa một gói đăng ký khỏi người dùng
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn remove_subscription_from_user(subscription_id: String) -> Result<(), String> {
    let _store_guard = crate::storage::lock_store();
    // Lấy dữ liệu hiện tại
    let mut data = load_data();
    // Lọc mảng, bỏ qua ID cần xóa
    let initial_len = data.subscriptions.len();
    data.subscriptions.retain(|s| s.id != subscription_id);
    
    // Kiểm tra số lượng
    if data.subscriptions.len() == initial_len {
        return Err(format!("Không tìm thấy gói đăng ký để xóa: {}", subscription_id));
    }
    
    // Lưu lại
    save_data(&data)?;
    // Thành công
    Ok(())
}

// Lệnh Tauri để liệt kê các đăng ký của một người dùng cụ thể
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_user_subscriptions(user_id: String) -> Result<Vec<Subscription>, String> {
    let _store_guard = crate::storage::lock_store();
    // Lấy dữ liệu
    let mut data = load_data();
    let now = current_timestamp();

    // Rà tự động gia hạn trước khi đồng bộ trạng thái: gói vừa được gia hạn
    // phải hiện ACTIVE ngay trong danh sách trả về, không đợi lần nạp sau.
    let (_, renewed) = run_auto_renew(&mut data, now);
    let refreshed = refresh_statuses(&mut data, now);
    if renewed || refreshed {
        let _ = save_data(&data);
    }

    // Lọc ra các subscription mà trường user_id trùng khớp
    let user_subs: Vec<Subscription> = data.subscriptions.into_iter().filter(|s| s.user_id == user_id).collect();
    
    // Trả về danh sách
    Ok(user_subs)
}

// Lệnh Tauri để kiểm tra xem một đăng ký còn hạn hay không
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn check_subscription_status(subscription_id: String) -> Result<bool, String> {
    // Tải dữ liệu
    let mut data = load_data();
    
    // Tìm kiếm đăng ký
    if let Some(sub) = data.subscriptions.iter_mut().find(|s| s.id == subscription_id) {
        let now = current_timestamp();
        // Gán trực tiếp thay cho if/else bool (clippy::needless_bool_assign).
        sub.is_active = sub.expiration_date > now;
        
        // Trạng thái (để trả về hàm)
        let active_status = sub.is_active;
        
        // Lưu lại trạng thái kích hoạt nếu có thay đổi
        save_data(&data)?;
        
        // Trả về boolean true/false
        return Ok(active_status);
    }
    
    Err(format!("Không tìm thấy đăng ký: {}", subscription_id))
}

// Lệnh Tauri để liệt kê toàn bộ gói đăng ký (hỗ trợ hiển thị cảnh báo cho tất cả người dùng)
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_all_subscriptions() -> Result<Vec<Subscription>, String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();
    let now = current_timestamp();

    // Rà auto-renew rồi mới đồng bộ trạng thái (xem `list_user_subscriptions`).
    let (_, renewed) = run_auto_renew(&mut data, now);
    let refreshed = refresh_statuses(&mut data, now);
    if renewed || refreshed {
        let _ = save_data(&data);
    }

    Ok(data.subscriptions)
}

/// Chạy rà tự động gia hạn và TRẢ VỀ báo cáo. `list_*` cũng rà nhưng bỏ báo
/// cáo đi; lệnh này để UI hiển thị "đã gia hạn N gói, trừ X VNĐ" và danh sách
/// bị bỏ qua vì thiếu số dư — tiền của khách bị trừ thì người dùng phải thấy.
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn process_auto_renewals() -> Result<AutoRenewReport, String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();
    let now = current_timestamp();

    let (report, renewed) = run_auto_renew(&mut data, now);
    let refreshed = refresh_statuses(&mut data, now);
    if renewed || refreshed {
        // Ở đây dùng `?`: người gọi chủ động yêu cầu thay đổi dữ liệu nên lỗi
        // ghi file phải báo ra, không nuốt như trong các lệnh chỉ đọc.
        save_data(&data)?;
    }

    Ok(report)
}

/// Bật/tắt tự động gia hạn cho một đăng ký cụ thể (dùng cho checkbox trong UI).
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn set_subscription_auto_renew(
    subscription_id: String,
    auto_renew: bool,
) -> Result<Subscription, String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();

    // Kiểm tra gói trước khi mượn mutable vào subscriptions.
    let pkg_id = data
        .subscriptions
        .iter()
        .find(|s| s.id == subscription_id)
        .map(|s| s.package_id.clone())
        .ok_or_else(|| format!("Không tìm thấy gói đăng ký: {}", subscription_id))?;

    if auto_renew {
        match data.packages.iter().find(|p| p.id == pkg_id) {
            Some(p) if p.duration_days == 0 => {
                return Err("Không thể bật tự động gia hạn cho gói có thời hạn 0 ngày".to_string());
            }
            None => {
                return Err(
                    "Không thể bật tự động gia hạn: gói dịch vụ không còn tồn tại".to_string()
                );
            }
            _ => {}
        }
    }

    let sub = data
        .subscriptions
        .iter_mut()
        .find(|s| s.id == subscription_id)
        .expect("đã kiểm tra tồn tại ở trên");
    sub.auto_renew = auto_renew;
    let updated = sub.clone();

    save_data(&data)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Package, User};
    use crate::storage::DataStore;

    const NOW: i64 = 1_700_000_000_000;

    /// Dựng DataStore trong bộ nhớ: 1 user, 1 gói 30 ngày, 1 đăng ký ĐÃ hết hạn.
    /// `run_auto_renew` là hàm thuần trên `&mut DataStore` nên test không chạm
    /// vào file thật — tránh phá data.json của người dùng.
    fn fixture(balance: i64, auto_renew: bool, price: u64, duration_days: u32) -> DataStore {
        DataStore {
            users: vec![User {
                id: "usr_1".into(),
                username: "Khach".into(),
                email: None,
                phone: None,
                contact_url: None,
                created_at: NOW,
                balance,
            }],
            packages: vec![Package {
                id: "pkg_1".into(),
                name: "Goi thang".into(),
                description: None,
                duration_days,
                price,
            }],
            subscriptions: vec![Subscription {
                id: "sub_1".into(),
                user_id: "usr_1".into(),
                package_id: "pkg_1".into(),
                // Hết hạn 1 giờ trước.
                expiration_date: NOW - 3_600_000,
                is_active: false,
                auto_renew,
                last_auto_renew_at: None,
            }],
            transactions: Vec::new(),
            payment_refs: Vec::new(),
        }
    }

    #[test]
    fn du_so_du_thi_gia_han_va_tru_tien() {
        let mut data = fixture(500_000, true, 200_000, 30);
        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(changed);
        assert_eq!(report.renewed, 1);
        assert_eq!(report.total_charged, 200_000);
        assert_eq!(data.users[0].balance, 300_000, "phải trừ đúng giá gói");
        assert!(data.subscriptions[0].is_active, "gói phải hoạt động lại");
        assert!(data.subscriptions[0].expiration_date > NOW);
        assert_eq!(data.subscriptions[0].last_auto_renew_at, Some(NOW));
        // Lịch sử phải khớp số tiền đã trừ.
        assert_eq!(data.transactions.len(), 1);
        assert_eq!(data.transactions[0].action, tx_action::AUTO_RENEW);
        assert_eq!(data.transactions[0].amount, 200_000);
    }

    #[test]
    fn khong_bat_auto_renew_thi_khong_tru_tien() {
        // Yêu cầu nghiệp vụ: hệ thống KHÔNG tự ý trừ tiền khi khách chưa tick.
        let mut data = fixture(500_000, false, 200_000, 30);
        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(!changed);
        assert_eq!(report.renewed, 0);
        assert_eq!(data.users[0].balance, 500_000, "số dư phải nguyên vẹn");
        assert!(data.transactions.is_empty());
        assert!(report.skipped.is_empty(), "không tick thì không báo bỏ qua");
    }

    #[test]
    fn thieu_so_du_thi_bo_qua_va_tat_auto_renew() {
        let mut data = fixture(150_000, true, 200_000, 30);
        let exp_cu = data.subscriptions[0].expiration_date;
        let (report, changed) = run_auto_renew(&mut data, NOW);

        // `changed = true` vì cờ auto_renew bị TẮT (không phải vì gia hạn).
        assert!(changed, "phải báo có thay đổi để caller ghi file");
        assert_eq!(report.renewed, 0);
        assert_eq!(data.users[0].balance, 150_000, "không được trừ một phần");
        assert_eq!(data.subscriptions[0].expiration_date, exp_cu);
        assert!(data.transactions.is_empty());

        // Yêu cầu nghiệp vụ: thiếu tiền thì TẮT tự động gia hạn, buộc người
        // dùng bật lại sau khi khách nạp thêm.
        assert!(
            !data.subscriptions[0].auto_renew,
            "phải tắt auto_renew khi số dư không đủ"
        );
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].auto_renew_disabled);
        assert!(
            report.skipped[0].reason.contains("Số dư không đủ"),
            "lý do: {}",
            report.skipped[0].reason
        );
    }

    #[test]
    fn dang_no_thi_khong_gia_han_va_tat_auto_renew() {
        // Số dư âm = công nợ. Không được cho nợ thêm.
        let mut data = fixture(-50_000, true, 200_000, 30);
        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(changed, "cờ auto_renew bị tắt nên phải ghi file");
        assert_eq!(data.users[0].balance, -50_000, "nợ không được tăng");
        assert_eq!(report.renewed, 0);
        assert!(!data.subscriptions[0].auto_renew, "phải tắt auto_renew");
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].auto_renew_disabled);
    }

    /// Rà lần hai KHÔNG được trừ tiền nữa: cờ đã tắt ở lần một. Đây là mục đích
    /// chính của việc tự tắt — tránh âm thầm trừ tiền khi khách vừa nạp đủ.
    #[test]
    fn sau_khi_tat_thi_lan_ra_sau_khong_tru_tien() {
        let mut data = fixture(150_000, true, 200_000, 30);
        run_auto_renew(&mut data, NOW);
        assert!(!data.subscriptions[0].auto_renew);

        // Khách nạp thêm cho đủ tiền.
        data.users[0].balance = 500_000;
        let (report2, changed2) = run_auto_renew(&mut data, NOW);

        assert!(!changed2, "không được làm gì khi cờ đã tắt");
        assert_eq!(report2.renewed, 0);
        assert_eq!(data.users[0].balance, 500_000, "tiền phải còn nguyên");
        assert!(data.transactions.is_empty());
        assert!(
            report2.skipped.is_empty(),
            "cờ đã tắt thì không còn là ứng viên, không báo bỏ qua nữa"
        );
    }

    #[test]
    fn so_du_bang_dung_gia_thi_van_gia_han() {
        // Biên: balance == price phải đủ (dùng `<` chứ không phải `<=`).
        let mut data = fixture(200_000, true, 200_000, 30);
        let (report, _) = run_auto_renew(&mut data, NOW);
        assert_eq!(report.renewed, 1);
        assert_eq!(data.users[0].balance, 0);
    }

    #[test]
    fn gia_goi_0_thi_gia_han_khong_ton_tien() {
        // Gói miễn phí: vẫn gia hạn được, không trừ gì.
        let mut data = fixture(0, true, 0, 30);
        let (report, changed) = run_auto_renew(&mut data, NOW);
        assert!(changed);
        assert_eq!(report.renewed, 1);
        assert_eq!(report.total_charged, 0);
        assert_eq!(data.users[0].balance, 0);
    }

    #[test]
    fn bu_nhieu_chu_ky_khi_app_lau_khong_mo() {
        // Hết hạn 95 ngày trước, gói 30 ngày → cần 4 chu kỳ để vượt `now`.
        let mut data = fixture(1_000_000, true, 100_000, 30);
        data.subscriptions[0].expiration_date = NOW - 95 * MS_IN_DAY;

        let (report, _) = run_auto_renew(&mut data, NOW);

        assert_eq!(report.renewed, 1, "đếm theo số đăng ký, không phải chu kỳ");
        assert_eq!(report.total_charged, 400_000, "4 chu kỳ x 100k");
        assert_eq!(data.users[0].balance, 600_000);
        assert_eq!(data.transactions.len(), 4, "mỗi chu kỳ một giao dịch");
        assert!(data.subscriptions[0].expiration_date > NOW);
    }

    #[test]
    fn het_tien_giua_chuoi_chu_ky_thi_dung_lai() {
        // Cần 4 chu kỳ nhưng chỉ đủ tiền cho 2.
        let mut data = fixture(250_000, true, 100_000, 30);
        data.subscriptions[0].expiration_date = NOW - 95 * MS_IN_DAY;

        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(changed);
        assert_eq!(report.total_charged, 200_000);
        assert_eq!(data.users[0].balance, 50_000, "không trừ quá số dư");
        assert_eq!(data.transactions.len(), 2);
        // Vẫn hết hạn → phải báo để người dùng biết mà nạp thêm.
        assert!(data.subscriptions[0].expiration_date <= NOW);
        assert!(!data.subscriptions[0].is_active);
        assert_eq!(report.skipped.len(), 1);
        // Gia hạn được một phần nhưng cuối cùng vẫn thiếu tiền → tắt cờ.
        assert!(!data.subscriptions[0].auto_renew);
        assert!(report.skipped[0].auto_renew_disabled);
    }

    #[test]
    fn goi_con_han_thi_khong_bi_gia_han_som() {
        let mut data = fixture(500_000, true, 200_000, 30);
        data.subscriptions[0].expiration_date = NOW + 10 * MS_IN_DAY;
        data.subscriptions[0].is_active = true;

        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(!changed);
        assert_eq!(report.renewed, 0);
        assert_eq!(data.users[0].balance, 500_000);
    }

    #[test]
    fn goi_0_ngay_khong_gay_lap_vo_han() {
        // duration_days = 0 → hạn mới không bao giờ vượt `now`. Phải bỏ qua
        // thay vì lặp vô hạn (đã chặn cả ở lúc bật auto_renew).
        let mut data = fixture(1_000_000, true, 0, 0);
        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(changed, "cờ auto_renew bị tắt nên phải ghi file");
        assert_eq!(report.renewed, 0);
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].reason.contains("0 ngày"));
        assert!(data.transactions.is_empty());
        // Vĩnh viễn không chạy được → tắt cờ để không rà lại vô ích.
        assert!(!data.subscriptions[0].auto_renew);
        assert!(report.skipped[0].auto_renew_disabled);
    }

    #[test]
    fn goi_bi_xoa_thi_bao_ly_do_khong_panic() {
        let mut data = fixture(500_000, true, 200_000, 30);
        data.packages.clear();
        let (report, changed) = run_auto_renew(&mut data, NOW);

        assert!(changed, "cờ auto_renew bị tắt nên phải ghi file");
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].reason.contains("không còn tồn tại"));
        assert!(!data.subscriptions[0].auto_renew);
        assert!(report.skipped[0].auto_renew_disabled);
    }

    #[test]
    fn tran_so_chu_ky_giu_nguyen_auto_renew() {
        // Gói 1 ngày, hết hạn 10 năm trước, dư tiền vô kể: phải dừng ở trần
        // thay vì tạo hàng nghìn giao dịch trong một lần rà.
        let mut data = fixture(i64::MAX / 2, true, 1, 1);
        data.subscriptions[0].expiration_date = NOW - 3650 * MS_IN_DAY;

        let (report, _) = run_auto_renew(&mut data, NOW);

        assert_eq!(data.transactions.len(), MAX_AUTO_RENEW_CYCLES as usize);
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].reason.contains("tối đa"));
        // Trần chu kỳ là giới hạn KỸ THUẬT của một lần rà, không phải khách
        // thiếu tiền → GIỮ cờ để lần mở app sau tiếp tục bù.
        assert!(
            data.subscriptions[0].auto_renew,
            "không được tắt auto_renew khi chỉ chạm trần chu kỳ"
        );
        assert!(!report.skipped[0].auto_renew_disabled);
    }
}
