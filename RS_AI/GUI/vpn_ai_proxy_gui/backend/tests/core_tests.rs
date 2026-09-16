#[cfg(test)]
mod tests {
    use std::io::Write;
    use vpn_ai_proxy_gui_lib::monitor::{RawTrafficLog, RingBufferLog};
    use vpn_ai_proxy_gui_lib::proxy::key_manager::EndpointKeyManager;

    #[test]
    fn test_raw_traffic_ring_buffer_all_statuses() {
        let buffer = RingBufferLog::new(2);

        // Ghi nhận packet 200 OK thành công
        buffer.push(RawTrafficLog {
            id: "pkt-1".to_string(),
            timestamp: "10:00:00".to_string(),
            route_id: "openai_rule".to_string(),
            method: "POST".to_string(),
            port: 3000,
            path: "/v1/chat/completions".to_string(),
            target_url: "https://api.openai.com/v1/chat/completions".to_string(),
            tunnel_id: "adguard_default".to_string(),
            key_used_preview: Some("sk-1111".to_string()),
            status_code: 200,
            duration_ms: 150,
            is_streaming: false,
            raw_request_headers: vec![("user-agent".to_string(), "curl/8.0".to_string())],
            raw_forwarded_headers: vec![("user-agent".to_string(), "stealth-engine".to_string())],
            raw_request_body: r#"{"model":"gpt-4o","messages":[]}"#.to_string(),
            raw_response_headers: vec![("content-type".to_string(), "application/json".to_string())],
            raw_response_body: r#"{"id":"chatcmpl-123","choices":[]}"#.to_string(),
            leaked_findings: vec![],
        });

        // Ghi nhận packet 429 Rate Limit
        buffer.push(RawTrafficLog {
            id: "pkt-2".to_string(),
            timestamp: "10:00:01".to_string(),
            route_id: "custom_mirror".to_string(),
            method: "POST".to_string(),
            port: 3001,
            path: "/chat".to_string(),
            target_url: "https://abc.xyz/v1/chat".to_string(),
            tunnel_id: "adguard_default".to_string(),
            key_used_preview: Some("sk-2222".to_string()),
            status_code: 429,
            duration_ms: 50,
            is_streaming: false,
            raw_request_headers: vec![],
            raw_forwarded_headers: vec![],
            raw_request_body: r#"{"test":1}"#.to_string(),
            raw_response_headers: vec![],
            raw_response_body: r#"{"error":{"message":"quota exceeded"}}"#.to_string(),
            leaked_findings: vec![],
        });

        assert_eq!(buffer.count(), 2);
        let logs = buffer.get_all();
        assert_eq!(logs[0].status_code, 200);
        assert_eq!(logs[1].status_code, 429);
        assert!(logs[0].raw_response_body.contains("chatcmpl-123"));
        assert!(logs[1].raw_response_body.contains("quota exceeded"));
    }

    #[test]
    fn test_endpoint_dedicated_key_file_rotation() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_dedicated_key.txt");
        {
            let mut file = std::fs::File::create(&file_path).unwrap();
            writeln!(file, "key-aaa-1").unwrap();
            writeln!(file, "key-bbb-2").unwrap();
        }

        let mut km = EndpointKeyManager {
            key_file_path: Some(file_path.to_str().unwrap().to_string()),
            current_key_index: 0,
            total_keys: 0,
            current_key_preview: None,
            last_switched_at: None,
            ..Default::default()
        };

        km.refresh_metadata();
        assert_eq!(km.total_keys, 2);
        assert_eq!(km.get_active_key(), Some("key-aaa-1".to_string()));

        km.advance_to_next_key();
        assert_eq!(km.get_active_key(), Some("key-bbb-2".to_string()));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_build_target_url_auto_routing() {
        use vpn_ai_proxy_gui_lib::proxy::build_target_url;

        // Case 1: Prefix /v1, Target https://gemini/v1, Req /v1/models -> https://gemini/v1/models
        assert_eq!(
            build_target_url("https://gemini/v1", "/v1", "/v1/models", ""),
            "https://gemini/v1/models"
        );

        // Case 2: Prefix /gemini, Target https://gemini/v1, Req /gemini/models -> https://gemini/v1/models
        assert_eq!(
            build_target_url("https://gemini/v1", "/gemini", "/gemini/models", ""),
            "https://gemini/v1/models"
        );

        // Case 3: Prefix /, Target https://gemini/v1, Req /models -> https://gemini/v1/models
        assert_eq!(
            build_target_url("https://gemini/v1", "/", "/models", ""),
            "https://gemini/v1/models"
        );

        // Case 4: With query parameters and double slashes prevented
        assert_eq!(
            build_target_url("https://api.openai.com/v1/", "/v1", "/v1/chat/completions", "?stream=true"),
            "https://api.openai.com/v1/chat/completions?stream=true"
        );
    }

    #[test]
    fn test_truncate_log_body() {
        use vpn_ai_proxy_gui_lib::proxy::truncate_log_body;

        let small = "hello world";
        assert_eq!(truncate_log_body(small, 2048), "hello world");

        let large = "a".repeat(3000);
        let truncated = truncate_log_body(&large, 2048);
        assert!(truncated.contains("... [Truncated 3000 chars]"));
        assert_eq!(truncated.chars().take(2048).count(), 2048);
    }

    #[test]
    fn test_generate_key_file() {
        let temp_keys_dir = std::env::temp_dir().join("vpn_ai_proxy_test_keys");
        let _ = std::fs::create_dir_all(&temp_keys_dir);
        let key_file = temp_keys_dir.join("openai_test_keys.txt");
        if !key_file.exists() {
            std::fs::write(&key_file, "# Paste API keys here\n").unwrap();
        }
        assert!(key_file.exists());
        let content = std::fs::read_to_string(&key_file).unwrap();
        assert!(content.contains("# Paste API keys here"));
        let _ = std::fs::remove_dir_all(temp_keys_dir);
    }

    #[test]
    fn test_key_lifecycle_filtering() {
        let temp_dir = std::env::temp_dir().join("vpn_ai_proxy_lifecycle_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let main_file = temp_dir.join("main_keys.txt");
        let failed_file = temp_dir.join("failed_keys.txt");

        std::fs::write(&main_file, "key_good\nkey_dead_401\nkey_quota_403\n").unwrap();

        let mut km = EndpointKeyManager {
            key_file_path: Some(main_file.to_str().unwrap().to_string()),
            failed_key_file_path: Some(failed_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        // 1. Test 401: Xóa vĩnh viễn khỏi file chính
        km.remove_key_from_main_file("key_dead_401").unwrap();
        let main_content = std::fs::read_to_string(&main_file).unwrap();
        assert!(!main_content.contains("key_dead_401"));
        assert!(main_content.contains("key_good"));
        assert!(main_content.contains("key_quota_403"));

        // 2. Test 403: Di chuyển sang file failed
        km.move_key_to_failed_file("key_quota_403").unwrap();
        let main_content_after = std::fs::read_to_string(&main_file).unwrap();
        let failed_content = std::fs::read_to_string(&failed_file).unwrap();
        assert!(!main_content_after.contains("key_quota_403"));
        assert!(failed_content.contains("key_quota_403"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_tunnel_dependency_check_offline() {
        use vpn_ai_proxy_gui_lib::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol};

        // Test một cổng TCP không tồn tại (port 54321 trên localhost)
        let fake_tunnel = OutboundTunnel {
            id: "fake_dead_tunnel".to_string(),
            name: "Dead SOCKS Proxy".to_string(),
            protocol: TunnelProtocol::Socks5,
            endpoint: "127.0.0.1:54321".to_string(),
            enabled: true,
            status: vpn_ai_proxy_gui_lib::vpn::TunnelStatus::Unknown,
            max_concurrent_streams: 0,
            start_command: None,
            stop_command: None,
            last_checked_at: None,
            last_error: None,
            last_exit_ip: None,
            last_latency_ms: None,
            tags: vec![],
        };

        let result = TunnelManager::test_tunnel(&fake_tunnel).await;
        assert!(!result.success);
        assert!(result.error.is_some());
        let err_text = result.error.unwrap();
        assert!(err_text.contains("Proxy unreachable"));
    }

    #[test]
    fn test_build_target_url_overlap_failsafe() {
        use vpn_ai_proxy_gui_lib::proxy::build_target_url;

        // Case 1 (Overlap): Target https://api/v1 + Req Path /v1/models -> https://api/v1/models (Loại bỏ /v1 dư)
        assert_eq!(
            build_target_url("https://api/v1", "/v1", "/v1/models", ""),
            "https://api/v1/models"
        );

        // Case 2 (Bình thường): Target https://api/v2 + Req Path /models -> https://api/v2/models
        assert_eq!(
            build_target_url("https://api/v2", "/v2", "/v2/models", ""),
            "https://api/v2/models"
        );

        // Case 3 (Dư dấu Slash): Target https://api/v1/ + Req Path v1/models -> https://api/v1/models
        assert_eq!(
            build_target_url("https://api/v1/", "", "v1/models", ""),
            "https://api/v1/models"
        );

        // Case 4: Target https://api.openai.com/v1 + Prefix /openai + Req /openai/v1/chat/completions -> https://api.openai.com/v1/chat/completions
        assert_eq!(
            build_target_url("https://api.openai.com/v1", "/openai", "/openai/v1/chat/completions", ""),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_disk_log_rotation_and_append() {
        use std::io::BufRead;
        use vpn_ai_proxy_gui_lib::monitor::{append_disk_log, RawTrafficLog};

        let temp_dir = std::env::temp_dir().join(format!("test_disk_log_{}", uuid::Uuid::new_v4()));
        let log_file = temp_dir.join("traffic_log.jsonl");

        let make_log = |idx: usize| RawTrafficLog {
            id: format!("req-{}", idx),
            timestamp: "12:00:00".to_string(),
            route_id: "test_route".to_string(),
            method: "GET".to_string(),
            port: 3000,
            path: format!("/path/{}", idx),
            target_url: format!("https://example.com/{}", idx),
            tunnel_id: "direct".to_string(),
            key_used_preview: None,
            status_code: 200,
            duration_ms: 10,
            is_streaming: false,
            raw_request_headers: vec![],
            raw_forwarded_headers: vec![],
            raw_request_body: "".to_string(),
            raw_response_headers: vec![],
            raw_response_body: format!("response {}", idx),
            leaked_findings: vec![],
        };

        // Ghi 60 logs với max_disk_entries = 50
        for i in 1..=60 {
            append_disk_log(&log_file, &make_log(i), 50).unwrap();
        }

        // Kiểm tra file tồn tại và đếm số dòng
        let file = std::fs::File::open(&log_file).unwrap();
        let reader = std::io::BufReader::new(file);
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 50, "File log should be rotated to 50 lines");

        // Kiểm tra 10 dòng đầu đã bị cắt, dòng đầu tiên còn lại phải là req-11
        assert!(lines[0].contains("req-11"), "First line should be req-11 after trimming");
        assert!(lines.last().unwrap().contains("req-60"), "Last line should be req-60");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_format_logged_body_binary_and_plain() {
        use reqwest::header::{HeaderMap, HeaderValue, CONTENT_ENCODING, CONTENT_TYPE};
        use vpn_ai_proxy_gui_lib::proxy::format_logged_body;

        // Plain text / JSON
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let json_body = br#"{"message": "success"}"#;
        let formatted = format_logged_body(json_body, &headers);
        assert_eq!(formatted, r#"{"message": "success"}"#);

        // Binary Image
        let mut headers_img = HeaderMap::new();
        headers_img.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
        let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let formatted_img = format_logged_body(&fake_png, &headers_img);
        assert_eq!(formatted_img, "[Binary Media: image/png | 8 bytes]");

        // Binary Audio
        let mut headers_audio = HeaderMap::new();
        headers_audio.insert(CONTENT_TYPE, HeaderValue::from_static("audio/mpeg"));
        let fake_audio = vec![0u8; 100];
        let formatted_audio = format_logged_body(&fake_audio, &headers_audio);
        assert_eq!(formatted_audio, "[Binary Media: audio/mpeg | 100 bytes]");

        // Octet-stream
        let mut headers_bin = HeaderMap::new();
        headers_bin.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
        let fake_bin = vec![0u8; 42];
        let formatted_bin = format_logged_body(&fake_bin, &headers_bin);
        assert_eq!(formatted_bin, "[Binary Media: application/octet-stream | 42 bytes]");

        // Compressed data (e.g. gzip)
        let mut headers_gzip = HeaderMap::new();
        headers_gzip.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        headers_gzip.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let fake_gzip = vec![0x1f, 0x8b, 0x08, 0x00];
        let formatted_gzip = format_logged_body(&fake_gzip, &headers_gzip);
        assert_eq!(formatted_gzip, "[Compressed Data: gzip | 4 bytes]");
    }

    #[test]
    fn test_ring_buffer_update_response_body() {
        let buffer = RingBufferLog::new(5);
        let log = RawTrafficLog {
            id: "req-update-1".to_string(),
            timestamp: "12:00:00".to_string(),
            route_id: "test_route".to_string(),
            method: "POST".to_string(),
            port: 3000,
            path: "/chat".to_string(),
            target_url: "https://example.com/chat".to_string(),
            tunnel_id: "direct".to_string(),
            key_used_preview: None,
            status_code: 200,
            duration_ms: 100,
            is_streaming: true,
            raw_request_headers: vec![],
            raw_forwarded_headers: vec![],
            raw_request_body: "".to_string(),
            raw_response_headers: vec![],
            raw_response_body: "[Streaming SSE Response Live Flow]".to_string(),
            leaked_findings: vec![],
        };

        buffer.push(log);
        assert_eq!(buffer.get_all()[0].raw_response_body, "[Streaming SSE Response Live Flow]");

        buffer.update_response_body("req-update-1", "data: {\"done\": true}\n\n".to_string());
        let all_logs = buffer.get_all();
        assert_eq!(all_logs.len(), 1);
        assert_eq!(all_logs[0].raw_response_body, "data: {\"done\": true}\n\n");

        // Non-existent id does nothing
        buffer.update_response_body("non-existent-id", "test".to_string());
        assert_eq!(buffer.get_all()[0].raw_response_body, "data: {\"done\": true}\n\n");
    }

    #[test]
    fn test_fingerprint_header_spoofing() {
        use std::collections::HashMap;
        use vpn_ai_proxy_gui_lib::fingerprint::{FingerprintAnalyzer, FingerprintProfile};

        let mut headers = HashMap::new();
        headers.insert("cursor-version".to_string(), "1.0".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let mut spoof_headers = HashMap::new();
        spoof_headers.insert("cursor-version".to_string(), "9.9".to_string());

        let profile = FingerprintProfile {
            strip_ide_headers: true, // normally strips cursor
            spoof_headers,
            ..Default::default()
        };

        let cleaned = FingerprintAnalyzer::sanitize_headers(&headers, &profile);

        // cursor-version must be spoofed to 9.9
        assert_eq!(cleaned.get("cursor-version"), Some(&"9.9".to_string()));
        assert_eq!(cleaned.get("content-type"), Some(&"application/json".to_string()));

        // Test empty spoof header removal
        let mut empty_spoof = HashMap::new();
        empty_spoof.insert("content-type".to_string(), "".to_string());
        let profile_remove = FingerprintProfile {
            spoof_headers: empty_spoof,
            ..Default::default()
        };
        let cleaned_removed = FingerprintAnalyzer::sanitize_headers(&headers, &profile_remove);
        assert!(!cleaned_removed.contains_key("content-type"));
    }

    #[test]
    fn test_unified_session_rotation() {
        use vpn_ai_proxy_gui_lib::fingerprint::FingerprintProfile;
        use vpn_ai_proxy_gui_lib::proxy::key_manager::EndpointKeyManager;
        use vpn_ai_proxy_gui_lib::proxy::{GatewayConfig, RouteRule};
        use vpn_ai_proxy_gui_lib::vpn::{OutboundTunnel, TunnelProtocol, TunnelStatus};

        let temp_dir = std::env::temp_dir();
        let key_file = temp_dir.join(format!("test_rotation_keys_{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&key_file, "key-1\nkey-2\nkey-3\n").unwrap();

        let mut km = EndpointKeyManager {
            key_file_path: Some(key_file.to_str().unwrap().to_string()),
            ..Default::default()
        };
        km.refresh_metadata();
        assert_eq!(km.get_active_key(), Some("key-1".to_string()));

        let mut config = GatewayConfig {
            config_version: 1,
            tunnels: vec![
                OutboundTunnel {
                    id: "tunnel-a".to_string(),
                    name: "Tunnel A".to_string(),
                    protocol: TunnelProtocol::Direct,
                    endpoint: "".to_string(),
                    enabled: true,
                    status: TunnelStatus::Online,
                    max_concurrent_streams: 0,
                    start_command: None,
                    stop_command: None,
                    last_checked_at: None,
                    last_error: None,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec![],
                },
                OutboundTunnel {
                    id: "tunnel-b".to_string(),
                    name: "Tunnel B".to_string(),
                    protocol: TunnelProtocol::Direct,
                    endpoint: "".to_string(),
                    enabled: true,
                    status: TunnelStatus::Online,
                    max_concurrent_streams: 0,
                    start_command: None,
                    stop_command: None,
                    last_checked_at: None,
                    last_error: None,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec![],
                },
            ],
            routes: vec![RouteRule {
                id: "route-1".to_string(),
                name: "Route 1".to_string(),
                port: 3000,
                path_prefix: "/v1".to_string(),
                target_base_url: "https://api.openai.com/v1".to_string(),
                tunnel_id: "tunnel-a".to_string(),
                enabled: true,
                strip_prefix: false,
                key_manager: km,
                custom_auth_token: None,
            }],
            fingerprint_profile: FingerprintProfile::default(),
            fingerprint_pool: vec![
                FingerprintProfile {
                    mode: "stealth-1".to_string(),
                    ..Default::default()
                },
                FingerprintProfile {
                    mode: "stealth-2".to_string(),
                    ..Default::default()
                },
            ],
            active_fingerprint_index: 0,
            max_log_entries: 100,
            max_disk_log_entries: 1000,
        };

        // Simulating Unified Session Rotation logic
        let enabled_tunnel_ids: Vec<String> = config
            .tunnels
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.id.clone())
            .collect();

        // Step 1: Advance key & rotate tunnel
        if let Some(r) = config.routes.iter_mut().find(|r| r.id == "route-1") {
            r.key_manager.advance_to_next_key();
            if enabled_tunnel_ids.len() > 1 {
                if let Some(curr_idx) = enabled_tunnel_ids.iter().position(|id| id == &r.tunnel_id) {
                    let next_idx = (curr_idx + 1) % enabled_tunnel_ids.len();
                    r.tunnel_id = enabled_tunnel_ids[next_idx].clone();
                }
            }
        }
        // Step 2: Rotate fingerprint index
        if !config.fingerprint_pool.is_empty() {
            config.active_fingerprint_index =
                (config.active_fingerprint_index + 1) % config.fingerprint_pool.len();
        }

        // Verify rotations:
        let route = &config.routes[0];
        assert_eq!(route.key_manager.get_active_key(), Some("key-2".to_string()));
        assert_eq!(route.tunnel_id, "tunnel-b");
        assert_eq!(config.active_fingerprint_index, 1);
        assert_eq!(config.get_active_fingerprint().mode, "stealth-2");

        let _ = std::fs::remove_file(key_file);
    }

    #[test]
    fn test_run_tunnel_command() {
        use vpn_ai_proxy_gui_lib::vpn::TunnelManager;
        let out = TunnelManager::run_tunnel_command("echo hello_vpn_manager").unwrap();
        assert!(out.contains("hello_vpn_manager"));
    }
}
