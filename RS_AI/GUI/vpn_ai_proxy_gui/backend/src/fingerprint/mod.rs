use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakFinding {
    pub category: String, // "os_info", "ide_fingerprint", "sdk_tracking", "path_leak", "device_id"
    pub field: String,    // "header: User-Agent", "header: x-stainless-os", "body: prompt"
    pub value: String,
    pub severity: String, // "low", "medium", "high"
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintProfile {
    pub mode: String, // "stealth", "browser_chrome", "minimal_curl", "raw", "custom"
    pub custom_user_agent: Option<String>,
    pub strip_sdk_headers: bool,
    pub strip_ide_headers: bool,
    pub strip_sec_ch_ua: bool,
    pub remove_empty_headers: bool,
    pub mask_local_paths_in_body: bool,
}

impl Default for FingerprintProfile {
    fn default() -> Self {
        Self {
            mode: "stealth".to_string(),
            custom_user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".to_string()),
            strip_sdk_headers: true,
            strip_ide_headers: true,
            strip_sec_ch_ua: true,
            remove_empty_headers: true,
            mask_local_paths_in_body: true,
        }
    }
}

pub struct FingerprintAnalyzer;

impl FingerprintAnalyzer {
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
            let path_regex = regex::Regex::new(r"(/home/[a-zA-Z0-9_-]+|[A-Za-z]:\\[a-zA-Z0-9_\\]+)").unwrap();
            if let Some(mat) = path_regex.find(body) {
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

    pub fn sanitize_headers(
        headers: &HashMap<String, String>,
        profile: &FingerprintProfile,
    ) -> HashMap<String, String> {
        let mut clean = HashMap::new();

        for (k, v) in headers {
            let key_lower = k.to_lowercase();

            // Strip SDK
            if profile.strip_sdk_headers && (key_lower.starts_with("x-stainless-") || key_lower.starts_with("anthropic-client-")) {
                continue;
            }

            // Strip IDE
            if profile.strip_ide_headers
                && (key_lower.contains("cursor")
                    || key_lower.contains("vscode")
                    || key_lower.contains("machine-id")
                    || key_lower.contains("session-id"))
            {
                continue;
            }

            // Strip Sec-CH-UA
            if profile.strip_sec_ch_ua && key_lower.starts_with("sec-ch-ua") {
                continue;
            }

            // Skip empty
            if profile.remove_empty_headers && v.trim().is_empty() {
                continue;
            }

            // User-Agent transform
            if key_lower == "user-agent" {
                if let Some(ref ua) = profile.custom_user_agent {
                    clean.insert("user-agent".to_string(), ua.clone());
                    continue;
                }
            }

            clean.insert(key_lower, v.clone());
        }

        // Ensure User-Agent is set if custom_user_agent exists and wasn't in original headers
        if let Some(ref ua) = profile.custom_user_agent {
            clean.entry("user-agent".to_string()).or_insert_with(|| ua.clone());
        }

        clean
    }
}
