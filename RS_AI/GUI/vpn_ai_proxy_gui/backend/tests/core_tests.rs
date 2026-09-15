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
}
