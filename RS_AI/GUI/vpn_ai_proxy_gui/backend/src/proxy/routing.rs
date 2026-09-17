use crate::config::RouteRule;
use crate::vpn::{OutboundTunnel, TunnelManager};

/// So khớp prefix theo ranh giới segment: "/v1" khớp "/v1/models" nhưng KHÔNG khớp "/v1beta/x".
/// Dùng cho cả route matching và strip prefix để tránh nuốt nhầm route.
pub fn prefix_matches(path: &str, prefix: &str) -> bool {
    if prefix == "/" || prefix.is_empty() {
        return true;
    }
    if !path.starts_with(prefix) {
        return false;
    }
    matches!(path.as_bytes().get(prefix.len()), None | Some(b'/'))
}

/// Xây dựng URL đích thông minh (Auto-detect + Overlap Failsafe):
/// Cắt bỏ `path_prefix` tương ứng nếu request path bắt đầu bằng prefix đó.
/// Nếu segment cuối của `target_base_url` trùng với segment đầu của phần sub_path còn lại,
/// loại bỏ đoạn trùng lặp đó để chống lỗi lặp /v1/v1.
pub fn build_target_url(target_base_url: &str, path_prefix: &str, req_path: &str, query: &str) -> String {
    let clean_base = target_base_url.trim_end_matches('/');
    let clean_prefix = if path_prefix == "/" { "" } else { path_prefix.trim_end_matches('/') };
    
    let sub_path = if !clean_prefix.is_empty()
        && (req_path == clean_prefix
            || req_path.as_bytes().get(clean_prefix.len()) == Some(&b'/'))
    {
        &req_path[clean_prefix.len()..]
    } else {
        req_path
    };
    
    let mut clean_sub = sub_path.trim_start_matches('/');

    // Overlap Failsafe: Trích xuất segment cuối của target_base_url (VD: "v1" trong "https://api.openai.com/v1")
    if let Some(last_segment) = clean_base.rsplit('/').next() {
        if !last_segment.is_empty() {
            // Kiểm tra xem clean_sub có bắt đầu bằng segment đó không (VD: "v1/models" hoặc "v1")
            if clean_sub == last_segment {
                clean_sub = "";
            } else if clean_sub.starts_with(&format!("{}/", last_segment)) {
                clean_sub = &clean_sub[last_segment.len() + 1..];
            }
        }
    }

    if clean_sub.is_empty() {
        format!("{}{}", clean_base, query)
    } else {
        format!("{}/{}{}", clean_base, clean_sub, query)
    }
}

/// Tìm rule khớp nhất trên cổng và path chỉ định (ưu tiên prefix dài nhất và khớp segment)
pub fn match_route<'a>(routes: &'a mut [RouteRule], port: u16, path: &str) -> Option<&'a mut RouteRule> {
    routes
        .iter_mut()
        .filter(|r| r.enabled && r.port == port && prefix_matches(path, &r.path_prefix))
        .max_by_key(|r| r.path_prefix.len())
}

/// Failover chọn tunnel khỏe nhất nếu tunnel gán với rule bị offline hoặc không tồn tại
pub fn resolve_tunnel_id(tunnels: &[OutboundTunnel], assigned_id: &str) -> Option<String> {
    TunnelManager::select_healthy_tunnel_id(tunnels, assigned_id)
}
