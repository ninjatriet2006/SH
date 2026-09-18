use std::sync::Arc;
use crate::proxy::AppState;

/// Xử lý xoay vòng phiên (Key Filtering + Unified Session Rotation) khi gặp mã lỗi 401 hoặc 403.
///
/// Chống đốt key oan (M2): key từ file chỉ bị loại khi số lỗi LIÊN TIẾP chạm ngưỡng
/// `max_key_failures` (chỉnh trong Settings, mặc định 3) — lỗi thoáng qua như WAF chặn
/// hay rate-limit 403 chỉ khiến advance sang key khác, không xóa. Response 2xx reset
/// bộ đếm của key đó qua `handle_success_reset`.
///
/// Phân biệt nguồn key (M1): route dùng `custom_auth_token` (không qua file) thì bỏ qua
/// toàn bộ thao tác file (xóa/advance) — token lỗi vẫn xoay tunnel/fingerprint như cũ.
pub fn handle_error_session_rotation(
    state: &Arc<AppState>,
    route_id: &str,
    status_code: u16,
    resolved_key: Option<&str>,
    key_from_file: bool,
) {
    if status_code != 401 && status_code != 403 {
        return;
    }

    let mut key_was_burned = false;
    if let Some(key_used) = resolved_key {
        if key_from_file {
            let mut conf = state.config.write();
            // Ngưỡng >= 1: threshold <= 1 giữ hành vi cũ (lỗi đầu tiên đốt luôn).
            let threshold = conf.max_key_failures.max(1);
            if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
                let failures = r.key_manager.record_key_failure(key_used);
                if failures >= threshold {
                    if status_code == 401 {
                        if let Ok(burned) = r.key_manager.remove_key_from_main_file(key_used) {
                            key_was_burned = burned;
                        }
                    } else if status_code == 403 {
                        if let Ok(burned) = r.key_manager.move_key_to_failed_file(key_used) {
                            key_was_burned = burned;
                        }
                    }
                }
                // Chỉ ghi đĩa khi thực sự đốt key: bộ đếm lỗi là transient
                // (serde skip) nên save mỗi 401 dưới ngưỡng là ghi file y hệt.
                if key_was_burned {
                    let _ = conf.save_to_disk();
                }
            }

            // Báo cho Frontend cập nhật UI Real-time nếu danh sách Key có biến động
            if key_was_burned {
                if let Some(ref handle) = *state.app_handle.read() {
                    use tauri::Emitter;
                    let _ = handle.emit("route-keys-updated", serde_json::json!({ "route_id": route_id }));
                }
            }
        }
    }

    // Unified Session Rotation: Xoay Key kế tiếp, chuyển tunnel khác và đổi fingerprint profile.
    // Chỉ ghi đĩa khi có đột biến thật (tránh write storm khi upstream 401/403 liên tục
    // mà pool rỗng / 1 tunnel / pool fingerprint rỗng — khi đó mọi nhánh đều no-op).
    // Per-endpoint fingerprint: route có fingerprint_index → advance index RIÊNG của nó;
    // route None (hoặc không tìm thấy route) → advance global như cũ.
    {
        let mut conf = state.config.write();
        let enabled_tunnel_ids: Vec<String> = conf.tunnels.iter().filter(|t| t.enabled).map(|t| t.id.clone()).collect();
        let pool_len = conf.fingerprint_pool.len();
        let mut changed = false;
        // Route None → vẫn xoay global (giữ hành vi cũ); Some → chỉ xoay per-route.
        let mut rotate_global = true;

        // 1. Advance route key (chỉ khi key từ file):
        // TUYỆT ĐỐI KHÔNG gọi advance_to_next_key() nếu key đã bị đốt (key kế tiếp đã tự động trượt vào slot hiện tại)
        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
            if key_from_file && !key_was_burned {
                changed |= r.key_manager.advance_to_next_key().is_some();
            }
            // Chuyển sang tunnel enabled kế tiếp nếu có nhiều hơn 1 tunnel đang chạy
            if enabled_tunnel_ids.len() > 1 {
                if let Some(curr_idx) = enabled_tunnel_ids.iter().position(|id| id == &r.tunnel_id) {
                    let next_idx = (curr_idx + 1) % enabled_tunnel_ids.len();
                    if r.tunnel_id != enabled_tunnel_ids[next_idx] {
                        r.tunnel_id = enabled_tunnel_ids[next_idx].clone();
                        changed = true;
                    }
                }
            }
            // 2. Rotate fingerprint per-route (pool_len đọc trước để tránh E0502:
            // r giữ &mut vào conf.routes nên không được borrow conf.fingerprint_pool ở đây).
            if let Some(idx) = r.fingerprint_index {
                rotate_global = false;
                if pool_len > 0 {
                    // pool 1 phần tử: next == idx là no-op thật → không đánh changed
                    // để tránh ghi đĩa mỗi 401/403.
                    let next = (idx + 1) % pool_len;
                    if next != idx {
                        r.fingerprint_index = Some(next);
                        changed = true;
                    }
                }
            }
        }
        // 3. Rotate global fingerprint (chỉ khi route dùng global default)
        if rotate_global && pool_len > 0 {
            // pool 1 phần tử là no-op → bỏ qua để khỏi ghi đĩa mỗi 401/403
            if pool_len > 1 {
                conf.active_fingerprint_index = (conf.active_fingerprint_index + 1) % pool_len;
                changed = true;
            }
        }
        if changed {
            let _ = conf.save_to_disk();
        }
    }
}

/// Response 2xx: key vừa chứng minh còn sống → xóa bộ đếm lỗi liên tiếp của nó.
/// Không ghi đĩa (counter là transient) để tránh write storm trên request thành công.
/// Dùng upgradable-read: đa số request không có counter → chỉ giữ read-lock chia sẻ,
/// không serialize toàn bộ gateway bằng write-lock độc quyền mỗi request 2xx.
pub fn handle_success_reset(
    state: &Arc<AppState>,
    route_id: &str,
    status_code: u16,
    resolved_key: Option<&str>,
    key_from_file: bool,
) {
    if !(200..300).contains(&status_code) || !key_from_file {
        return;
    }
    if let Some(key_used) = resolved_key {
        let conf = state.config.upgradable_read();
        let has_counter = conf
            .routes
            .iter()
            .find(|r| r.id == route_id)
            .map(|r| r.key_manager.consecutive_failures.contains_key(key_used))
            .unwrap_or(false);
        if has_counter {
            let mut conf = parking_lot::RwLockUpgradableReadGuard::upgrade(conf);
            if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
                r.key_manager.record_key_success(key_used);
            }
        }
    }
}
