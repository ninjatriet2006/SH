/*
[INTEGRITY NOTES]
- Mục đích: Phát hành và tra cứu MÃ THANH TOÁN dùng làm nội dung chuyển khoản.
- Trách nhiệm: Sinh mã ngắn, an toàn với sao kê ngân hàng; lưu lại mỗi lần in
  hóa đơn để truy vết; tra cứu ngược từ mã (kể cả khi khách gõ sai) về người
  chuyển và các giao dịch liên quan.
- Tương tác: `storage` (đọc/ghi `payment_refs`), `bridge/payment_bridge.ts`.

THIẾT KẾ MÃ — vì sao như vậy:

    SM  7K2M  X4B9
    ─┬  ─┬──  ─┬──
     │   │     └── 4 ký tự ngẫu nhiên: chống đoán, chống trùng
     │   └──────── 4 ký tự NHẬN DẠNG NGƯỜI CHUYỂN (user_token), dẫn xuất từ
     │             user_id nên CỐ ĐỊNH theo từng khách
     └──────────── tiền tố cố định, giúp nhận ra đây là mã của hệ thống

  - Chỉ dùng bộ chữ Crockford base32 (bỏ I, L, O, U) nên không lẫn 0/O, 1/I/L
    khi khách đọc từ hóa đơn giấy và gõ lại.
  - Dài 10 ký tự: nằm gọn trong giới hạn nội dung CK của mọi ngân hàng VN, và
    ngắn đủ để gõ tay không sai.
  - `user_token` là dấu hiệu đặc biệt gắn với người chuyển: nếu khách gõ lệch
    phần ngẫu nhiên, ta vẫn lọc ra được chủ giao dịch từ 4 ký tự này.
*/

use crate::models::PaymentRef;
use crate::storage::{load_data, save_data};
use crate::utils::current_timestamp;

/// Bộ ký tự Crockford base32: bỏ `I`, `L`, `O`, `U` để tránh đọc/gõ lẫn.
const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Tiền tố nhận dạng mã của hệ thống (SubManager).
const PREFIX: &str = "SM";

/// Độ dài phần nhận dạng người chuyển.
const TOKEN_LEN: usize = 4;

/// Độ dài phần ngẫu nhiên.
const RANDOM_LEN: usize = 4;

/// Chuyển một số thành chuỗi theo `ALPHABET`, độ dài cố định.
fn encode(mut value: u64, len: usize) -> String {
    let mut out = vec![b'0'; len];
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(value % ALPHABET.len() as u64) as usize];
        value /= ALPHABET.len() as u64;
    }
    String::from_utf8(out).expect("ALPHABET là ASCII")
}

/// Hash FNV-1a 64-bit — nhỏ, không phụ thuộc thư viện ngoài, đủ để phân tán
/// `user_id` thành token. KHÔNG dùng cho mục đích bảo mật.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Token nhận dạng người chuyển: CỐ ĐỊNH theo `user_id`.
///
/// Cùng một khách luôn cho ra cùng token, nên nhìn nội dung chuyển khoản là
/// biết ngay của ai — kể cả khi phần còn lại của mã bị gõ sai.
pub fn user_token(user_id: &str) -> String {
    encode(fnv1a(user_id.as_bytes()), TOKEN_LEN)
}

/// Phần ngẫu nhiên. Dùng thời gian + địa chỉ stack làm nguồn entropy để không
/// phải thêm dependency `rand` chỉ cho 4 ký tự.
fn random_part(salt: u64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let local = 0u8;
    let addr = &local as *const u8 as u64;
    encode(fnv1a(&(nanos ^ addr ^ salt).to_le_bytes()), RANDOM_LEN)
}

/// Chuẩn hoá chuỗi người dùng nhập để tra cứu: bỏ khoảng trắng/gạch, in hoa, và
/// sửa các ký tự thường bị đọc lẫn (O→0, I/L→1, U→V) theo quy ước Crockford.
///
/// Nhờ vậy "sm 7k2m-x4b9", "SM7K2MX4B9", "SM7K2MX4B9 " đều tra ra cùng một mã.
pub fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            'U' => 'V',
            other => other,
        })
        .collect()
}

/// Phát hành mã cho một nhóm giao dịch — hàm THUẦN, không chạm file.
///
/// Tách khỏi command để test được toàn bộ quy tắc nghiệp vụ (kiểm tra chủ sở
/// hữu, cộng tiền, chống trùng mã) mà không phá `data.json` thật.
fn issue_into(
    data: &mut crate::storage::DataStore,
    user_id: &str,
    transaction_ids: &[String],
    now: i64,
) -> Result<PaymentRef, String> {
    if transaction_ids.is_empty() {
        return Err("Cần ít nhất một giao dịch để phát hành mã".to_string());
    }

    let username = data
        .users
        .iter()
        .find(|u| u.id == user_id)
        .ok_or_else(|| format!("Không tìm thấy người dùng: {}", user_id))?
        .username
        .clone();

    // Mọi giao dịch phải tồn tại và thuộc đúng khách này — mã gắn với một người
    // chuyển, gộp giao dịch của người khác vào sẽ làm đối soát sai chủ.
    let mut amount: u64 = 0;
    for tx_id in transaction_ids {
        let tx = data
            .transactions
            .iter()
            .find(|t| &t.id == tx_id)
            .ok_or_else(|| format!("Không tìm thấy giao dịch: {}", tx_id))?;
        if tx.user_id != user_id {
            return Err(format!("Giao dịch {} không thuộc người dùng {}", tx_id, user_id));
        }
        amount = amount.saturating_add(tx.amount);
    }

    let token = user_token(user_id);

    // Sinh mã tới khi không trùng mã đã phát hành.
    let mut code = String::new();
    for attempt in 0..64 {
        let candidate = format!("{PREFIX}{token}{}", random_part(attempt));
        if !data.payment_refs.iter().any(|r| r.code == candidate) {
            code = candidate;
            break;
        }
    }
    if code.is_empty() {
        return Err("Không sinh được mã thanh toán duy nhất, thử lại".to_string());
    }

    let payment_ref = PaymentRef {
        code,
        user_token: token,
        user_id: user_id.to_string(),
        username,
        transaction_ids: transaction_ids.to_vec(),
        amount,
        created_at: now,
        settled_at: None,
    };
    data.payment_refs.push(payment_ref.clone());
    Ok(payment_ref)
}

/// Tra cứu mã — hàm THUẦN, không chạm file.
fn lookup_in(data: &crate::storage::DataStore, input: &str) -> PaymentLookup {
    let cleaned = normalize(input);

    if let Some(found) = data.payment_refs.iter().find(|r| r.code == cleaned) {
        return PaymentLookup {
            user_token: found.user_token.clone(),
            exact: Some(found.clone()),
            same_user: Vec::new(),
        };
    }

    // Không khớp: rút token ra khỏi vị trí cố định để lần theo người chuyển.
    let token_start = PREFIX.len();
    let token_end = token_start + TOKEN_LEN;
    let token = if cleaned.len() >= token_end && cleaned.starts_with(PREFIX) {
        cleaned[token_start..token_end].to_string()
    } else {
        String::new()
    };

    let same_user = if token.is_empty() {
        Vec::new()
    } else {
        let mut v: Vec<PaymentRef> = data
            .payment_refs
            .iter()
            .filter(|r| r.user_token == token)
            .cloned()
            .collect();
        // Mới nhất trước: khách thường chuyển cho hóa đơn vừa nhận.
        v.sort_by_key(|r| std::cmp::Reverse(r.created_at));
        v
    };

    PaymentLookup {
        exact: None,
        same_user,
        user_token: token,
    }
}

/// Phát hành mã mới cho một nhóm giao dịch của CÙNG một khách và LƯU LẠI.
///
/// Gọi mỗi lần in hóa đơn (đơn lẻ hoặc gộp) nên mọi nội dung chuyển khoản đã
/// đưa cho khách đều có bản ghi để đối soát về sau.
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn issue_payment_ref(user_id: String, transaction_ids: Vec<String>) -> Result<PaymentRef, String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();
    let payment_ref = issue_into(&mut data, &user_id, &transaction_ids, current_timestamp())?;
    save_data(&data)?;
    Ok(payment_ref)
}

/// Kết quả tra cứu một mã từ sao kê ngân hàng.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLookup {
    /// Khớp chính xác mã đã phát hành.
    pub exact: Option<PaymentRef>,
    /// Khi không khớp chính xác: các mã CÙNG `user_token` — tức cùng người
    /// chuyển. Đây là giá trị của việc nhúng token vào mã: khách gõ sai phần
    /// ngẫu nhiên vẫn truy ra được ai chuyển tiền.
    pub same_user: Vec<PaymentRef>,
    /// Token đọc được từ chuỗi đã nhập (rỗng nếu chuỗi quá ngắn/không hợp lệ).
    pub user_token: String,
}

/// Tra cứu nội dung chuyển khoản → người chuyển + giao dịch.
///
/// Chấp nhận chuỗi bẩn từ sao kê (có khoảng trắng, chữ thường, lẫn O/0) nhờ
/// `normalize`, và vẫn trả kết quả hữu ích khi mã bị gõ sai một phần.
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn lookup_payment_ref(input: String) -> Result<PaymentLookup, String> {
    let data = load_data();
    Ok(lookup_in(&data, &input))
}

/// Danh sách mã đã phát hành, mới nhất trước. `user_id` rỗng = lấy tất cả.
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn list_payment_refs(user_id: Option<String>) -> Result<Vec<PaymentRef>, String> {
    let data = load_data();
    let mut refs: Vec<PaymentRef> = match user_id.as_deref() {
        Some(uid) if !uid.is_empty() => data.payment_refs.into_iter().filter(|r| r.user_id == uid).collect(),
        _ => data.payment_refs,
    };
    refs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
    Ok(refs)
}

/// Đánh dấu đã nhận được tiền cho một mã (đối soát xong).
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn settle_payment_ref(code: String, settled: bool) -> Result<PaymentRef, String> {
    let _store_guard = crate::storage::lock_store();
    let mut data = load_data();
    let cleaned = normalize(&code);

    let entry = data
        .payment_refs
        .iter_mut()
        .find(|r| r.code == cleaned)
        .ok_or_else(|| format!("Không tìm thấy mã thanh toán: {}", cleaned))?;

    entry.settled_at = if settled { Some(current_timestamp()) } else { None };
    let updated = entry.clone();

    save_data(&data)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ma_chi_dung_ky_tu_khong_gay_nham_lan() {
        // O, I, L, U bị loại để không lẫn với 0, 1, V khi khách đọc hóa đơn giấy.
        let token = user_token("usr_1757000000000_0");
        for c in token.chars() {
            assert!(ALPHABET.contains(&(c as u8)), "ký tự '{c}' không thuộc bộ an toàn");
            assert!(!"ILOU".contains(c), "ký tự '{c}' dễ gây nhầm lẫn");
        }
    }

    #[test]
    fn token_co_dinh_theo_user() {
        // Cùng user → cùng token, đây là điều kiện để truy ngược người chuyển.
        let a = user_token("usr_abc");
        let b = user_token("usr_abc");
        assert_eq!(a, b);
        assert_eq!(a.len(), TOKEN_LEN);
        // Khác user → token khác (không đảm bảo tuyệt đối nhưng phải khác ở
        // các trường hợp thực tế này).
        assert_ne!(user_token("usr_abc"), user_token("usr_xyz"));
    }

    #[test]
    fn normalize_chiu_duoc_chuoi_ban_tu_sao_ke() {
        // Sao kê ngân hàng thường viết hoa/thường lẫn lộn, thêm khoảng trắng.
        assert_eq!(normalize("sm 7k2m-x4b9"), "SM7K2MX4B9");
        assert_eq!(normalize("  SM7K2MX4B9  "), "SM7K2MX4B9");
        // Ký tự dễ đọc lẫn được quy về dạng chuẩn.
        assert_eq!(normalize("SMOI1U"), "SM011V");
        // Bỏ ký tự lạ, không panic.
        assert_eq!(normalize("SM/7K2M*X4B9!"), "SM7K2MX4B9");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn ma_ngau_nhien_khac_nhau_giua_cac_lan() {
        // Nếu phần ngẫu nhiên trùng nhau, hai hóa đơn khác nhau sẽ có cùng nội
        // dung chuyển khoản → không đối soát được.
        let mut seen = std::collections::HashSet::new();
        for i in 0..32 {
            seen.insert(random_part(i));
        }
        assert!(
            seen.len() > 24,
            "phần ngẫu nhiên trùng quá nhiều: {} giá trị khác nhau trên 32 lần",
            seen.len()
        );
    }

    #[test]
    fn encode_do_dai_co_dinh() {
        assert_eq!(encode(0, 4).len(), 4);
        assert_eq!(encode(u64::MAX, 4).len(), 4);
        assert_eq!(encode(0, 4), "0000");
    }

    // ===== Test nghiệp vụ trên DataStore trong bộ nhớ (không chạm file) =====

    use crate::models::{Transaction, User};
    use crate::storage::DataStore;

    const NOW: i64 = 1_700_000_000_000;

    fn fixture() -> DataStore {
        DataStore {
            users: vec![
                User {
                    id: "usr_1".into(),
                    username: "Khach A".into(),
                    email: None,
                    phone: None,
                    contact_url: None,
                    created_at: NOW,
                    balance: 0,
                },
                User {
                    id: "usr_2".into(),
                    username: "Khach B".into(),
                    email: None,
                    phone: None,
                    contact_url: None,
                    created_at: NOW,
                    balance: 0,
                },
            ],
            packages: Vec::new(),
            subscriptions: Vec::new(),
            transactions: vec![
                Transaction {
                    id: "tx_1".into(),
                    user_id: "usr_1".into(),
                    package_id: "pkg_1".into(),
                    amount: 200_000,
                    action: "ASSIGN".into(),
                    created_at: NOW,
                },
                Transaction {
                    id: "tx_2".into(),
                    user_id: "usr_1".into(),
                    package_id: "pkg_1".into(),
                    amount: 300_000,
                    action: "RENEW".into(),
                    created_at: NOW,
                },
                Transaction {
                    id: "tx_3".into(),
                    user_id: "usr_2".into(),
                    package_id: "pkg_1".into(),
                    amount: 500_000,
                    action: "ASSIGN".into(),
                    created_at: NOW,
                },
            ],
            payment_refs: Vec::new(),
        }
    }

    #[test]
    fn ma_co_dinh_dang_prefix_token_random() {
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();

        assert!(r.code.starts_with(PREFIX), "mã: {}", r.code);
        assert_eq!(r.code.len(), PREFIX.len() + TOKEN_LEN + RANDOM_LEN);
        // Token phải NẰM TRONG mã — đây là dấu hiệu truy ngược người chuyển.
        assert!(r.code.contains(&r.user_token));
        assert_eq!(r.user_token, user_token("usr_1"));
        // Đủ ngắn cho nội dung chuyển khoản của mọi ngân hàng VN.
        assert!(r.code.len() <= 16, "mã dài quá: {}", r.code.len());
    }

    #[test]
    fn in_gop_cong_dung_tong_tien() {
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into(), "tx_2".into()], NOW).unwrap();
        assert_eq!(r.amount, 500_000);
        assert_eq!(r.transaction_ids.len(), 2);
    }

    #[test]
    fn moi_lan_in_deu_duoc_luu_lai() {
        // Yêu cầu: in 1 hoặc in gộp đều tự lưu để truy vết về sau.
        let mut data = fixture();
        issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();
        issue_into(&mut data, "usr_1", &["tx_2".into()], NOW + 1).unwrap();
        assert_eq!(data.payment_refs.len(), 2);
    }

    #[test]
    fn tu_choi_gop_giao_dich_cua_khach_khac() {
        // Gộp lẫn chủ sẽ làm đối soát quy sai người chuyển.
        let mut data = fixture();
        let err = issue_into(&mut data, "usr_1", &["tx_1".into(), "tx_3".into()], NOW).expect_err("phải từ chối");
        assert!(err.contains("không thuộc người dùng"), "lỗi: {err}");
        assert!(data.payment_refs.is_empty(), "không được lưu gì khi lỗi");
    }

    #[test]
    fn tu_choi_giao_dich_hoac_user_khong_ton_tai() {
        let mut data = fixture();
        assert!(issue_into(&mut data, "usr_1", &["tx_khong_co".into()], NOW).is_err());
        assert!(issue_into(&mut data, "usr_khong_co", &["tx_1".into()], NOW).is_err());
        assert!(issue_into(&mut data, "usr_1", &[], NOW).is_err());
        assert!(data.payment_refs.is_empty());
    }

    #[test]
    fn tra_cuu_khop_chinh_xac() {
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();

        let found = lookup_in(&data, &r.code);
        let exact = found.exact.expect("phải khớp chính xác");
        assert_eq!(exact.code, r.code);
        assert_eq!(exact.user_id, "usr_1");
        assert_eq!(exact.username, "Khach A");
        assert_eq!(exact.transaction_ids, vec!["tx_1".to_string()]);
    }

    #[test]
    fn tra_cuu_chiu_duoc_chuoi_ban_tu_sao_ke() {
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();

        // Sao kê hay viết thường, thêm khoảng trắng/gạch nối.
        let messy = format!("  {}-{} ", &r.code[..6].to_lowercase(), &r.code[6..]);
        let found = lookup_in(&data, &messy);
        assert!(found.exact.is_some(), "phải khớp dù chuỗi bẩn: {messy}");
    }

    /// Đây là điểm cốt lõi: khách gõ SAI phần ngẫu nhiên thì vẫn phải truy ra
    /// được ai đã chuyển tiền, nhờ `user_token` nhúng trong mã.
    #[test]
    fn go_sai_phan_ngau_nhien_van_truy_ra_nguoi_chuyen() {
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();
        // Mã của khách khác, để chắc chắn không trả về bừa.
        issue_into(&mut data, "usr_2", &["tx_3".into()], NOW).unwrap();

        // Giữ prefix + token, thay 4 ký tự cuối bằng chuỗi sai.
        let sai = format!("{}{}ZZZZ", PREFIX, r.user_token);
        let found = lookup_in(&data, &sai);

        assert!(found.exact.is_none(), "không được khớp chính xác");
        assert_eq!(found.user_token, r.user_token);
        assert_eq!(found.same_user.len(), 1, "phải tìm ra đúng 1 mã của khách này");
        assert_eq!(found.same_user[0].user_id, "usr_1");
        assert_eq!(found.same_user[0].username, "Khach A");
    }

    #[test]
    fn go_sai_ca_prefix_thi_khong_doan_bua() {
        let mut data = fixture();
        issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();

        let found = lookup_in(&data, "chuyen tien hoc phi");
        assert!(found.exact.is_none());
        assert!(
            found.same_user.is_empty(),
            "không có dấu hiệu nhận dạng thì không được đoán bừa chủ giao dịch"
        );
    }

    #[test]
    fn cung_khach_nhieu_ma_thi_tra_ve_moi_nhat_truoc() {
        let mut data = fixture();
        let cu = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();
        let moi = issue_into(&mut data, "usr_1", &["tx_2".into()], NOW + 10_000).unwrap();

        let sai = format!("{}{}ZZZZ", PREFIX, cu.user_token);
        let found = lookup_in(&data, &sai);

        assert_eq!(found.same_user.len(), 2);
        assert_eq!(
            found.same_user[0].code, moi.code,
            "mã mới nhất phải đứng đầu — khách thường chuyển cho hóa đơn vừa nhận"
        );
    }

    #[test]
    fn ma_khong_trung_nhau_giua_cac_lan_phat_hanh() {
        let mut data = fixture();
        let mut codes = std::collections::HashSet::new();
        for i in 0..20 {
            let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW + i).unwrap();
            assert!(codes.insert(r.code.clone()), "mã trùng: {}", r.code);
        }
        assert_eq!(data.payment_refs.len(), 20);
    }

    #[test]
    fn snapshot_ten_khach_khong_doi_khi_khach_doi_ten() {
        // Hóa đơn cũ phải đọc được tên tại thời điểm phát hành.
        let mut data = fixture();
        let r = issue_into(&mut data, "usr_1", &["tx_1".into()], NOW).unwrap();
        data.users[0].username = "Ten moi".into();

        let found = lookup_in(&data, &r.code).exact.unwrap();
        assert_eq!(found.username, "Khach A");
    }
}
