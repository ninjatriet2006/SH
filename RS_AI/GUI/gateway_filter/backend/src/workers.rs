use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use crate::proxy::AppState;
use crate::state::ServerContext;
use crate::commands;

/// Fix Phantom ON: tự động khởi chạy các tunnel đang enabled lúc boot (tuần tự,
/// chạy nền để không chặn cửa sổ app). Tunnel nào start thất bại thì GIỮ NGUYÊN enabled,
/// chỉ hạ status xuống Offline + ghi lý do — TUYỆT ĐỐI KHÔNG tự tắt config của user.
pub fn spawn_boot_tunnel_worker(boot_ctx: ServerContext, boot_handle: AppHandle) {
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
                    t.status = crate::vpn::TunnelStatus::Offline;
                    t.last_error = Some(format!("Boot auto-start failed: {}", e));
                    // Xóa IP/latency cũ: start thất bại sớm (VD: kẹt lock, cổng bị chiếm)
                    // có thể return trước khi tới bước test — IP cũ trong config sẽ thành ghost.
                    t.last_exit_ip = None;
                    t.last_latency_ms = None;
                    t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                    let _ = conf.save_to_disk();
                }
                changed = true;
            } else {
                // Start thành công cũng đổi metadata (Online/IP) → refresh chung 1 lần cuối.
                changed = true;
            }
        }
        if changed {
            boot_ctx.app_state.refresh_clients();
        }
        let _ = boot_handle.emit(
            "tunnel-status-changed",
            serde_json::json!({ "boot": true }),
        );
    });
}

/// Background Worker: Chế độ "Đi Tuần" (Proactive VPN Check) định kỳ mỗi 60 giây
pub fn spawn_proactive_vpn_worker(app_state: Arc<AppState>, handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let enabled_tunnels: Vec<crate::vpn::OutboundTunnel> = {
                let conf = app_state.config.read();
                conf.tunnels.iter().filter(|t| t.enabled).cloned().collect()
            };

            for tunnel in enabled_tunnels {
                // Tunnel Direct không có tiến trình CLI để kiểm tra — luôn coi là Online,
                // tránh gọi ipify mỗi phút một cách vô ích (rate-limit + tốn tài nguyên).
                if tunnel.protocol == crate::vpn::TunnelProtocol::Direct {
                    let mut conf = app_state.config.write();
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
                    let mut conf = app_state.config.write();
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
                        // P3: chỉ ghi đĩa khi có thay đổi thực chất (status/error/IP/latency) —
                        // trước đây save mỗi vòng 60s/tunnel dù không đổi gì (hao SSD, đánh thức watcher).
                        // last_checked_at vẫn cập nhật RAM để UI hiện giờ check mới nhất.
                        let new_error = test_res.error;
                        let error_changed = t.last_error != new_error;
                        let ip_changed = test_res.success
                            && (t.last_exit_ip != test_res.exit_ip
                                || t.last_latency_ms != test_res.latency_ms);
                        let cleared = !test_res.success
                            && (t.last_exit_ip.is_some() || t.last_latency_ms.is_some());
                        t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                        t.last_error = new_error;
                        if test_res.success {
                            t.last_exit_ip = test_res.exit_ip;
                            t.last_latency_ms = test_res.latency_ms;
                        } else {
                            t.last_exit_ip = None;
                            t.last_latency_ms = None;
                        }
                        if status_changed || error_changed || ip_changed || cleared {
                            let _ = conf.save_to_disk();
                        }
                    }
                }

                if status_changed {
                    let _ = handle.emit("tunnel-status-changed", serde_json::json!({
                        "tunnel_id": tunnel.id,
                        "success": test_res.success,
                    }));
                }
            }
        }
    });
}
