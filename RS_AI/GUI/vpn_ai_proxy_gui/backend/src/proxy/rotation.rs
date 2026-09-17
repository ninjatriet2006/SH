use std::sync::Arc;
use crate::proxy::AppState;

/// Xử lý xoay vòng phiên (Key Filtering + Unified Session Rotation) khi gặp mã lỗi 401 hoặc 403:
/// - 401 Unauthorized: Key sai/thu hồi -> tự động xóa vĩnh viễn khỏi file chính
/// - 403 Forbidden: Key hết quota -> chuyển sang file failed_key_file_path
/// - CRITICAL: Đổi Key đồng nghĩa với đổi Account. Bắt buộc phải xoay đồng bộ cả Fingerprint và Tunnel (IP VPN)
///   để tạo một Identity hoàn toàn mới, tránh Anti-fraud của AI (OpenAI/Anthropic) truy vết IP cũ và shadowban toàn pool.
pub fn handle_error_session_rotation(
    state: &Arc<AppState>,
    route_id: &str,
    status_code: u16,
    resolved_key: Option<&str>,
) {
    if status_code != 401 && status_code != 403 {
        return;
    }

    let mut key_was_removed = false;
    if let Some(key_used) = resolved_key {
        let mut conf = state.config.write();
        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
            if status_code == 401 {
                if let Ok(removed) = r.key_manager.remove_key_from_main_file(key_used) {
                    key_was_removed = removed;
                }
            } else if status_code == 403 {
                if let Ok(removed) = r.key_manager.move_key_to_failed_file(key_used) {
                    key_was_removed = removed;
                }
            }
            let _ = conf.save_to_disk();
        }

        // Báo cho Frontend cập nhật UI Real-time nếu danh sách Key có biến động
        if key_was_removed {
            if let Some(ref handle) = *state.app_handle.read() {
                use tauri::Emitter;
                let _ = handle.emit("route-keys-updated", serde_json::json!({ "route_id": route_id }));
            }
        }
    }

    // Unified Session Rotation: Xoay Key kế tiếp, chuyển tunnel khác và đổi fingerprint profile
    {
        let mut conf = state.config.write();
        let enabled_tunnel_ids: Vec<String> = conf.tunnels.iter().filter(|t| t.enabled).map(|t| t.id.clone()).collect();
        
        // 1. Advance route key:
        // TUYỆT ĐỐI KHÔNG gọi advance_to_next_key() nếu key đã bị xóa (key kế tiếp đã tự động trượt vào slot hiện tại)
        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
            if !key_was_removed {
                r.key_manager.advance_to_next_key();
            }
            // Chuyển sang tunnel enabled kế tiếp nếu có nhiều hơn 1 tunnel đang chạy
            if enabled_tunnel_ids.len() > 1 {
                if let Some(curr_idx) = enabled_tunnel_ids.iter().position(|id| id == &r.tunnel_id) {
                    let next_idx = (curr_idx + 1) % enabled_tunnel_ids.len();
                    r.tunnel_id = enabled_tunnel_ids[next_idx].clone();
                }
            }
        }
        // 2. Rotate fingerprint profile
        if !conf.fingerprint_pool.is_empty() {
            conf.active_fingerprint_index = (conf.active_fingerprint_index + 1) % conf.fingerprint_pool.len();
        }
        let _ = conf.save_to_disk();
    }
}
