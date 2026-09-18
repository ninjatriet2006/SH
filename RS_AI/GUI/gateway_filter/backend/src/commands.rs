use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

use crate::monitor::RawTrafficLog;
use crate::config::{GatewayConfig, RouteRule};
use crate::state::ServerContext;
use crate::vpn::{OutboundTunnel, TunnelManager, TunnelTestResult};

/// Single source of truth cho "tunnel CLI đang chạy": quét config.tunnels tìm tunnel
/// có enabled + có start_command + status Online (thay cho biến static riêng dễ lệch pha).
/// Yêu cầu Online (không phải "chưa Offline"): tunnel Unknown mới tạo/chưa từng start
/// không được tính là đang chạy — nếu không badge RUNNING hiện giả và start tunnel
/// khác bị reject oan bởi lock độc quyền.
fn find_running_cli_tunnel(conf: &GatewayConfig, exclude_id: &str) -> Option<(String, String)> {
    conf.tunnels
        .iter()
        .find(|t| {
            t.id != exclude_id
                && t.enabled
                && t.start_command
                    .as_deref()
                    .map(|c| !c.trim().is_empty())
                    .unwrap_or(false)
                && t.status == crate::vpn::TunnelStatus::Online
        })
        .map(|t| (t.id.clone(), t.name.clone()))
}

#[tauri::command]
pub fn get_config(ctx: State<'_, ServerContext>) -> GatewayConfig {
    let mut conf = ctx.app_state.config.write();
    for r in &mut conf.routes {
        r.key_manager.refresh_metadata();
    }
    conf.clone()
}

#[tauri::command]
pub fn save_config(
    mut new_config: GatewayConfig,
    app: tauri::AppHandle,
    ctx: State<'_, ServerContext>,
) -> Result<(), String> {
    new_config.clamp_fingerprint_indices();
    let _ = new_config.save_to_disk();
    {
        let mut conf = ctx.app_state.config.write();
        // Áp dụng giới hạn log RAM mới ngay lập tức (trước đây phải restart app)
        ctx.app_state.logs.set_max_entries(new_config.max_log_entries);
        *conf = new_config;
        for r in &mut conf.routes {
            r.key_manager.refresh_metadata();
        }
    }
    ctx.app_state.refresh_clients();
    ctx.listener_mgr
        .sync_listeners(Arc::clone(&ctx.app_state), Some(app));
    Ok(())
}

#[tauri::command]
pub fn add_or_update_tunnel(tunnel: OutboundTunnel, ctx: State<'_, ServerContext>) -> Result<(), String> {
    {
        let mut conf = ctx.app_state.config.write();
        if let Some(pos) = conf.tunnels.iter().position(|t| t.id == tunnel.id) {
            // Giữ lại trạng thái sức khỏe do health-check/background worker cập nhật —
            // modal Edit mở từ trước có thể chứa status cũ, không được đè ngược (State Overwrite).
            // NGOẠI LỆ: user đổi endpoint/protocol → status cũ vô nghĩa, reset về Unknown để test lại.
            let old = conf.tunnels[pos].clone();
            let endpoint_changed =
                old.endpoint != tunnel.endpoint || old.protocol != tunnel.protocol;
            conf.tunnels[pos] = tunnel;
            if endpoint_changed {
                conf.tunnels[pos].status = crate::vpn::TunnelStatus::Unknown;
                conf.tunnels[pos].last_checked_at = None;
                conf.tunnels[pos].last_error = None;
                conf.tunnels[pos].last_exit_ip = None;
                conf.tunnels[pos].last_latency_ms = None;
            } else {
                conf.tunnels[pos].status = old.status;
                conf.tunnels[pos].last_checked_at = old.last_checked_at;
                conf.tunnels[pos].last_error = old.last_error;
                conf.tunnels[pos].last_exit_ip = old.last_exit_ip;
                conf.tunnels[pos].last_latency_ms = old.last_latency_ms;
            }
        } else {
            conf.tunnels.push(tunnel);
        }
        let _ = conf.save_to_disk();
    }
    ctx.app_state.refresh_clients();
    Ok(())
}

/// Ghi nhận tunnel khởi động thất bại: hạ Offline + lưu lý do + xóa IP/latency cũ.
/// Invariant chống ghost IP: mọi đường start-fail đều phải qua đây (trừ tunnel không tồn tại
/// và path Direct — Online-by-definition, không có tiến trình để fail).
fn mark_start_failed(ctx: &ServerContext, tunnel_id: &str, reason: String) {
    let mut conf = ctx.app_state.config.write();
    if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
        t.status = crate::vpn::TunnelStatus::Offline;
        t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        t.last_error = Some(reason.clone());
        t.last_exit_ip = None;
        t.last_latency_ms = None;
        let _ = conf.save_to_disk();
    }
    drop(conf);
    // Ghi debug log để UI theo dõi được (tránh hộp lỗi tĩnh mù tịt nguyên nhân)
    ctx.app_state.log_tunnel_event(tunnel_id, "error", reason);
}

/// Logic dùng chung: khởi động tiến trình CLI + kiểm tra kết nối.
/// Trả về (thông báo, online).
/// Fix 1: tunnel không có CLI (Direct Internet) → Ok "No process required" + Online, không văng lỗi.
pub async fn start_process_inner(ctx: &ServerContext, tunnel_id: &str) -> Result<(String, bool), String> {
    let tunnel_clone = {
        let conf = ctx.app_state.config.read();
        match conf.tunnels.iter().find(|t| t.id == tunnel_id).cloned() {
            Some(tunnel) => tunnel,
            None => return Err(format!("Tunnel [{}] không tồn tại", tunnel_id)),
        }
    };

    // Fix 1: Tunnel Direct không cần tiến trình CLI — coi như Online ngay lập tức
    let cmd = match tunnel_clone.start_command.clone() {
        Some(c) if !c.trim().is_empty() => c,
        _ => {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.status = crate::vpn::TunnelStatus::Online;
                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                t.last_error = None;
                let _ = conf.save_to_disk();
            }
            return Ok(("No process required".to_string(), true));
        }
    };

    // Độc quyền CLI: quét config tìm tunnel CLI khác đang chạy (không dùng static riêng).
    // Lớp bảo vệ vật lý cuối cùng vẫn là pre-flight port check bên dưới.
    // Mọi đường reject đều persist Offline + xóa IP cũ (chống ghost IP xanh trên UI).
    // NOTE: parking_lot guard là !Send — scope read phải đóng TRƯỚC mọi .await.
    {
        let conf = ctx.app_state.config.read();
        if let Some((_other_id, other_name)) = find_running_cli_tunnel(&conf, tunnel_id) {
            let msg = format!(
                "Lỗi: Tunnel [{}] đang chạy. Bạn phải tự tay tắt Tunnel [{}] trước khi có thể chạy Tunnel [{}]!",
                other_name, other_name, tunnel_clone.name
            );
            drop(conf);
            mark_start_failed(ctx, tunnel_id, msg.clone());
            return Err(msg);
        }
    }

    // P4: endpoint_addr phân giải DNS đồng bộ (ToSocketAddrs) — bọc spawn_blocking
    // để DNS chậm không treo tokio worker của IPC (chạy ngoài mọi lock).
    let endpoint = tunnel_clone.endpoint.clone();
    let addr_opt = tokio::task::spawn_blocking(move || TunnelManager::endpoint_addr(&endpoint))
        .await
        .unwrap_or(None);
    if let Some(addr) = addr_opt {
        if std::net::TcpListener::bind(addr).is_err() {
            let msg = format!(
                "Lỗi: Cổng {} đã bị chiếm dụng bởi tiến trình ngoại lai. Hãy dùng nút Force Stop / Kill để giải phóng trước khi chạy VPN.",
                addr
            );
            mark_start_failed(ctx, tunnel_id, msg.clone());
            return Err(msg);
        }
    }

    let output = match TunnelManager::run_tunnel_command_timeout(&cmd, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await {
        Ok(o) => o,
        Err(e) => {
            // Tiến trình CLI spawn/run thất bại: persist để UI hiện đỏ + xóa ghost IP.
            mark_start_failed(ctx, tunnel_id, e.clone());
            return Err(e);
        }
    };

    // Chờ 500ms để process bind cổng rồi test kết nối
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let test_res = TunnelManager::test_tunnel(&tunnel_clone).await;

    {
        let mut conf = ctx.app_state.config.write();
        if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
            t.status = if test_res.success {
                crate::vpn::TunnelStatus::Online
            } else {
                crate::vpn::TunnelStatus::Offline
            };
            t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            t.last_error = test_res.error.clone();
            if test_res.success {
                t.last_exit_ip = test_res.exit_ip.clone();
                t.last_latency_ms = test_res.latency_ms;
                ctx.app_state.log_tunnel_event(
                    tunnel_id,
                    "ok",
                    format!(
                        "Start OK, exit IP: {}",
                        test_res.exit_ip.as_deref().unwrap_or("?")
                    ),
                );
            } else {
                t.last_exit_ip = None;
                t.last_latency_ms = None;
                ctx.app_state.log_tunnel_event(
                    tunnel_id,
                    "error",
                    format!(
                        "Start xong nhưng test rớt: {}",
                        test_res.error.as_deref().unwrap_or("unknown error")
                    ),
                );
            }
            let _ = conf.save_to_disk();
        }
    }

    if test_res.success {
        Ok((format!("Command executed. Result: {}", output.trim()), true))
    } else {
        Err(format!(
            "VPN process started but connectivity test failed: {}",
            test_res.error.unwrap_or_else(|| "unknown error".to_string())
        ))
    }
}

/// Logic dùng chung: dừng tiến trình CLI.
/// Fix 1: tunnel không có stop command → Ok "No process required" + Offline, không văng lỗi.
async fn stop_process_inner(ctx: &ServerContext, tunnel_id: &str) -> Result<String, String> {
    let cmd_opt = {
        let conf = ctx.app_state.config.read();
        match conf.tunnels.iter().find(|t| t.id == tunnel_id) {
            Some(t) => t.stop_command.clone(),
            None => return Err(format!("Tunnel [{}] không tồn tại", tunnel_id)),
        }
    };

    let output = match cmd_opt {
        Some(cmd) if !cmd.trim().is_empty() => {
            match TunnelManager::run_tunnel_command_timeout(&cmd, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await {
                Ok(o) => o,
                Err(e) => {
                    ctx.app_state.log_tunnel_event(
                        tunnel_id,
                        "error",
                        format!("Stop thất bại: {}", e),
                    );
                    return Err(e);
                }
            }
        }
        _ => "No process required".to_string(),
    };

    {
        let mut conf = ctx.app_state.config.write();
        if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
            t.status = crate::vpn::TunnelStatus::Offline;
            t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            t.last_error = Some("Stopped by user".to_string());
            // Tunnel đã dừng: exit IP cũ vô nghĩa, xóa để tránh ghost IP xanh trên UI.
            t.last_exit_ip = None;
            t.last_latency_ms = None;
            let _ = conf.save_to_disk();
        }
    }
    ctx.app_state.log_tunnel_event(tunnel_id, "info", "Đã dừng (Stopped by user)".to_string());

    Ok(format!("Command executed. Result: {}", output.trim()))
}

/// Fix 4: toggle_tunnel ôm toàn bộ vòng đời process — Frontend chỉ gọi đúng 1 IPC duy nhất.
/// - enabled == true: chạy start process + test kết nối. CHỈ KHI test Online mới cho vào Routing Pool.
/// - enabled == false: loại khỏi Routing Pool trước, sau đó mới stop process bên dưới.
#[tauri::command]
pub async fn toggle_tunnel(tunnel_id: String, enabled: bool, ctx: State<'_, ServerContext>) -> Result<(), String> {
    if enabled {
        start_process_inner(&ctx, &tunnel_id).await?;
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.enabled = true;
                let _ = conf.save_to_disk();
            } else {
                return Err(format!("Tunnel [{}] không tồn tại", tunnel_id));
            }
        }
        ctx.app_state.refresh_clients();
        Ok(())
    } else {
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.enabled = false;
                let _ = conf.save_to_disk();
            } else {
                return Err(format!("Tunnel [{}] không tồn tại", tunnel_id));
            }
        }
        ctx.app_state.refresh_clients();
        stop_process_inner(&ctx, &tunnel_id).await?;
        Ok(())
    }
}

#[tauri::command]
pub fn toggle_route(
    route_id: String,
    enabled: bool,
    app: tauri::AppHandle,
    ctx: State<'_, ServerContext>,
) -> Result<(), String> {
    {
        let mut conf = ctx.app_state.config.write();
        if let Some(pos) = conf.routes.iter().position(|r| r.id == route_id) {
            conf.routes[pos].enabled = enabled;
            let _ = conf.save_to_disk();
        } else {
            return Err(format!("Route [{}] not found", route_id));
        }
    }
    ctx.listener_mgr
        .sync_listeners(Arc::clone(&ctx.app_state), Some(app));
    Ok(())
}

#[tauri::command]
pub async fn start_tunnel_process(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    let (msg, _online) = start_process_inner(&ctx, &tunnel_id).await?;
    Ok(msg)
}

#[tauri::command]
pub async fn stop_tunnel_process(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    stop_process_inner(&ctx, &tunnel_id).await
}

/// Trả ID tunnel CLI đang chạy (None nếu không có) — quét trực tiếp config.tunnels.
/// Chỉ tính status Online: tunnel Unknown chưa từng start không được báo RUNNING giả.
#[tauri::command]
pub fn get_active_cli_tunnel(ctx: State<'_, ServerContext>) -> Option<String> {
    let conf = ctx.app_state.config.read();
    conf.tunnels
        .iter()
        .find(|t| {
            t.enabled
                && t.start_command
                    .as_deref()
                    .map(|c| !c.trim().is_empty())
                    .unwrap_or(false)
                && t.status == crate::vpn::TunnelStatus::Online
        })
        .map(|t| t.id.clone())
}

/// Force Stop / Kill: bỏ qua Mutex, gọi stop_command + killall để tiêu diệt tiến trình kẹt
#[tauri::command]
pub async fn force_stop_tunnel(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    let (stop_cmd, start_cmd, tunnel_name) = {
        let conf = ctx.app_state.config.read();
        match conf.tunnels.iter().find(|t| t.id == tunnel_id) {
            Some(t) => (t.stop_command.clone(), t.start_command.clone(), t.name.clone()),
            None => return Err(format!("Tunnel [{}] không tồn tại", tunnel_id)),
        }
    };

    let mut attempts: Vec<String> = Vec::new();
    if let Some(c) = stop_cmd.as_deref() {
        if !c.trim().is_empty() {
            attempts.push(c.to_string());
        }
    }
    // Lệnh kill dự phòng theo OS (killall không tồn tại trên Windows)
    fn kill_cmd(bin: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            format!("taskkill /F /IM {}", bin)
        }
        #[cfg(not(target_os = "windows"))]
        {
            format!("killall {}", bin)
        }
    }
    let lower = tunnel_name.to_lowercase();
    if lower.contains("adguard") {
        attempts.push(kill_cmd("adguardvpn-cli"));
    }
    if lower.contains("warp") {
        attempts.push(kill_cmd("warp-cli"));
    }
    if attempts.is_empty() {
        // Fallback cuối: kill theo binary của start_command — nhưng PHẢI bỏ wrapper
        // (sudo/doas/env/sh/bash) và chỉ lấy basename. Nếu không `sudo warp-cli ...`
        // thành `killall sudo` (giết mọi tiến trình sudo) hoặc `bash -c ...`
        // thành `killall bash` (thảm họa). Wrapper không match → bỏ qua an toàn.
        const WRAPPERS: &[&str] = &["sudo", "doas", "su", "env", "bash", "sh", "dash", "cmd"];
        if let Some(sc) = start_cmd.as_deref() {
            let real_bin = sc.split_whitespace().find_map(|tok| {
                let base = tok.rsplit('/').next().unwrap_or(tok);
                let base = base.rsplit('\\').next().unwrap_or(base);
                if base.is_empty() || WRAPPERS.contains(&base) {
                    None
                } else {
                    Some(base)
                }
            });
            if let Some(bin) = real_bin {
                attempts.push(kill_cmd(bin));
            }
        }
    }
    if attempts.is_empty() {
        return Err("Không có Stop Command hoặc tên CLI nào để force kill".to_string());
    }

    let mut results = Vec::new();
    let mut last_err: Option<String> = None;
    for attempt in &attempts {
        match TunnelManager::run_tunnel_command_timeout(attempt, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await {
            Ok(out) => results.push(format!("OK: {}", out.trim())),
            Err(e) => last_err = Some(format!("[{}] => {}", attempt, e)),
        }
    }

    // Force Kill bỏ qua mọi kiểm tra độc quyền: cứ bắn stop_command + killall,
    // trạng thái đồng bộ về sau (enabled=false, Offline).
    {
        let mut conf = ctx.app_state.config.write();
        if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
            // Fix 3: Force Kill phải hạ luôn cờ enabled để nút Toggle đồng bộ (kẻo UI vẫn xanh trong khi process đã chết)
            t.enabled = false;
            t.status = crate::vpn::TunnelStatus::Offline;
            t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            t.last_error = Some("Force stopped by user".to_string());
            // Process đã bị kill: exit IP cũ vô nghĩa, xóa để tránh ghost IP xanh trên UI.
            t.last_exit_ip = None;
            t.last_latency_ms = None;
            let _ = conf.save_to_disk();
        }
    }
    ctx.app_state.refresh_clients();

    if results.is_empty() {
        let msg = last_err.unwrap_or_else(|| "Force stop failed".to_string());
        ctx.app_state.log_tunnel_event(&tunnel_id, "error", format!("Force stop thất bại: {}", msg));
        Err(msg)
    } else {
        let msg = results.join(" | ");
        ctx.app_state.log_tunnel_event(
            &tunnel_id,
            "ok",
            format!("Force stop OK: {}", msg.chars().take(200).collect::<String>()),
        );
        Ok(msg)
    }
}

#[tauri::command]
pub async fn delete_tunnel(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<(), String> {
    // Best-effort: nếu tunnel đang chạy (enabled + có stop command) thì stop trước khi xóa,
    // tránh tiến trình mồ côi giữ cổng. Lỗi stop không chặn xóa (user đã quyết xóa).
    let stop_cmd: Option<String> = {
        let conf = ctx.app_state.config.read();
        conf.tunnels
            .iter()
            .find(|t| t.id == tunnel_id)
            .filter(|t| {
                t.enabled
                    && t.status != crate::vpn::TunnelStatus::Offline
                    && t.stop_command.as_deref().map(|c| !c.trim().is_empty()).unwrap_or(false)
            })
            .and_then(|t| t.stop_command.clone())
    };
    if let Some(cmd) = stop_cmd {
        let _ = TunnelManager::run_tunnel_command_timeout(&cmd, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await;
    }
        {
            let mut conf = ctx.app_state.config.write();
            conf.tunnels.retain(|t| t.id != tunnel_id);
            let _ = conf.save_to_disk();
        }
        // Dọn debug log của tunnel vừa xóa (không còn UI nào đọc)
        ctx.app_state.tunnel_events.lock().remove(&tunnel_id);
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub fn add_or_update_route(
    mut route: RouteRule,
    app: tauri::AppHandle,
    ctx: State<'_, ServerContext>,
) -> Result<(), String> {
    route.key_manager.refresh_metadata();
    {
        let mut conf = ctx.app_state.config.write();
        // Kẹp fingerprint_index về tầm pool hiện tại (xóa profile làm index cũ lệch).
        if !conf.fingerprint_pool.is_empty()
            && let Some(i) = route.fingerprint_index
        {
            route.fingerprint_index = Some(i % conf.fingerprint_pool.len());
        }
        if let Some(pos) = conf.routes.iter().position(|r| r.id == route.id) {
            conf.routes[pos] = route;
        } else {
            conf.routes.push(route);
        }
        let _ = conf.save_to_disk();
    }
    ctx.listener_mgr
        .sync_listeners(Arc::clone(&ctx.app_state), Some(app));
    Ok(())
}

#[tauri::command]
pub fn delete_route(
    route_id: String,
    app: tauri::AppHandle,
    ctx: State<'_, ServerContext>,
) -> Result<(), String> {
    {
        let mut conf = ctx.app_state.config.write();
        conf.routes.retain(|r| r.id != route_id);
        let _ = conf.save_to_disk();
    }
    // Dọn slot pacing của route vừa xóa (slots key theo route_id)
    ctx.app_state.pacing_slots.lock().remove(&route_id);
    ctx.listener_mgr
        .sync_listeners(Arc::clone(&ctx.app_state), Some(app));
    Ok(())
}

#[tauri::command]
pub fn advance_endpoint_key(route_id: String, ctx: State<'_, ServerContext>) -> Result<Option<String>, String> {
    Ok(ctx.app_state.advance_route_key(&route_id))
}

#[tauri::command]
pub async fn test_single_tunnel(tunnel: OutboundTunnel, ctx: State<'_, ServerContext>) -> Result<TunnelTestResult, String> {
    let res = TunnelManager::test_tunnel(&tunnel).await;
    // Persist kết quả test vào config state & file json
    {
        let mut conf = ctx.app_state.config.write();
        if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel.id) {
            t.status = if res.success {
                crate::vpn::TunnelStatus::Online
            } else {
                crate::vpn::TunnelStatus::Offline
            };
            t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            t.last_error = res.error.clone();
            if res.success {
                t.last_exit_ip = res.exit_ip.clone();
                t.last_latency_ms = res.latency_ms;
            } else {
                t.last_exit_ip = None;
                t.last_latency_ms = None;
            }
            let _ = conf.save_to_disk();
        }
    }
    // Ghi debug log để UI theo dõi (thay hộp kết quả tĩnh)
    if res.success {
        ctx.app_state.log_tunnel_event(
            &tunnel.id,
            "ok",
            format!(
                "Test OK, exit IP: {} ({}ms)",
                res.exit_ip.as_deref().unwrap_or("?"),
                res.latency_ms.unwrap_or(0)
            ),
        );
    } else {
        ctx.app_state.log_tunnel_event(
            &tunnel.id,
            "error",
            format!("Test thất bại: {}", res.error.as_deref().unwrap_or("unknown")),
        );
    }
    Ok(res)
}

/// Đăng nhập VPN CLI (lệnh cố định theo preset, suy từ tên tunnel).
/// - AdGuard: `adguardvpn-cli login`
/// - WARP: `warp-cli registration new`
/// Lệnh login có thể cần tương tác (trình duyệt/nhập liệu) → chạy với timeout,
/// stdin đóng nên sẽ fail nhanh kèm hướng dẫn trong output; user hoàn tất nốt
/// trong terminal nếu cần. Kết quả ghi vào debug log của tunnel.
#[tauri::command]
pub async fn login_tunnel_cli(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    // GIỮ LẠI tương thích IPC cũ: chuyển thành login nền 1 vòng gọn —
    // thực tế frontend mới dùng start_cli_login/cancel_cli_login bên dưới.
    start_cli_login_inner(&ctx, &tunnel_id).await
}

/// Lệnh login cố định theo preset. WARP bắt buộc `--accept-tos` trước
/// `registration new` (không accept ToS thì không bật được command nào).
fn login_command_for(tunnel_name: &str) -> Option<&'static str> {
    let lower = tunnel_name.to_lowercase();
    if lower.contains("adguard") {
        Some("adguardvpn-cli login")
    } else if lower.contains("warp") {
        Some("warp-cli --accept-tos && warp-cli registration new")
    } else {
        None
    }
}

/// Trích URL + mã device-code từ output login ("https://... user_code=XXXX").
fn extract_login_url(output: &str) -> Option<String> {
    let url = output
        .split_whitespace()
        .find(|t| t.starts_with("http"))?
        .trim_end_matches(|c: char| c == '.' || c == ',' || c == ')');
    let code = output
        .split("user_code=")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .map(|s| {
            s.trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
                .to_string()
        })
        .filter(|s| !s.is_empty());
    Some(match code {
        Some(c) => format!("{} (mã: {})", url, c),
        None => url.to_string(),
    })
}

fn emit_login(ctx: &ServerContext, event: &str, payload: serde_json::Value) {
    if let Some(handle) = ctx.app_state.app_handle.read().as_ref() {
        use tauri::Emitter;
        let _ = handle.emit(event, payload);
    }
}

/// Spawn tiến trình login nền (dùng chung cho IPC 1-phát và flow UI có hủy).
/// Trả Ok(message) khi spawn xong; waiter nền lo URL/finish/cleanup.
async fn start_cli_login_inner(
    ctx: &ServerContext,
    tunnel_id: &str,
) -> Result<String, String> {
    let tunnel_name = {
        let conf = ctx.app_state.config.read();
        match conf.tunnels.iter().find(|t| t.id == tunnel_id) {
            Some(t) => t.name.clone(),
            None => return Err(format!("Tunnel [{}] không tồn tại", tunnel_id)),
        }
    };
    let login_cmd = login_command_for(&tunnel_name).ok_or_else(|| {
        format!(
            "Tunnel [{}] không có preset login (chỉ hỗ trợ AdGuard/WARP).",
            tunnel_name
        )
    })?;

    let stdout = {
        let mut map = ctx.app_state.login_children.lock();
        if map.contains_key(tunnel_id) {
            return Err("Phiên login đang chạy rồi (bấm Hủy trước khi chạy lại)".to_string());
        }
        let mut child = tokio::process::Command::new("sh")
            .args(["-c", login_cmd])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Không khởi chạy được login: {}", e))?;
        let out = child.stdout.take();
        map.insert(tunnel_id.to_string(), child);
        out
    };

    ctx.app_state.log_tunnel_event(
        tunnel_id,
        "info",
        format!("Bắt đầu login: {}", login_cmd),
    );

    // Clone những gì waiter cần (không giữ lock nào qua await)
    let state = Arc::clone(&ctx.app_state);
    let tid = tunnel_id.to_string();
    tauri::async_runtime::spawn(async move {
        use tokio::io::AsyncReadExt;

        // Reader: chunk đầu (15s) tìm URL device-code để UI mở trình duyệt ngay.
        if let Some(mut out) = stdout {
            let mut buf = vec![0u8; 8192];
            let n = tokio::time::timeout(std::time::Duration::from_secs(15), out.read(&mut buf))
                .await
                .unwrap_or(Ok(0))
                .unwrap_or(0);
            if n > 0 {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(url) = extract_login_url(&text) {
                    state.login_info.lock().insert(tid.clone(), url.clone());
                    state.log_tunnel_event(&tid, "info", format!("Mở trình duyệt authorize: {}", url));
                    if let Some(handle) = state.app_handle.read().as_ref() {
                        use tauri::Emitter;
                        let _ = handle.emit(
                            "login-url-ready",
                            serde_json::json!({ "tunnel_id": tid, "url": url }),
                        );
                    }
                }
            }
            // Drain nền tới khi tiến trình thoát để pipe đầy không kẹt CLI
            tauri::async_runtime::spawn(async move {
                let mut rest = Vec::new();
                let _ = out.read_to_end(&mut rest).await;
            });
        }

        // Waiter: poll try_wait mỗi giây (không giữ lock qua await).
        // Cap 1500s (< 1725s hiệu lực code AdGuard) rồi kill dọn.
        let start = std::time::Instant::now();
        const CAP_SECS: u64 = 1500;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            enum Poll {
                Running,
                Done(std::process::ExitStatus),
                Gone,
            }
            let poll = {
                match state.login_children.lock().get_mut(&tid) {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => Poll::Done(status),
                        Ok(None) => Poll::Running,
                        Err(_) => Poll::Gone,
                    },
                    None => Poll::Gone,
                }
            };
            match poll {
                Poll::Running => {
                    if start.elapsed().as_secs() > CAP_SECS {
                        let child = state.login_children.lock().remove(&tid);
                        if let Some(mut c) = child {
                            let _ = c.kill().await;
                            let _ = c.wait().await;
                        }
                        state.login_info.lock().remove(&tid);
                        state.log_tunnel_event(&tid, "error", "Login quá hạn 25 phút, đã dừng".to_string());
                        if let Some(handle) = state.app_handle.read().as_ref() {
                            use tauri::Emitter;
                            let _ = handle.emit(
                                "login-finished",
                                serde_json::json!({ "tunnel_id": tid, "success": false, "error": "timeout" }),
                            );
                        }
                        break;
                    }
                }
                Poll::Done(status) => {
                    state.login_children.lock().remove(&tid);
                    state.login_info.lock().remove(&tid);
                    if status.success() {
                        state.log_tunnel_event(&tid, "ok", "Login thành công".to_string());
                    } else {
                        state.log_tunnel_event(
                            &tid,
                            "error",
                            format!("Login thất bại (exit {}), xem output hoặc thử lại", status),
                        );
                    }
                    if let Some(handle) = state.app_handle.read().as_ref() {
                        use tauri::Emitter;
                        let _ = handle.emit(
                            "login-finished",
                            serde_json::json!({ "tunnel_id": tid, "success": status.success() }),
                        );
                    }
                    break;
                }
                Poll::Gone => break, // Bị cancel từ nơi khác
            }
        }
    });

    Ok(format!("Đã khởi chạy login ({}), chờ URL authorize...", login_cmd))
}

/// Bắt đầu phiên login nền cho UI (trả ngay, URL/kết quả qua events).
#[tauri::command]
pub async fn start_cli_login(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    start_cli_login_inner(&ctx, &tunnel_id).await
}

/// Hủy phiên login đang chạy (kill nhẹ 1 lần; adguard bị kill có thể dump core
/// do lỗi của nó — systemd giữ, không rơi vào workspace).
#[tauri::command]
pub async fn cancel_cli_login(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
    let child = ctx.app_state.login_children.lock().remove(&tunnel_id);
    match child {
        Some(mut c) => {
            let _ = c.kill().await;
            let _ = c.wait().await;
            ctx.app_state.login_info.lock().remove(&tunnel_id);
            ctx.app_state.log_tunnel_event(&tunnel_id, "info", "Đã hủy phiên login".to_string());
            emit_login(
                &ctx,
                "login-finished",
                serde_json::json!({ "tunnel_id": tunnel_id, "success": false, "cancelled": true }),
            );
            Ok("Đã hủy phiên login".to_string())
        }
        None => Err("Không có phiên login nào đang chạy".to_string()),
    }
}

/// Map tunnel → URL authorize đang chờ (UI refresh sau reload).
#[tauri::command]
pub fn get_active_logins(ctx: State<'_, ServerContext>) -> HashMap<String, String> {
    ctx.app_state.login_info.lock().clone()
}

/// Mở URL trong trình duyệt hệ thống (chỉ http/https, không shell).
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    let u = url.trim();
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err("URL không hợp lệ (chỉ http/https)".to_string());
    }
    if u.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("URL chứa ký tự lạ".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(u)
            .spawn()
            .map_err(|e| format!("Không mở được trình duyệt: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", u])
            .spawn()
            .map_err(|e| format!("Không mở được trình duyệt: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(u)
            .spawn()
            .map_err(|e| format!("Không mở được trình duyệt: {}", e))?;
    }
    Ok(())
}

/// Snapshot debug log per-tunnel cho UI (RAM-only).
#[tauri::command]
pub fn get_tunnel_events(ctx: State<'_, ServerContext>) -> HashMap<String, Vec<crate::vpn::TunnelEvent>> {
    ctx.app_state.tunnel_events_snapshot()
}

#[tauri::command]
pub fn get_raw_traffic(ctx: State<'_, ServerContext>) -> Vec<RawTrafficLog> {
    ctx.app_state.logs.get_all()
}

#[tauri::command]
pub fn clear_logs(ctx: State<'_, ServerContext>) {
    ctx.app_state.logs.clear();
    let log_path = crate::monitor::get_traffic_log_path();
    let _ = std::fs::write(&log_path, "");
}

#[tauri::command]
pub fn generate_key_file(endpoint_name: String) -> Result<String, String> {
    // Nhất quán với GatewayConfig::config_dir(): ưu tiên GATEWAY_FILTER_CONFIG_DIR (portable mode)
    let base_dir = crate::config::GatewayConfig::config_dir().join("keys");

    std::fs::create_dir_all(&base_dir)
        .map_err(|e| format!("Failed to create keys directory: {}", e))?;

    let safe_name: String = endpoint_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let file_name = format!("{}_keys.txt", if safe_name.is_empty() { "default" } else { &safe_name });
    let file_path = base_dir.join(file_name);

    if !file_path.exists() {
        std::fs::write(&file_path, "# Paste API keys here, one per line\n")
            .map_err(|e| format!("Failed to write key file: {}", e))?;
    }

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_key_file(file_path: String) -> Result<(), String> {
    let p = std::path::Path::new(&file_path);
    if !p.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&file_path).spawn()
            .map_err(|e| format!("Failed to open file with xdg-open: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", &file_path]).spawn()
            .map_err(|e| format!("Failed to open file with cmd start: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&file_path).spawn()
            .map_err(|e| format!("Failed to open file with open: {}", e))?;
    }
    Ok(())
}
