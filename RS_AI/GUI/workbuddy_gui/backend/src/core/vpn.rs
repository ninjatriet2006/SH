//! AdGuard VPN manager — ported from Go internal/vpn.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConfig {
    pub enabled: bool,
    pub location: String,
    pub port: i32,
}

impl Default for VpnConfig {
    fn default() -> Self {
        Self { enabled: false, location: String::new(), port: 1080 }
    }
}

pub struct VpnManager {
    cfg: VpnConfig,
}

impl VpnManager {
    pub fn new(cfg: VpnConfig) -> Self {
        Self { cfg }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.cfg.enabled && self.cfg.port <= 0 {
            return Err(format!("vpn.port must be positive, got {}", self.cfg.port));
        }
        Ok(())
    }

    pub fn connect(&self) -> Result<String, String> {
        if !self.cfg.enabled {
            return Ok("VPN disabled".to_string());
        }
        let mut args = vec!["connect", "--socks"];
        let port_str = self.cfg.port.to_string();
        args.push(&port_str);
        if !self.cfg.location.is_empty() {
            args.push("--location");
            args.push(&self.cfg.location);
        }
        let output = Command::new("adguardvpn-cli")
            .args(&args)
            .output()
            .map_err(|e| format!("VPN connect failed: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            Ok(format!("VPN connected: {stdout}"))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("VPN connect error: {stderr}"))
        }
    }

    pub fn disconnect(&self) -> Result<String, String> {
        let output = Command::new("adguardvpn-cli")
            .arg("disconnect")
            .output()
            .map_err(|e| format!("VPN disconnect failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn proxy_url(&self) -> Option<String> {
        if self.cfg.enabled {
            Some(format!("socks5://127.0.0.1:{}", self.cfg.port))
        } else {
            None
        }
    }
}
