//! IDE Executable and Desktop file detector for Linux/macOS/Windows.

use crate::ipc::{respond, IpcResult, Req};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct DetectIdePathRequest {
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectIdePathResponse {
    pub found: bool,
    pub path: Option<String>,
    pub message: String,
}

/// Quét các tệp `.desktop` trên hệ thống Linux để tìm đường dẫn thực thi chính xác của IDE.
pub fn scan_desktop_files_for_exec(target_key: &str) -> Option<String> {
    let mut search_dirs = vec![
        std::path::PathBuf::from("/usr/share/applications"),
        std::path::PathBuf::from("/usr/local/share/applications"),
        std::path::PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        search_dirs.push(std::path::Path::new(&home).join(".local/share/applications"));
        search_dirs.push(std::path::Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }

    let target_lower = target_key.to_lowercase();

    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "desktop") {
                    let filename = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();

                    let matched = match target_lower.as_str() {
                        "antigravity_ide" => filename.contains("antigravity-ide"),
                        "antigravity_desktop" => {
                            filename.contains("antigravity") && !filename.contains("antigravity-ide")
                        }
                        "vscode" => {
                            filename == "code.desktop"
                                || filename.contains("visual-studio-code")
                                || filename.contains("com.microsoft.vscode")
                        }
                        "cursor" => filename.contains("cursor") && !filename.contains("url-handler"),
                        "zed" => filename.contains("zed"),
                        "trae" | "trae_solo" | "trae_cn" => filename.contains("trae"),
                        "codebuddy" => filename.contains("codebuddy") && !filename.contains("codebuddy-cn"),
                        "codebuddy_cn" => filename.contains("codebuddy-cn") || filename.contains("codebuddy_cn"),
                        "claude" => filename.contains("claude"),
                        "windsurf" | "devin" => filename.contains("windsurf") || filename.contains("devin"),
                        "codex" => filename.contains("codex"),
                        "kiro" => filename.contains("kiro"),
                        "workbuddy" => filename.contains("workbuddy"),
                        "qoder" => filename.contains("qoder"),
                        "zcode" => filename.contains("zcode"),
                        "grok" => filename.contains("grok"),
                        _ => false,
                    };

                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let is_match = matched
                            || match target_lower.as_str() {
                                "antigravity_ide" => content.contains("Name=Antigravity IDE"),
                                "antigravity_desktop" => {
                                    content.contains("Name=Antigravity")
                                        && !content.contains("Name=Antigravity IDE")
                                }
                                "vscode" => {
                                    content.contains("Name=Visual Studio Code")
                                        || content.contains("Name=Code")
                                }
                                "cursor" => content.contains("Name=Cursor"),
                                "zed" => content.contains("Name=Zed"),
                                "trae" | "trae_solo" | "trae_cn" => content.contains("Name=Trae"),
                                "codebuddy" => content.contains("Name=CodeBuddy") && !content.contains("Name=CodeBuddy CN"),
                                "codebuddy_cn" => content.contains("Name=CodeBuddy CN") || content.contains("CodeBuddy (CN)"),
                                "claude" => content.contains("Name=Claude"),
                                "windsurf" | "devin" => content.contains("Name=Windsurf") || content.contains("Name=Devin"),
                                "codex" => content.contains("Name=Codex"),
                                "kiro" => content.contains("Name=Kiro"),
                                "workbuddy" => content.contains("Name=WorkBuddy"),
                                "qoder" => content.contains("Name=Qoder"),
                                "zcode" => content.contains("Name=ZCode"),
                                "grok" => content.contains("Name=Grok"),
                                _ => false,
                            };

                        if is_match {
                            for line in content.lines() {
                                if let Some(stripped) = line.strip_prefix("Exec=") {
                                    let trimmed = stripped.trim();
                                    let raw_path = if let Some(idx) = trimmed.find(" %") {
                                        &trimmed[..idx]
                                    } else {
                                        trimmed.split_whitespace().next().unwrap_or(trimmed)
                                    };
                                    let path_clean = raw_path.trim_matches('"');
                                    if std::path::Path::new(path_clean).is_file() {
                                        return Some(path_clean.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: check `which` binary command
    let binary_name = match target_lower.as_str() {
        "vscode" => Some("code"),
        "antigravity_ide" => Some("antigravity-ide"),
        "antigravity_desktop" => Some("antigravity"),
        "cursor" => Some("cursor"),
        "trae" | "trae_solo" | "trae_cn" => Some("trae"),
        "zed" => Some("zed"),
        "codebuddy" => Some("codebuddy"),
        "codebuddy_cn" => Some("codebuddy-cn"),
        "claude" => Some("claude"),
        "windsurf" => Some("windsurf"),
        "devin" => Some("devin"),
        "codex" => Some("codex"),
        "kiro" => Some("kiro"),
        "workbuddy" => Some("workbuddy"),
        "qoder" => Some("qoder"),
        "zcode" => Some("zcode"),
        "grok" => Some("grok"),
        _ => None,
    };
    if let Some(cmd) = binary_name {
        if let Ok(out) = std::process::Command::new("which").arg(cmd).output() {
            if out.status.success() {
                let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path_str.is_empty() && std::path::Path::new(&path_str).is_file() {
                    return Some(path_str);
                }
            }
        }
    }

    None
}

#[tauri::command(rename_all = "snake_case")]
pub fn auto_detect_ide_path(
    request: Req<DetectIdePathRequest>,
) -> IpcResult<DetectIdePathResponse> {
    let (request_id, payload) = request.validate()?;
    if let Some(found_path) = scan_desktop_files_for_exec(&payload.target) {
        Ok(respond(
            request_id,
            DetectIdePathResponse {
                found: true,
                path: Some(found_path.clone()),
                message: format!("Đã phát hiện thành công qua tệp .desktop: {}", found_path),
            },
        ))
    } else {
        Ok(respond(
            request_id,
            DetectIdePathResponse {
                found: false,
                path: None,
                message: "Không tự động tìm thấy ứng dụng qua tệp .desktop. Vui lòng bấm 'Browse' để chọn tệp thủ công.".into(),
            },
        ))
    }
}
