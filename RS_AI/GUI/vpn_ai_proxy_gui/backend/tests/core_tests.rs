#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use vpn_ai_proxy_gui_lib::fingerprint::{FingerprintAnalyzer, FingerprintProfile};
    use vpn_ai_proxy_gui_lib::monitor::{RequestLog, RingBufferLog};
    use vpn_ai_proxy_gui_lib::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol};

    #[test]
    fn test_ring_buffer_overwrite() {
        let buffer = RingBufferLog::new(3);
        for i in 1..=5 {
            buffer.push(RequestLog {
                id: format!("req-{}", i),
                timestamp: "12:00:00".to_string(),
                method: "POST".to_string(),
                path: ":3000/v1".to_string(),
                target_url: "https://api.openai.com/v1".to_string(),
                status_code: 200,
                duration_ms: 120,
                leaked_findings: vec![],
                client_headers: vec![],
                forwarded_headers: vec![],
                prompt_preview: None,
                response_preview: None,
                is_streaming: false,
                bytes_sent: 100,
                bytes_received: 200,
            });
        }

        assert_eq!(buffer.count(), 3);
        let logs = buffer.get_all();
        assert_eq!(logs[0].id, "req-3");
        assert_eq!(logs[1].id, "req-4");
        assert_eq!(logs[2].id, "req-5");
    }

    #[test]
    fn test_fingerprint_leak_detection() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "OpenCode/1.0 (Linux x86_64; Ubuntu 24.04)".to_string());
        headers.insert("x-stainless-os".to_string(), "Linux".to_string());
        headers.insert("x-cursor-client-version".to_string(), "0.42.0".to_string());

        let body = r#"{"model":"gpt-4o","messages":[{"role":"user","content":"Read file /home/bimatkeo/Documents/secret.txt"}]}"#;

        let leaks = FingerprintAnalyzer::analyze_inbound(&headers, Some(body));

        let categories: Vec<String> = leaks.into_iter().map(|l| l.category).collect();
        assert!(categories.contains(&"os_info".to_string()));
        assert!(categories.contains(&"sdk_tracking".to_string()));
        assert!(categories.contains(&"device_id".to_string()));
        assert!(categories.contains(&"path_leak".to_string()));
    }

    #[test]
    fn test_header_sanitization_and_spoofing() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "BadClient/1.0".to_string());
        headers.insert("x-stainless-os".to_string(), "Linux".to_string());
        headers.insert("authorization".to_string(), "Bearer sk-123456".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let profile = FingerprintProfile {
            mode: "stealth".to_string(),
            custom_user_agent: Some("Custom-Anonymous-Engine/1.0".to_string()),
            strip_sdk_headers: true,
            strip_ide_headers: true,
            strip_sec_ch_ua: true,
            remove_empty_headers: true,
            mask_local_paths_in_body: true,
        };

        let clean = FingerprintAnalyzer::sanitize_headers(&headers, &profile);

        assert!(!clean.contains_key("x-stainless-os"));
        assert_eq!(clean.get("user-agent").unwrap(), "Custom-Anonymous-Engine/1.0");
        assert_eq!(clean.get("authorization").unwrap(), "Bearer sk-123456");
        assert_eq!(clean.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_multi_tunnel_client_builder() {
        let socks_tunnel = OutboundTunnel {
            id: "adguard".to_string(),
            name: "Adguard".to_string(),
            protocol: TunnelProtocol::Socks5,
            endpoint: "127.0.0.1:1080".to_string(),
            enabled: true,
            last_exit_ip: None,
            last_latency_ms: None,
            tags: vec![],
        };

        let direct_tunnel = OutboundTunnel {
            id: "direct".to_string(),
            name: "Direct".to_string(),
            protocol: TunnelProtocol::Direct,
            endpoint: "".to_string(),
            enabled: true,
            last_exit_ip: None,
            last_latency_ms: None,
            tags: vec![],
        };

        let _client1 = TunnelManager::build_client(&socks_tunnel);
        let _client2 = TunnelManager::build_client(&direct_tunnel);
    }
}
