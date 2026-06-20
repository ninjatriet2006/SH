use std::process::Command;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

pub fn check_package_managers() -> Vec<(String, bool)> {
    let mut results = Vec::new();
    
    // Check winget
    let winget_ok = Command::new("winget").arg("--version").output().is_ok();
    results.push(("Winget".to_string(), winget_ok));
    
    // Check choco
    let choco_ok = Command::new("choco").arg("--version").output().is_ok();
    results.push(("Chocolatey".to_string(), choco_ok));
    
    // Check scoop
    let scoop_ok = Command::new("scoop").arg("--version").output().is_ok();
    results.push(("Scoop".to_string(), scoop_ok));
    
    results
}

pub fn install_package_manager(name: &str) -> Result<String, String> {
    match name {
        "Winget" => {
            let script = "Invoke-WebRequest -Uri 'https://github.com/microsoft/winget-cli/releases/latest/download/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle' -OutFile winget.msixbundle; Add-AppxPackage winget.msixbundle; Remove-Item winget.msixbundle -Force";
            let status = Command::new("powershell")
                .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
                .status();
            match status {
                Ok(s) if s.success() => Ok("Đã cài đặt Winget thành công!".to_string()),
                _ => Err("Lỗi khi cài đặt Winget".to_string()),
            }
        }
        "Chocolatey" => {
            let script = "Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))";
            let status = Command::new("powershell")
                .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
                .status();
            match status {
                Ok(s) if s.success() => Ok("Đã cài đặt Chocolatey thành công!".to_string()),
                _ => Err("Lỗi khi cài đặt Chocolatey".to_string()),
            }
        }
        "Scoop" => {
            let script = "Set-ExecutionPolicy RemoteSigned -Scope CurrentUser; Invoke-RestMethod -Uri https://get.scoop.sh | Invoke-Expression";
            let status = Command::new("powershell")
                .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
                .status();
            match status {
                Ok(s) if s.success() => Ok("Đã cài đặt Scoop thành công!".to_string()),
                _ => Err("Lỗi khi cài đặt Scoop".to_string()),
            }
        }
        _ => Err("Không hỗ trợ cài đặt công cụ này".to_string()),
    }
}

pub fn search_apps(query: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    
    // 1. Winget search
    if Command::new("winget").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("winget").args(&["search", query, "--accept-source-agreements"]).output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            let mut started = false;
            for line in combined.lines() {
                let tline = line.trim();
                if tline.contains("Name") && tline.contains("Id") && tline.contains("Version") {
                    started = true; continue;
                }
                if started {
                    if tline.starts_with('-') || tline.is_empty() { continue; }
                    let parts: Vec<&str> = tline.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let source = parts.last().unwrap().to_string();
                        if parts.len() >= 4 {
                            let current = parts[parts.len()-2].to_string();
                            let id = parts[parts.len()-3].to_string();
                            let name = parts[..parts.len()-3].join(" ");
                            if id != "Id" && name != "Name" {
                                results.push(SearchResult {
                                    name, id, version: current, source
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Chocolatey search
    if Command::new("choco").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("choco").args(&["search", query, "--limit-output"]).output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            for line in combined.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 2 {
                    let id = parts[0].trim().to_string();
                    let version = parts[1].trim().to_string();
                    results.push(SearchResult {
                        name: id.clone(),
                        id,
                        version,
                        source: "chocolatey".to_string(),
                    });
                }
            }
        }
    }
    
    // 3. Scoop search
    if Command::new("scoop").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("powershell").args(&["-NoProfile", "-Command", &format!("scoop search {}", query)]).output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            let mut started = false;
            for line in combined.lines() {
                let tline = line.trim();
                if tline.starts_with("Results from") {
                    started = true; continue;
                }
                if started && tline.is_empty() { continue; }
                if started {
                    let parts: Vec<&str> = tline.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let id = parts[0].trim().to_string();
                        let version_str = parts[1].trim();
                        let version = version_str.trim_matches('(').trim_matches(')').to_string();
                        // Ignore warning lines or weird outputs
                        if !id.contains(":") && !id.starts_with("WARN") {
                            results.push(SearchResult {
                                name: id.clone(),
                                id,
                                version,
                                source: "scoop".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    
    results
}

pub fn install_app(id: &str, source: &str) -> Result<String, String> {
    let status = match source {
        "winget" | "msstore" => {
            Command::new("winget").args(&["install", "--id", id, "--silent", "--accept-package-agreements", "--accept-source-agreements"]).status()
        }
        "chocolatey" => {
            Command::new("choco").args(&["install", id, "-y"]).status()
        }
        "scoop" => {
            Command::new("powershell").args(&["-NoProfile", "-Command", &format!("scoop install {}", id)]).status()
        }
        _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unknown source")),
    };
    
    match status {
        Ok(s) if s.success() => Ok(format!("Đã cài đặt thành công {} qua {}", id, source)),
        _ => Err(format!("Lỗi khi cài đặt {} qua {}", id, source)),
    }
}
