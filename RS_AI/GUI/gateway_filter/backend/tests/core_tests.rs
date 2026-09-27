#[cfg(test)]
mod tests {
    use std::io::Write;
    use gateway_filter_lib::monitor::{RawTrafficLog, RingBufferLog};
    use gateway_filter_lib::proxy::key_manager::EndpointKeyManager;

    /// Serialize các test chạm `GATEWAY_FILTER_CONFIG_DIR` (env process-wide):
    /// chạy song song sẽ redirect save của nhau → assert sai + ghi nhầm chỗ.
    static CONFIG_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
            raw_forwarded_body: String::new(),
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
            raw_forwarded_body: String::new(),
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
        use gateway_filter_lib::proxy::build_target_url;

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

        // Case 5: Segment boundary protection (prefix /v1 must not strip from /v1beta/test)
        assert_eq!(
            build_target_url("https://api.openai.com/v1", "/v1", "/v1beta/test", ""),
            "https://api.openai.com/v1/v1beta/test"
        );
    }

    #[test]
    fn test_prefix_matches_segment_boundary() {
        use gateway_filter_lib::proxy::routing::prefix_matches;

        // Root prefix matches everything
        assert!(prefix_matches("/", "/"));
        assert!(prefix_matches("/anything", "/"));
        assert!(prefix_matches("/anything", ""));

        // Exact match
        assert!(prefix_matches("/v1", "/v1"));

        // Segment boundary match
        assert!(prefix_matches("/v1/chat/completions", "/v1"));
        assert!(prefix_matches("/v1/models", "/v1"));

        // Non-segment boundaries must NOT match
        assert!(!prefix_matches("/v1beta/chat", "/v1"));
        assert!(!prefix_matches("/v10/chat", "/v1"));
        assert!(!prefix_matches("/v1_extra", "/v1"));
    }

    #[test]
    fn test_key_failure_threshold_burns_only_at_limit() {
        use gateway_filter_lib::proxy::rotation::{
            handle_error_session_rotation, handle_success_reset,
        };
        use gateway_filter_lib::proxy::{AppState, GatewayConfig};

        // CÔ LẬP TEST (bài học xương máu): handle_*_rotation gọi save_to_disk() →
        // PHẢI trỏ GATEWAY_FILTER_CONFIG_DIR sang thư mục tạm, nếu không test sẽ ghi đè
        // ~/.config/gateway_filter/gateway_filter_config.json THẬT của user. Test này là test duy nhất
        // trong suite chạm đường save, nên set một lần ở đây là an toàn.
        let _guard = CONFIG_DIR_LOCK.lock().unwrap();
        let iso_dir = std::env::temp_dir()
            .join(format!("gateway_filter_test_cfg_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&iso_dir).unwrap();
        unsafe { std::env::set_var("GATEWAY_FILTER_CONFIG_DIR", &iso_dir) };

        // Counter thuần: đếm tăng dần, success reset về 0
        let mut km = EndpointKeyManager::default();
        assert_eq!(km.record_key_failure("sk-dead"), 1);
        assert_eq!(km.record_key_failure("sk-dead"), 2);
        km.record_key_success("sk-dead");
        assert_eq!(km.record_key_failure("sk-dead"), 1);

        // End-to-end qua rotation với threshold = 3: 2 lỗi đầu giữ key, lỗi thứ 3 đốt
        let temp_dir = std::env::temp_dir();
        let key_file = temp_dir.join(format!("test_threshold_keys_{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&key_file, "key-only-1\n").unwrap();

        let mut cfg = GatewayConfig::default();
        cfg.max_key_failures = 3;
        cfg.routes.push(gateway_filter_lib::proxy::RouteRule {
            id: "route-th".to_string(),
            name: "Route TH".to_string(),
            port: 3999,
            path_prefix: "/v1".to_string(),
            target_base_url: "https://example.com/v1".to_string(),
            tunnel_id: "direct_bypass".to_string(),
            enabled: true,
            status: gateway_filter_lib::proxy::RouteStatus::Active,
            last_error: None,
            key_manager: EndpointKeyManager {
                key_file_path: Some(key_file.to_str().unwrap().to_string()),
                ..Default::default()
            },
            custom_auth_token: None,
            min_request_interval_ms: 0,
            fingerprint_index: None,
            protocol_adapter: gateway_filter_lib::config::ProtocolAdapter::None,
        });
        let state = std::sync::Arc::new(AppState::new(cfg));

        for _ in 0..2 {
            handle_error_session_rotation(&state, "route-th", 401, Some("key-only-1"), true);
            let content = std::fs::read_to_string(&key_file).unwrap();
            assert!(content.contains("key-only-1"), "key phải sống sót trước ngưỡng");
        }
        handle_error_session_rotation(&state, "route-th", 401, Some("key-only-1"), true);
        let content = std::fs::read_to_string(&key_file).unwrap();
        assert!(!content.contains("key-only-1"), "chạm ngưỡng 3 phải đốt key");

        // Token cấu hình tay (không qua file) không bao giờ bị đốt
        handle_error_session_rotation(&state, "route-th", 401, Some("custom-token"), false);
        handle_success_reset(&state, "route-th", 200, Some("key-only-1"), true);

        // Mọi save của rotation phải rơi vào thư mục cô lập, KHÔNG phải config thật
        assert!(
            iso_dir.join("gateway_filter_config.json").exists(),
            "rotation save phải vào thư mục cô lập"
        );

        let _ = std::fs::remove_file(&key_file);
        let _ = std::fs::remove_dir_all(&iso_dir);
    }

    #[test]
    fn test_malformed_json_key_file_fails_closed() {
        // L1: file .json hỏng phải Err (fail-closed), không được rơi sang parse TXT
        // thành token rác gửi upstream.
        let temp_dir = std::env::temp_dir();
        let bad_json = temp_dir.join(format!("test_bad_keys_{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&bad_json, r#"{"not": "an array"}"#).unwrap();
        let res = EndpointKeyManager::load_keys_from_file(bad_json.to_str().unwrap());
        assert!(res.is_err(), "JSON hỏng phải báo lỗi, không parse TXT fallback");

        // JSON hợp lệ vẫn đọc bình thường
        let good_json = temp_dir.join(format!("test_good_keys_{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&good_json, r#"["sk-aaa", "sk-bbb"]"#).unwrap();
        let keys = EndpointKeyManager::load_keys_from_file(good_json.to_str().unwrap()).unwrap();
        assert_eq!(keys, vec!["sk-aaa".to_string(), "sk-bbb".to_string()]);

        let _ = std::fs::remove_file(&bad_json);
        let _ = std::fs::remove_file(&good_json);
    }

    #[test]
    fn test_unicode_key_preview_never_panics() {
        // L2: key chứa UTF-8 không được panic do byte-slice chẻ đôi char.
        let emoji_key = "🔑🔑🔑🔑🔑🔑secret-key-123456";
        let preview = EndpointKeyManager::preview_key(emoji_key);
        assert!(preview.contains("..."));
        assert_eq!(preview.chars().count(), 6 + 3 + 4);

        // Key ASCII ngắn giữ nguyên
        assert_eq!(EndpointKeyManager::preview_key("short"), "short");
        // Key ASCII dài vẫn đúng format cũ
        assert_eq!(
            EndpointKeyManager::preview_key("sk-1234567890abcdef"),
            "sk-123...cdef"
        );
    }

    #[test]
    fn test_mask_local_paths_only_when_matched() {
        use gateway_filter_lib::fingerprint::patterns::mask_local_paths;

        // Body sạch → None (caller giữ nguyên bytes gốc)
        assert_eq!(mask_local_paths(r#"{"model":"gpt-4o"}"#), None);

        // Path Linux bị che
        let masked = mask_local_paths("read /home/alice/secret.txt please").unwrap();
        assert!(masked.contains("[REDACTED_PATH]"));
        assert!(!masked.contains("/home/alice"));

        // Path Windows bị che
        let masked_win = mask_local_paths(r#"open C:\Users\bob\doc.txt"#).unwrap();
        assert!(masked_win.contains("[REDACTED_PATH]"));

        // Path macOS bị che
        let masked_mac = mask_local_paths("file in /Users/bimatkeo/data.json here").unwrap();
        assert!(masked_mac.contains("[REDACTED_PATH]"));
        assert!(!masked_mac.contains("/Users/bimatkeo"));

        // Path Windows JSON-escaped bị che
        let masked_win_json = mask_local_paths(r#"{"path":"C:\\Users\\Admin\\project\\code.rs"}"#).unwrap();
        assert!(masked_win_json.contains("[REDACTED_PATH]"));
    }

    #[test]
    fn test_reset_runtime_health_no_stale_boot() {
        use gateway_filter_lib::config::{reset_runtime_health, GatewayConfig, RouteStatus};

        // Dựng config giả lập "tắt app lúc đang Online": status/IP/error/check-time cũ
        let mut cfg = GatewayConfig::default();
        cfg.tunnels.push(gateway_filter_lib::vpn::OutboundTunnel {
            id: "t1".to_string(),
            name: "T1".to_string(),
            protocol: gateway_filter_lib::vpn::TunnelProtocol::Socks5,
            endpoint: "127.0.0.1:1080".to_string(),
            enabled: true,
            status: gateway_filter_lib::vpn::TunnelStatus::Online,
            max_concurrent_streams: 0,
            min_request_interval_ms: 0,
            start_command: Some("vpn-cli connect".to_string()),
            stop_command: None,
            auth_user: None,
            auth_pass: None,
            last_checked_at: Some("2026-01-01 00:00:00".to_string()),
            last_error: None,
            last_exit_ip: Some("212.0.0.1".to_string()),
            last_latency_ms: Some(42),
            tags: vec![],
            adguard_location: None,
        });
        cfg.tunnels.push(gateway_filter_lib::vpn::OutboundTunnel {
            id: "t2".to_string(),
            name: "T2".to_string(),
            protocol: gateway_filter_lib::vpn::TunnelProtocol::Direct,
            endpoint: String::new(),
            enabled: false,
            status: gateway_filter_lib::vpn::TunnelStatus::Online,
            max_concurrent_streams: 0,
            min_request_interval_ms: 0,
            start_command: None,
            stop_command: None,
            auth_user: None,
            auth_pass: None,
            last_checked_at: Some("2026-01-01 00:00:00".to_string()),
            last_error: Some("old error".to_string()),
            last_exit_ip: Some("1.2.3.4".to_string()),
            last_latency_ms: Some(7),
            tags: vec![],
            adguard_location: None,
        });

        reset_runtime_health(&mut cfg);

        // Tunnel enabled → Unknown chờ worker, disabled → Offline; sạch IP/latency/error/time
        assert_eq!(cfg.tunnels[0].status, gateway_filter_lib::vpn::TunnelStatus::Unknown);
        assert_eq!(cfg.tunnels[1].status, gateway_filter_lib::vpn::TunnelStatus::Offline);
        for t in &cfg.tunnels {
            assert!(t.last_exit_ip.is_none());
            assert!(t.last_latency_ms.is_none());
            assert!(t.last_error.is_none());
            assert!(t.last_checked_at.is_none());
        }
        // Route để status cố tình khác Unknown trước reset
        cfg.routes.push(gateway_filter_lib::proxy::RouteRule {
            id: "r1".to_string(),
            name: "R1".to_string(),
            port: 3000,
            path_prefix: "/v1".to_string(),
            target_base_url: "https://example.com/v1".to_string(),
            tunnel_id: "t1".to_string(),
            enabled: true,
            status: gateway_filter_lib::proxy::RouteStatus::Active,
            last_error: Some("bind failed".to_string()),
            key_manager: Default::default(),
            custom_auth_token: None,
            min_request_interval_ms: 0,
            fingerprint_index: None,
            protocol_adapter: gateway_filter_lib::config::ProtocolAdapter::None,
        });
        reset_runtime_health(&mut cfg);
        assert_eq!(cfg.routes[0].status, RouteStatus::Unknown);
        assert!(cfg.routes[0].last_error.is_none());
    }

    #[test]
    fn test_unknown_cli_tunnel_not_routed() {
        use gateway_filter_lib::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol, TunnelStatus};

        fn mk_cli(id: &str, status: TunnelStatus) -> OutboundTunnel {
            OutboundTunnel {
                id: id.to_string(),
                name: id.to_string(),
                protocol: TunnelProtocol::Socks5,
                endpoint: "127.0.0.1:1080".to_string(),
                enabled: true,
                status,
                max_concurrent_streams: 0,
                min_request_interval_ms: 0,
                start_command: Some("vpn-cli connect".to_string()),
                stop_command: None,
                auth_user: None,
                auth_pass: None,
                last_checked_at: None,
                last_error: None,
                last_exit_ip: None,
                last_latency_ms: None,
                tags: vec![],
                adguard_location: None,
            }
        }

        // Tunnel CLI Unknown (chưa từng Online) không được nhận traffic, kể cả khi
        // được gán trực tiếp → None để caller báo 503 trung thực thay vì thử rồi 502
        let pool = vec![mk_cli("never_started", TunnelStatus::Unknown)];
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "never_started"),
            None
        );

        // CLI Unknown bị bỏ qua khi failover, nhường Online
        let pool2 = vec![
            mk_cli("dead", TunnelStatus::Offline),
            mk_cli("fresh", TunnelStatus::Unknown),
            mk_cli("good", TunnelStatus::Online),
        ];
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool2, "dead"),
            Some("good".to_string())
        );

        // Direct Unknown (không cần process) vẫn dùng được như cũ
        let mut direct = mk_cli("d", TunnelStatus::Unknown);
        direct.protocol = TunnelProtocol::Direct;
        direct.start_command = None;
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&[direct], "d"),
            Some("d".to_string())
        );
    }

    #[test]
    fn test_pacing_slot_gcra() {
        use std::time::Duration;
        use gateway_filter_lib::proxy::compute_pacing_slot;

        let now = tokio::time::Instant::now();
        let interval = Duration::from_millis(400);

        // Chưa có lịch sử → không chờ, target = now
        let (wait, target) = compute_pacing_slot(None, interval, now);
        assert!(wait.is_zero());
        assert_eq!(target, now);

        // Lần cuối đã lâu → không chờ
        let (wait, _) =
            compute_pacing_slot(Some(now - Duration::from_secs(10)), interval, now);
        assert!(wait.is_zero());

        // Vừa gửi xong → chờ đủ interval, target = now + interval
        let (wait, target) = compute_pacing_slot(Some(now), interval, now);
        assert_eq!(wait, interval);
        assert_eq!(target, now + interval);

        // Burst dồn dập (last đã ở tương lai now+350ms) → chờ 750ms,
        // giữ khoảng cách đều, không underflow
        let (wait, target) = compute_pacing_slot(
            Some(now + Duration::from_millis(350)),
            interval,
            now,
        );
        assert_eq!(wait, Duration::from_millis(750));
        assert_eq!(target, now + Duration::from_millis(750));
    }

    #[test]
    fn test_pacing_two_levels_slowest_wins() {
        use std::collections::HashMap;
        use std::time::Duration;
        use gateway_filter_lib::proxy::reserve_pacing_slot;

        let now = tokio::time::Instant::now();
        let mut slots = HashMap::new();

        // Cả 2 tắt → không chờ, không tạo slot
        assert!(reserve_pacing_slot(&mut slots, "r1", "t1", 0, 0, now).is_zero());
        assert!(slots.is_empty());

        // Route 400ms: lần đầu qua, lần 2 ngay sau chờ đủ 400
        assert!(reserve_pacing_slot(&mut slots, "r1", "t1", 400, 0, now).is_zero());
        assert_eq!(
            reserve_pacing_slot(&mut slots, "r1", "t1", 400, 0, now),
            Duration::from_millis(400)
        );

        // Tunnel aggregate: route KHÁC chung tunnel vẫn phải chờ theo nhịp tunnel
        let mut slots2 = HashMap::new();
        assert!(reserve_pacing_slot(&mut slots2, "ra", "t9", 0, 400, now).is_zero());
        assert_eq!(
            reserve_pacing_slot(&mut slots2, "rb", "t9", 0, 400, now),
            Duration::from_millis(400)
        );

        // Chậm nhất thắng: route 100 + tunnel 400 → chờ 400
        let mut slots3 = HashMap::new();
        assert!(reserve_pacing_slot(&mut slots3, "r1", "t1", 100, 400, now).is_zero());
        assert_eq!(
            reserve_pacing_slot(&mut slots3, "r1", "t1", 100, 400, now),
            Duration::from_millis(400)
        );
    }

    #[test]
    fn test_truncate_log_body() {
        use gateway_filter_lib::proxy::truncate_log_body;

        let small = "hello world";
        assert_eq!(truncate_log_body(small, 2048), "hello world");

        let large = "a".repeat(3000);
        let truncated = truncate_log_body(&large, 2048);
        // 3000 chars, giữ 2048 → cắt 952 chars (báo đúng số chars, không phải bytes)
        assert!(truncated.contains("... [Truncated 952 chars]"));
        assert_eq!(truncated.chars().take(2048).count(), 2048);
    }

    #[test]
    fn test_generate_key_file() {
        let temp_keys_dir = std::env::temp_dir().join("gateway_filter_test_keys");
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
        use gateway_filter_lib::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol};

        // Test một cổng TCP không tồn tại (port 54321 trên localhost)
        let fake_tunnel = OutboundTunnel {
            id: "fake_dead_tunnel".to_string(),
            name: "Dead SOCKS Proxy".to_string(),
            protocol: TunnelProtocol::Socks5,
            endpoint: "127.0.0.1:54321".to_string(),
            enabled: true,
            status: gateway_filter_lib::vpn::TunnelStatus::Unknown,
            max_concurrent_streams: 0,
            min_request_interval_ms: 0,
            start_command: None,
            stop_command: None,
            auth_user: None,
            auth_pass: None,
            last_checked_at: None,
            last_error: None,
            last_exit_ip: None,
            last_latency_ms: None,
            tags: vec![],
            adguard_location: None,
        };

        let result = TunnelManager::test_tunnel(&fake_tunnel).await;
        assert!(!result.success);
        assert!(result.error.is_some());
        let err_text = result.error.unwrap();
        assert!(err_text.contains("Proxy unreachable"));
    }

    #[test]
    fn test_build_target_url_overlap_failsafe() {
        use gateway_filter_lib::proxy::build_target_url;

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
        use gateway_filter_lib::monitor::{append_disk_log, rotate_disk_log_if_needed, RawTrafficLog};

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
            raw_forwarded_body: String::new(),
            leaked_findings: vec![],
        };

        // Ghi 60 logs với max_disk_entries = 50
        for i in 1..=60 {
            append_disk_log(&log_file, &make_log(i), 50).unwrap();
        }

        // Rotation check trong append_disk_log chạy theo chu kỳ (không mỗi lần ghi),
        // nên gọi trực tiếp để kiểm tra logic xoay vòng một cách xác định
        rotate_disk_log_if_needed(&log_file, 50).unwrap();

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
        use gateway_filter_lib::proxy::format_logged_body;

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
            raw_forwarded_body: String::new(),
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
        use gateway_filter_lib::fingerprint::{FingerprintAnalyzer, FingerprintProfile};

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
        use gateway_filter_lib::fingerprint::FingerprintProfile;
        use gateway_filter_lib::proxy::key_manager::EndpointKeyManager;
        use gateway_filter_lib::proxy::{GatewayConfig, RouteRule};
        use gateway_filter_lib::vpn::{OutboundTunnel, TunnelProtocol, TunnelStatus};

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
                min_request_interval_ms: 0,
                    start_command: None,
                    stop_command: None,
                    auth_user: None,
                    auth_pass: None,
                    last_checked_at: None,
                    last_error: None,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec![],
                    adguard_location: None,
                },
                OutboundTunnel {
                    id: "tunnel-b".to_string(),
                    name: "Tunnel B".to_string(),
                    protocol: TunnelProtocol::Direct,
                    endpoint: "".to_string(),
                    enabled: true,
                    status: TunnelStatus::Online,
                    max_concurrent_streams: 0,
                min_request_interval_ms: 0,
                    start_command: None,
                    stop_command: None,
                    auth_user: None,
                    auth_pass: None,
                    last_checked_at: None,
                    last_error: None,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec![],
                    adguard_location: None,
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
                status: gateway_filter_lib::proxy::RouteStatus::Active,
                last_error: None,
                key_manager: km,
                custom_auth_token: None,
                min_request_interval_ms: 0,
                fingerprint_index: None,
                protocol_adapter: gateway_filter_lib::config::ProtocolAdapter::None,
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
            max_key_failures: 3,
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

    #[tokio::test]
    async fn test_run_tunnel_command() {
        use gateway_filter_lib::vpn::TunnelManager;
        let out = TunnelManager::run_tunnel_command_timeout("echo hello_vpn_manager", 10)
            .await
            .unwrap();
        assert!(out.contains("hello_vpn_manager"));
    }

    #[test]
    fn test_endpoint_addr_parsing() {
        use gateway_filter_lib::vpn::TunnelManager;
        assert_eq!(
            TunnelManager::endpoint_addr("127.0.0.1:1090"),
            Some("127.0.0.1:1090".parse().unwrap())
        );
        assert_eq!(
            TunnelManager::endpoint_addr("socks5h://127.0.0.1:1080"),
            Some("127.0.0.1:1080".parse().unwrap())
        );
        assert_eq!(
            TunnelManager::endpoint_addr("http://127.0.0.1:3128/path"),
            Some("127.0.0.1:3128".parse().unwrap())
        );
        assert_eq!(TunnelManager::endpoint_addr(""), None);
        // Hostname resolve được (localhost) → Some để pre-flight check hoạt động
        assert!(TunnelManager::endpoint_addr("localhost:9999").is_some());
        // Hostname không resolve được → None (bỏ qua check, không crash)
        assert_eq!(TunnelManager::endpoint_addr("nonexistent.invalid:9999"), None);
    }

    #[test]
    fn test_skip_key_bug() {
        use gateway_filter_lib::proxy::key_manager::EndpointKeyManager;
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("test_skip_key_{}.txt", uuid::Uuid::new_v4()));
        let file_str = file_path.to_str().unwrap().to_string();

        // Chuẩn bị 3 key theo thứ tự: key1, key2, key3
        std::fs::write(&file_path, "sk-key1\nsk-key2\nsk-key3\n").unwrap();

        let mut km = EndpointKeyManager {
            key_file_path: Some(file_str.clone()),
            current_key_index: 0,
            ..Default::default()
        };
        km.refresh_metadata();
        assert_eq!(km.total_keys, 3);
        assert_eq!(km.current_key_preview.as_deref(), Some("sk-key1"));

        // Giả lập key1 bị 401 Unauthorized và bị gỡ khỏi file
        let removed = km.remove_key_from_main_file("sk-key1").unwrap();
        assert!(removed);
        assert_eq!(km.total_keys, 2);

        // Con trỏ current_key_index phải vẫn giữ nguyên là 0 vì key2 đã trượt lên vị trí 0
        assert_eq!(km.current_key_index, 0);
        assert_eq!(km.current_key_preview.as_deref(), Some("sk-key2"));

        // Khi request tiếp theo sử dụng, key2 được chọn đúng mà KHÔNG bị skip sang key3!
        let current_key = km.get_active_key();
        assert_eq!(current_key.as_deref(), Some("sk-key2"));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_select_healthy_tunnel_failover() {
        use gateway_filter_lib::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol, TunnelStatus};

        fn mk(id: &str, enabled: bool, status: TunnelStatus) -> OutboundTunnel {
            OutboundTunnel {
                id: id.to_string(),
                name: id.to_string(),
                protocol: TunnelProtocol::Socks5,
                endpoint: "127.0.0.1:1080".to_string(),
                enabled,
                status,
                max_concurrent_streams: 0,
                min_request_interval_ms: 0,
                start_command: None,
                stop_command: None,
                auth_user: None,
                auth_pass: None,
                last_checked_at: None,
                last_error: None,
                last_exit_ip: None,
                last_latency_ms: None,
                tags: vec![],
                adguard_location: None,
            }
        }

        let pool = vec![
            mk("assigned", true, TunnelStatus::Online),
            mk("backup_online", true, TunnelStatus::Online),
            mk("backup_unknown", true, TunnelStatus::Unknown),
            mk("dead", true, TunnelStatus::Offline),
            mk("paused", false, TunnelStatus::Online),
        ];

        // Tunnel gán còn khỏe → giữ nguyên
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "assigned"),
            Some("assigned".to_string())
        );
        // direct_bypass luôn đi thẳng
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "direct_bypass"),
            Some("direct_bypass".to_string())
        );
        // Tunnel gán bị tắt → failover sang Online (không chọn Unknown/Offline/paused)
        // (max_by_key trả Online đứng sau cùng khi hòa điểm → backup_online, vẫn đúng)
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "paused"),
            Some("backup_online".to_string())
        );
        // Tunnel gán rớt mạng → failover
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "dead"),
            Some("backup_online".to_string())
        );
        // Tunnel gán không tồn tại → failover thay vì 503 chết đứng
        assert_eq!(
            TunnelManager::select_healthy_tunnel_id(&pool, "ghost_id"),
            Some("backup_online".to_string())
        );
        // Tất cả đều chết/tắt → None (caller báo 503)
        let all_bad = vec![
            mk("a", false, TunnelStatus::Online),
            mk("b", true, TunnelStatus::Offline),
        ];
        assert_eq!(TunnelManager::select_healthy_tunnel_id(&all_bad, "a"), None);
        assert_eq!(TunnelManager::select_healthy_tunnel_id(&[], "a"), None);
    }

    #[tokio::test]
    async fn test_run_tunnel_command_timeout() {
        use gateway_filter_lib::vpn::TunnelManager;

        // Lệnh nhanh → Ok bình thường
        let ok = TunnelManager::run_tunnel_command_timeout("echo hello_timeout", 5)
            .await
            .unwrap();
        assert!(ok.contains("hello_timeout"));

        // Lệnh rỗng → Err ngay
        assert!(TunnelManager::run_tunnel_command_timeout("   ", 5)
            .await
            .is_err());
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn test_run_tunnel_failure_includes_stdout_and_login_hint() {
        use gateway_filter_lib::vpn::TunnelManager;

        // Mô phỏng adguard: exit != 0, lý do nằm ở STDOUT, stderr rỗng.
        // Trước fix, error log trống rỗng ("exit status: 1: ").
        let err = TunnelManager::run_tunnel_command_timeout(
            "echo 'Please log in to connect'; exit 11",
            5,
        )
        .await
        .unwrap_err();
        assert!(err.contains("Please log in to connect"), "mất stdout: {}", err);
        assert!(err.contains("Chưa đăng nhập"), "mất hint login: {}", err);

        // Lỗi thuần stderr vẫn hiện như cũ
        let err2 = TunnelManager::run_tunnel_command_timeout(
            "echo boom >&2; exit 3",
            5,
        )
        .await
        .unwrap_err();
        assert!(err2.contains("boom"), "mất stderr: {}", err2);
    }

    #[test]
    fn test_get_route_fingerprint_per_endpoint() {
        use gateway_filter_lib::fingerprint::FingerprintProfile;
        use gateway_filter_lib::proxy::{GatewayConfig, RouteRule};

        fn mk_route(fp: Option<usize>) -> RouteRule {
            RouteRule {
                id: "r".to_string(),
                name: "R".to_string(),
                port: 3000,
                path_prefix: "/v1".to_string(),
                target_base_url: "https://example.com/v1".to_string(),
                tunnel_id: "t".to_string(),
                enabled: true,
                status: gateway_filter_lib::proxy::RouteStatus::Active,
                last_error: None,
                key_manager: Default::default(),
                custom_auth_token: None,
                min_request_interval_ms: 0,
                fingerprint_index: fp,
                protocol_adapter: gateway_filter_lib::config::ProtocolAdapter::None,
            }
        }

        let mut cfg = GatewayConfig::default();
        // Pool rỗng → mọi route fallback về global default
        assert_eq!(
            cfg.get_route_fingerprint(&mk_route(Some(0))).mode,
            cfg.fingerprint_profile.mode
        );
        assert_eq!(
            cfg.get_route_fingerprint(&mk_route(None)).mode,
            cfg.fingerprint_profile.mode
        );

        cfg.fingerprint_pool = vec![
            FingerprintProfile { mode: "fp-a".to_string(), ..Default::default() },
            FingerprintProfile { mode: "fp-b".to_string(), ..Default::default() },
        ];
        cfg.active_fingerprint_index = 1;
        // Route có index → pool[i], bất chấp global active
        assert_eq!(cfg.get_route_fingerprint(&mk_route(Some(0))).mode, "fp-a");
        assert_eq!(cfg.get_route_fingerprint(&mk_route(Some(1))).mode, "fp-b");
        // None → global active (index 1 → fp-b)
        assert_eq!(cfg.get_route_fingerprint(&mk_route(None)).mode, "fp-b");
        // Index lệch (sau khi xóa profile) → modulo, không panic
        assert_eq!(cfg.get_route_fingerprint(&mk_route(Some(5))).mode, "fp-b");

        // clamp đưa index lệch về tầm hợp lệ
        cfg.routes.push(mk_route(Some(7)));
        cfg.clamp_fingerprint_indices();
        assert_eq!(cfg.routes[0].fingerprint_index, Some(1));

        // Serde tương thích config cũ (thiếu field → None)
        let legacy: RouteRule = serde_json::from_str(
            r#"{"id":"x","name":"X","port":3000,"path_prefix":"/v1","target_base_url":"https://e.com","tunnel_id":"t","enabled":true,"key_manager":{"current_key_index":0,"total_keys":0},"custom_auth_token":null,"min_request_interval_ms":0}"#,
        )
        .unwrap();
        assert_eq!(legacy.fingerprint_index, None);
    }

    #[test]
    fn test_rotation_per_route_fingerprint() {
        use gateway_filter_lib::fingerprint::FingerprintProfile;
        use gateway_filter_lib::proxy::key_manager::EndpointKeyManager;
        use gateway_filter_lib::proxy::rotation::handle_error_session_rotation;
        use gateway_filter_lib::proxy::{AppState, GatewayConfig, RouteRule};
        use gateway_filter_lib::vpn::{OutboundTunnel, TunnelProtocol, TunnelStatus};

        // Cô lập save_to_disk như test threshold (không ghi đè config thật)
        let _guard = CONFIG_DIR_LOCK.lock().unwrap();
        let iso_dir = std::env::temp_dir()
            .join(format!("gateway_filter_test_fp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&iso_dir).unwrap();
        unsafe { std::env::set_var("GATEWAY_FILTER_CONFIG_DIR", &iso_dir) };

        fn mk_tunnel(id: &str) -> OutboundTunnel {
            OutboundTunnel {
                id: id.to_string(),
                name: id.to_string(),
                protocol: TunnelProtocol::Direct,
                endpoint: String::new(),
                enabled: true,
                status: TunnelStatus::Online,
                max_concurrent_streams: 0,
                min_request_interval_ms: 0,
                start_command: None,
                stop_command: None,
                auth_user: None,
                auth_pass: None,
                last_checked_at: None,
                last_error: None,
                last_exit_ip: None,
                last_latency_ms: None,
                tags: vec![],
                adguard_location: None,
            }
        }
        fn mk_route(id: &str, fp: Option<usize>) -> RouteRule {
            RouteRule {
                id: id.to_string(),
                name: id.to_string(),
                port: 3000,
                path_prefix: "/v1".to_string(),
                target_base_url: "https://example.com/v1".to_string(),
                tunnel_id: "direct_bypass".to_string(),
                enabled: true,
                status: gateway_filter_lib::proxy::RouteStatus::Active,
                last_error: None,
                key_manager: EndpointKeyManager::default(),
                custom_auth_token: None,
                min_request_interval_ms: 0,
                fingerprint_index: fp,
                protocol_adapter: gateway_filter_lib::config::ProtocolAdapter::None,
            }
        }
        fn mk_cfg() -> GatewayConfig {
            GatewayConfig {
                config_version: 1,
                tunnels: vec![mk_tunnel("direct_bypass")],
                routes: vec![mk_route("r-per", Some(0)), mk_route("r-global", None)],
                fingerprint_profile: FingerprintProfile::default(),
                fingerprint_pool: vec![
                    FingerprintProfile { mode: "fp-0".to_string(), ..Default::default() },
                    FingerprintProfile { mode: "fp-1".to_string(), ..Default::default() },
                    FingerprintProfile { mode: "fp-2".to_string(), ..Default::default() },
                ],
                active_fingerprint_index: 0,
                max_log_entries: 100,
                max_disk_log_entries: 1000,
                max_key_failures: 3,
            }
        }

        // Route có index → chỉ advance per-route, global đứng yên
        let state = std::sync::Arc::new(AppState::new(mk_cfg()));
        handle_error_session_rotation(&state, "r-per", 401, None, false);
        {
            let conf = state.config.read();
            let per = conf.routes.iter().find(|r| r.id == "r-per").unwrap();
            assert_eq!(per.fingerprint_index, Some(1));
            assert_eq!(conf.active_fingerprint_index, 0, "global phải đứng yên");
            assert_eq!(conf.get_route_fingerprint(per).mode, "fp-1");
        }

        // Route None → advance global như cũ, per-route đứng yên
        handle_error_session_rotation(&state, "r-global", 403, None, false);
        {
            let conf = state.config.read();
            let per = conf.routes.iter().find(|r| r.id == "r-per").unwrap();
            assert_eq!(per.fingerprint_index, Some(1), "per-route không bị lây");
            assert_eq!(conf.active_fingerprint_index, 1);
        }

        // Non-401/403 → no-op toàn bộ
        handle_error_session_rotation(&state, "r-per", 500, None, false);
        {
            let conf = state.config.read();
            assert_eq!(
                conf.routes.iter().find(|r| r.id == "r-per").unwrap().fingerprint_index,
                Some(1)
            );
            assert_eq!(conf.active_fingerprint_index, 1);
        }

        let _ = std::fs::remove_dir_all(&iso_dir);
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn test_run_tunnel_command_timeout_kills_hang() {        use gateway_filter_lib::vpn::TunnelManager;
        use std::time::Instant;

        // Lệnh treo (sleep 30) với timeout 1s → phải Err timeout, không treo test
        let start = Instant::now();
        let err = TunnelManager::run_tunnel_command_timeout("sleep 30", 1)
            .await
            .unwrap_err();
        assert!(err.contains("timed out"), "unexpected error: {}", err);
        assert!(start.elapsed().as_secs() < 10, "timeout did not fire promptly");
    }

    #[test]
    fn test_onemin_adapter_full_flow() {
        use gateway_filter_lib::proxy::adapters::onemin::*;
        use serde_json::json;

        // 1. Target URL checking
        assert!(is_1min_target("https://api.1min.ai"));
        assert!(is_1min_target("https://api.1min.ai/api/features"));
        assert!(!is_1min_target("https://api.openai.com/v1"));

        // 2. Models resolution
        assert_eq!(resolve_model_id("gpt-4o"), "gpt-4o");
        assert_eq!(resolve_model_id("claude-3-5-sonnet"), "us.anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert_eq!(resolve_model_id("gemini-2.5-flash"), "gemini-2.5-flash");
        assert_eq!(resolve_model_id("deepseek-chat"), "deepseek-v4-pro");
        assert_eq!(resolve_model_id("qwen3.7-max"), "qwen3.7-max");

        // 3. Low-temp translation request -> CONTENT_TRANSLATOR
        let trans_req = json!({
            "model": "gpt-4o",
            "temperature": 0.1,
            "messages": [
                {"role": "system", "content": "You are a professional translator into Vietnamese."},
                {"role": "user", "content": "The quick brown fox jumps over the lazy dog."}
            ]
        });
        let trans_bytes = serde_json::to_vec(&trans_req).unwrap();
        let (url, transformed, stream, model) = transform_request("https://api.1min.ai/v1/chat/completions", &trans_bytes).unwrap();
        assert_eq!(model, "gpt-4o");
        assert!(!stream);
        assert!(url.contains("/api/features"));
        let val: serde_json::Value = serde_json::from_slice(&transformed).unwrap();
        assert_eq!(val["type"], "CONTENT_TRANSLATOR");
        assert_eq!(val["promptObject"]["tone"], "clinical");
        assert_eq!(val["promptObject"]["writingStyle"], "Academic");
        assert_eq!(val["promptObject"]["targetLanguage"], "vi");

        // 4. Chat request -> UNIFY_CHAT_WITH_AI
        let chat_req = json!({
            "model": "claude-3-5-sonnet",
            "temperature": 0.7,
            "stream": true,
            "messages": [
                {"role": "user", "content": "Write a short poem about coding."}
            ]
        });
        let chat_bytes = serde_json::to_vec(&chat_req).unwrap();
        let (chat_url, chat_transformed, chat_stream, chat_model) = transform_request("https://api.1min.ai/v1/chat/completions", &chat_bytes).unwrap();
        assert_eq!(chat_model, "claude-3-5-sonnet");
        assert!(chat_stream);
        assert!(chat_url.contains("/api/chat-with-ai?isStreaming=true"));
        let chat_val: serde_json::Value = serde_json::from_slice(&chat_transformed).unwrap();
        assert_eq!(chat_val["type"], "UNIFY_CHAT_WITH_AI");
        assert_eq!(chat_val["model"], "us.anthropic.claude-3-5-sonnet-20241022-v2:0");

        // 5. Non-streaming Response transformation
        let one_min_resp = json!({
            "aiRecord": {
                "uuid": "record-1234-abcd",
                "aiRecordDetail": {
                    "resultObject": ["Con cáo nâu nhanh nhẹn nhảy qua con chó lười biếng."]
                }
            }
        });
        let one_min_resp_bytes = serde_json::to_vec(&one_min_resp).unwrap();
        let openai_resp_bytes = transform_response_json(&one_min_resp_bytes, "gpt-4o").unwrap();
        let openai_val: serde_json::Value = serde_json::from_slice(&openai_resp_bytes).unwrap();
        assert_eq!(openai_val["id"], "chatcmpl-record-1234-abcd");
        assert_eq!(openai_val["choices"][0]["message"]["content"], "Con cáo nâu nhanh nhẹn nhảy qua con chó lười biếng.");

        // 6. SSE Stream Transformer
        let mut transformer = OneMinSseTransformer::new("gpt-4o");
        let chunk = b"event: content\ndata: {\"content\": \"Xin ch\xC3\xA0o\"}\n\nevent: done\ndata: {}\n\n";
        let out_bytes = transformer.feed_bytes(chunk);
        let out_str = String::from_utf8(out_bytes).unwrap();
        assert!(out_str.contains("data: "));
        assert!(out_str.contains("chat.completion.chunk"));
        assert!(out_str.contains("\"content\":\"Xin ch\\u00e0o\"") || out_str.contains("\"content\":\"Xin chào\""));
        assert!(out_str.contains("data: [DONE]"));
    }

    #[test]
    fn test_anthropic_adapter_full_flow() {
        use gateway_filter_lib::proxy::adapters::anthropic::{
            build_anthropic_target_url, resolve_model_id, transform_request,
            transform_response_json, AnthropicSseTransformer,
        };
        use serde_json::json;

        // 1. Resolve model ID
        assert_eq!(resolve_model_id("claude-3-5-sonnet"), "claude-3-5-sonnet-20241022");
        assert_eq!(resolve_model_id("claude-3-7-sonnet"), "claude-3-7-sonnet-20250219");
        assert_eq!(resolve_model_id("claude-3-5-haiku"), "claude-3-5-haiku-20241022");
        assert_eq!(resolve_model_id("gpt-4o"), "claude-3-5-sonnet-20241022");

        // 2. Build target URL
        let rewritten_url = build_anthropic_target_url("https://api.anthropic.com/v1/chat/completions");
        assert_eq!(rewritten_url, "https://api.anthropic.com/v1/messages");

        // 3. Transform request body: Extract system prompt & map parameters
        let openai_req = json!({
            "model": "claude-3-5-sonnet",
            "messages": [
                {"role": "system", "content": "You are a helpful coding assistant."},
                {"role": "user", "content": "Hello Claude!"}
            ],
            "temperature": 0.5,
            "stream": true
        });
        let req_bytes = serde_json::to_vec(&openai_req).unwrap();
        let (new_url, transformed_req, is_streaming, model_used) =
            transform_request("https://api.anthropic.com/v1/chat/completions", &req_bytes).unwrap();
        let anthropic_req: serde_json::Value = serde_json::from_slice(&transformed_req).unwrap();

        assert_eq!(new_url, "https://api.anthropic.com/v1/messages");
        assert_eq!(is_streaming, true);
        assert_eq!(model_used, "claude-3-5-sonnet");
        assert_eq!(anthropic_req["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(anthropic_req["system"], "You are a helpful coding assistant.");
        assert_eq!(anthropic_req["messages"].as_array().unwrap().len(), 1);
        assert_eq!(anthropic_req["messages"][0]["role"], "user");
        assert_eq!(anthropic_req["max_tokens"], 4096);
        assert_eq!(anthropic_req["stream"], true);

        // 4. Transform non-streaming response JSON
        let anthropic_resp = json!({
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I assist you with your coding today?"
                }
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 15,
                "output_tokens": 12
            }
        });
        let anthropic_resp_bytes = serde_json::to_vec(&anthropic_resp).unwrap();
        let openai_resp_bytes = transform_response_json(&anthropic_resp_bytes, "claude-3-5-sonnet").unwrap();
        let openai_val: serde_json::Value = serde_json::from_slice(&openai_resp_bytes).unwrap();
        assert_eq!(openai_val["id"], "chatcmpl-msg_01XFDUDYJgAACzvnptvVoYEL");
        assert_eq!(openai_val["choices"][0]["message"]["content"], "Hello! How can I assist you with your coding today?");
        assert_eq!(openai_val["choices"][0]["finish_reason"], "stop");

        // 5. SSE Stream Transformer
        let mut transformer = AnthropicSseTransformer::new("claude-3-5-sonnet");
        let chunk = b"event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello world\"}}\n\nevent: message_stop\ndata: {\"type\": \"message_stop\"}\n\n";
        let out_bytes = transformer.feed_bytes(chunk);
        let out_str = String::from_utf8(out_bytes).unwrap();
        assert!(out_str.contains("data: "));
        assert!(out_str.contains("chat.completion.chunk"));
        assert!(out_str.contains("\"content\":\"Hello world\""));
        assert!(out_str.contains("data: [DONE]"));
    }
}

