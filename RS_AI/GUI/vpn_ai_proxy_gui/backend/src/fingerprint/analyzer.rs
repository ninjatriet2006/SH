use std::collections::HashMap;

use super::patterns::LOCAL_PATH_REGEX;
use super::types::LeakFinding;
use crate::fingerprint::FingerprintAnalyzer;

impl FingerprintAnalyzer {
    /// Phân tích inbound request để phát hiện rò rỉ fingerprint:
    /// SDK tracking headers, IDE/machine IDs, OS leak qua User-Agent,
    /// browser client hints và local path trong body.
    pub fn analyze_inbound(headers: &HashMap<String, String>, body_sample: Option<&str>) -> Vec<LeakFinding> {
        let mut findings = Vec::new();

        for (k, v) in headers {
            let key_lower = k.to_lowercase();

            // 1. Check SDK tracking headers (OpenAI, Anthropic, etc.)
            if key_lower.starts_with("x-stainless-") {
                findings.push(LeakFinding {
                    category: "sdk_tracking".to_string(),
                    field: format!("Header: {}", k),
                    value: v.clone(),
                    severity: "medium".to_string(),
                    description: "Stainless/OpenAI SDK fingerprint revealing runtime, OS & arch".to_string(),
                });
            } else if key_lower.starts_with("anthropic-client-") || key_lower.contains("anthropic-version") {
                findings.push(LeakFinding {
                    category: "sdk_tracking".to_string(),
                    field: format!("Header: {}", k),
                    value: v.clone(),
                    severity: "low".to_string(),
                    description: "Anthropic Client telemetry/version header".to_string(),
                });
            }

            // 2. Check IDE / Machine IDs
            if key_lower.contains("cursor")
                || key_lower.contains("vscode")
                || key_lower.contains("session-id")
                || key_lower.contains("machine-id")
                || key_lower.contains("client-id")
            {
                findings.push(LeakFinding {
                    category: "device_id".to_string(),
                    field: format!("Header: {}", k),
                    value: v.clone(),
                    severity: "high".to_string(),
                    description: "IDE or unique machine fingerprint ID".to_string(),
                });
            }

            // 3. Check User-Agent for OS & Client specifics
            if key_lower == "user-agent" {
                let v_lower = v.to_lowercase();
                let mut os_detected = Vec::new();
                if v_lower.contains("linux") || v_lower.contains("ubuntu") || v_lower.contains("arch") {
                    os_detected.push("Linux/Distro");
                }
                if v_lower.contains("windows") {
                    os_detected.push("Windows");
                }
                if v_lower.contains("macintosh") || v_lower.contains("mac os") {
                    os_detected.push("macOS");
                }
                if v_lower.contains("electron") {
                    os_detected.push("Electron App");
                }
                if v_lower.contains("opencode") || v_lower.contains("code/") || v_lower.contains("cursor") {
                    os_detected.push("Specific Code Tool");
                }

                if !os_detected.is_empty() {
                    findings.push(LeakFinding {
                        category: "os_info".to_string(),
                        field: "Header: User-Agent".to_string(),
                        value: v.clone(),
                        severity: "medium".to_string(),
                        description: format!("User-Agent leaks client environment: {}", os_detected.join(", ")),
                    });
                }
            }

            // 4. Check sec-ch-ua
            if key_lower.starts_with("sec-ch-ua") {
                findings.push(LeakFinding {
                    category: "browser_hints".to_string(),
                    field: format!("Header: {}", k),
                    value: v.clone(),
                    severity: "low".to_string(),
                    description: "Client Hints revealing platform, architecture or browser engine".to_string(),
                });
            }
        }

        // 5. Body Inspection: check for local filesystem paths
        if let Some(body) = body_sample {
            if let Some(mat) = LOCAL_PATH_REGEX.find(body) {
                findings.push(LeakFinding {
                    category: "path_leak".to_string(),
                    field: "Body: Prompt/Payload".to_string(),
                    value: mat.as_str().to_string(),
                    severity: "high".to_string(),
                    description: "Local user home directory path detected inside payload".to_string(),
                });
            }
        }

        findings
    }
}
