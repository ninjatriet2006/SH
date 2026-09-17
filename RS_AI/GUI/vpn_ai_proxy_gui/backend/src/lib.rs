pub mod fingerprint;
pub mod monitor;
pub mod proxy;
pub mod vpn;

use std::sync::Arc;
use tauri::State;

use crate::monitor::RawTrafficLog;
use crate::proxy::manager::ListenerManager;
use crate::proxy::{AppState, GatewayConfig, RouteRule};
use crate::vpn::{OutboundTunnel, TunnelManager, TunnelTestResult};

pub struct ServerContext {
    pub app_state: Arc<AppState>,
    pub listener_mgr: Arc<ListenerManager>,
}

mod commands {
    use super::*;

    /// Single source of truth cho "tunnel CLI đang chạy": quét config.tunnels tìm tunnel
    /// có enabled + có start_command + chưa Offline (thay cho biến static riêng dễ lệch pha).
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
                    && t.status != crate::vpn::TunnelStatus::Offline
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
        new_config: GatewayConfig,
        app: tauri::AppHandle,
        ctx: State<'_, ServerContext>,
    ) -> Result<(), String> {
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

    /// Logic dùng chung: khởi động tiến trình CLI + kiểm tra kết nối.
    /// Trả về (thông báo, online).
    /// Fix 1: tunnel không có CLI (Direct Internet) → Ok "No process required" + Online, không văng lỗi.
    pub(crate) async fn start_process_inner(ctx: &ServerContext, tunnel_id: &str) -> Result<(String, bool), String> {
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
        {
            let conf = ctx.app_state.config.read();
            if let Some((_other_id, other_name)) = find_running_cli_tunnel(&conf, tunnel_id) {
                return Err(format!(
                    "Lỗi: Tunnel [{}] đang chạy. Bạn phải tự tay tắt Tunnel [{}] trước khi có thể chạy Tunnel [{}]!",
                    other_name, other_name, tunnel_clone.name
                ));
            }

            if let Some(addr) = TunnelManager::endpoint_addr(&tunnel_clone.endpoint) {
                if std::net::TcpListener::bind(addr).is_err() {
                    return Err(format!(
                        "Lỗi: Cổng {} đã bị chiếm dụng bởi tiến trình ngoại lai. Hãy dùng nút Force Stop / Kill để giải phóng trước khi chạy VPN.",
                        addr
                    ));
                }
            }
        }

        let output = match TunnelManager::run_tunnel_command_timeout(&cmd, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await {
            Ok(o) => o,
            Err(e) => return Err(e),
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
                    t.last_exit_ip = test_res.exit_ip;
                    t.last_latency_ms = test_res.latency_ms;
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
                TunnelManager::run_tunnel_command_timeout(&cmd, crate::vpn::TUNNEL_CMD_TIMEOUT_SECS).await?
            }
            _ => "No process required".to_string(),
        };

        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.status = crate::vpn::TunnelStatus::Offline;
                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                t.last_error = Some("Stopped by user".to_string());
                let _ = conf.save_to_disk();
            }
        }

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
                    && t.status != crate::vpn::TunnelStatus::Offline
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
            if let Some(sc) = start_cmd.as_deref() {
                if let Some(bin) = sc.split_whitespace().next() {
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
                let _ = conf.save_to_disk();
            }
        }
        ctx.app_state.refresh_clients();

        if results.is_empty() {
            Err(last_err.unwrap_or_else(|| "Force stop failed".to_string()))
        } else {
            Ok(results.join(" | "))
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
                }
                let _ = conf.save_to_disk();
            }
        }
        Ok(res)
    }

    #[tauri::command]
    pub fn get_raw_traffic(ctx: State<'_, ServerContext>) -> Vec<RawTrafficLog> {
        ctx.app_state.logs.get_all()
    }

    #[tauri::command]
    pub fn clear_logs(ctx: State<'_, ServerContext>) {
        ctx.app_state.logs.clear();
        // Xóa luôn file disk log — trước đây nút Clear chỉ xóa RAM,
        // file traffic_log.jsonl vẫn phình to và log cũ hiện lại sau restart.
        let log_path = crate::monitor::get_traffic_log_path();
        let _ = std::fs::write(&log_path, "");
    }

    #[tauri::command]
    pub fn generate_key_file(endpoint_name: String) -> Result<String, String> {
        // Nhất quán với GatewayConfig::config_path(): ưu tiên VPN_AI_PROXY_CONFIG_DIR (portable mode)
        let base_dir = if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
            std::path::PathBuf::from(dir).join("keys")
        } else if let Some(home) = std::env::var_os("HOME") {
            std::path::PathBuf::from(home)
                .join(".config")
                .join("vpn_ai_proxy_gui")
                .join("keys")
        } else {
            std::path::PathBuf::from("keys")
        };

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
}

pub fn run() {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    let initial_config = GatewayConfig::load_or_default();
    let app_state = Arc::new(AppState::new(initial_config));
    let listener_mgr = Arc::new(ListenerManager::new());

    let server_ctx = ServerContext {
        app_state: Arc::clone(&app_state),
        listener_mgr: Arc::clone(&listener_mgr),
    };

    tauri::Builder::default()
        .manage(server_ctx)
        .setup(move |app| {
            // Spawn dynamic listeners safely inside Tauri async runtime
            let handle = app.handle().clone();
            *app_state.app_handle.write() = Some(handle.clone());
            listener_mgr.sync_listeners(Arc::clone(&app_state), Some(handle.clone()));

            // Fix Phantom ON: tự động khởi chạy các tunnel đang enabled lúc boot (tuần tự,
            // chạy nền để không chặn cửa sổ app). Tunnel nào start thất bại thì GIỮ NGUYÊN enabled,
            // chỉ hạ status xuống Offline + ghi lý do — TUYỆT ĐỐI KHÔNG tự tắt config của user
            // (lỗi boot có thể chỉ là tạm thời: daemon VPN chưa lên, mạng chưa có...).
            let boot_ctx = ServerContext {
                app_state: Arc::clone(&app_state),
                listener_mgr: Arc::clone(&listener_mgr),
            };
            let boot_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let ids: Vec<String> = {
                    let conf = boot_ctx.app_state.config.read();
                    conf.tunnels
                        .iter()
                        .filter(|t| t.enabled)
                        .map(|t| t.id.clone())
                        .collect()
                };
                let mut changed = false;
                for id in ids {
                    if let Err(e) = commands::start_process_inner(&boot_ctx, &id).await {
                        eprintln!("Boot auto-start tunnel [{}] failed: {}", id, e);
                        let mut conf = boot_ctx.app_state.config.write();
                        if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == id) {
                            // Giữ enabled của user, chỉ phản ánh thực tế qua status
                            t.status = crate::vpn::TunnelStatus::Offline;
                            t.last_error = Some(format!("Boot auto-start failed: {}", e));
                            t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                            let _ = conf.save_to_disk();
                        }
                        changed = true;
                    }
                }
                if changed {
                    boot_ctx.app_state.refresh_clients();
                }
                use tauri::Emitter;
                let _ = boot_handle.emit(
                    "tunnel-status-changed",
                    serde_json::json!({ "boot": true }),
                );
            });

            // Background Worker: Chế độ "Đi Tuần" (Proactive VPN Check) định kỳ mỗi 60 giây
            let bg_state = Arc::clone(&app_state);
            let bg_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    
                    let enabled_tunnels: Vec<crate::vpn::OutboundTunnel> = {
                        let conf = bg_state.config.read();
                        conf.tunnels.iter().filter(|t| t.enabled).cloned().collect()
                    };

                    for tunnel in enabled_tunnels {
                        // Tunnel Direct không có tiến trình CLI để kiểm tra — luôn coi là Online,
                        // tránh gọi ipify mỗi phút một cách vô ích (rate-limit + tốn tài nguyên).
                        if tunnel.protocol == crate::vpn::TunnelProtocol::Direct {
                            let mut conf = bg_state.config.write();
                            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel.id) {
                                if t.status != crate::vpn::TunnelStatus::Online {
                                    t.status = crate::vpn::TunnelStatus::Online;
                                    t.last_error = None;
                                    t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                                    let _ = conf.save_to_disk();
                                }
                            }
                            continue;
                        }
                        let test_res = crate::vpn::TunnelManager::test_tunnel(&tunnel).await;
                        let mut status_changed = false;
                        {
                            let mut conf = bg_state.config.write();
                            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel.id) {
                                let new_status = if test_res.success {
                                    crate::vpn::TunnelStatus::Online
                                } else {
                                    crate::vpn::TunnelStatus::Offline
                                };
                                if t.status != new_status {
                                    t.status = new_status;
                                    status_changed = true;
                                }
                                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                                t.last_error = test_res.error;
                                if test_res.success {
                                    t.last_exit_ip = test_res.exit_ip;
                                    t.last_latency_ms = test_res.latency_ms;
                                }
                                let _ = conf.save_to_disk();
                            }
                        }

                        if status_changed {
                            use tauri::Emitter;
                            let _ = bg_handle.emit("tunnel-status-changed", serde_json::json!({
                                "tunnel_id": tunnel.id,
                                "success": test_res.success,
                            }));
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::add_or_update_tunnel,
            commands::toggle_tunnel,
            commands::toggle_route,
            commands::delete_tunnel,
            commands::add_or_update_route,
            commands::delete_route,
            commands::advance_endpoint_key,
            commands::test_single_tunnel,
            commands::start_tunnel_process,
            commands::stop_tunnel_process,
            commands::get_active_cli_tunnel,
            commands::force_stop_tunnel,
            commands::get_raw_traffic,
            commands::clear_logs,
            commands::generate_key_file,
            commands::open_key_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
