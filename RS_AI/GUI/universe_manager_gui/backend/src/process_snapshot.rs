use crate::config::AppEntry;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ProcessSnapshot {
    pub canonical_exes: HashSet<PathBuf>,
    pub names: HashSet<String>,
    pub cmdlines: Vec<String>,
}

impl ProcessSnapshot {
    pub fn collect() -> Self {
        let mut canonical_exes = HashSet::new();
        let mut names = HashSet::new();
        let mut cmdlines = Vec::new();

        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str.chars().all(|c| c.is_ascii_digit()) {
                        let exe_link = path.join("exe");
                        if let Ok(target) = fs::read_link(&exe_link) {
                            if let Some(n) = target.file_name() {
                                names.insert(n.to_string_lossy().to_string());
                            }
                            canonical_exes.insert(target);
                        }

                        let cmd_file = path.join("cmdline");
                        if let Ok(cmd_bytes) = fs::read(cmd_file) {
                            let cmd_str = String::from_utf8_lossy(&cmd_bytes).replace('\0', " ");
                            let trimmed = cmd_str.trim();
                            if !trimmed.is_empty() {
                                cmdlines.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(out) = std::process::Command::new("tasklist").args(["/FO", "CSV", "/NH"]).output() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim_matches('"')).collect();
                    if let Some(first) = parts.first() {
                        let proc_name = first.to_string();
                        names.insert(proc_name.clone());
                        if proc_name.to_lowercase().ends_with(".exe") {
                            names.insert(proc_name[..proc_name.len() - 4].to_string());
                        }
                    }
                }
            }
        }

        ProcessSnapshot {
            canonical_exes,
            names,
            cmdlines,
        }
    }

    pub fn is_running(&self, app: &AppEntry) -> bool {
        let ptype = app.package_type.as_deref().unwrap_or("Local");

        if ptype == "Flatpak" || app.id.ends_with("-flatpak") {
            let clean_id = app.id.strip_suffix("-flatpak").unwrap_or(&app.id);
            return self.cmdlines.iter().any(|cmd| cmd.contains(clean_id));
        }

        if ptype == "Snap" || app.id.ends_with("-snap") {
            let clean_id = app.id.strip_suffix("-snap").unwrap_or(&app.id);
            return self.cmdlines.iter().any(|cmd| cmd.contains(clean_id));
        }

        if !app.exec_path.is_empty() {
            let exec_path_buf = PathBuf::from(&app.exec_path);
            if self.canonical_exes.contains(&exec_path_buf) {
                return true;
            }
            if let Some(file_name) = exec_path_buf.file_name().and_then(|n| n.to_str()) {
                if self.names.contains(file_name) {
                    return true;
                }
            }
            if let Some(first_word) = app.exec_path.split_whitespace().next() {
                let p = Path::new(first_word);
                if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                    if self.names.contains(file_name) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn running_app_ids(&self, apps: &[AppEntry]) -> Vec<String> {
        apps.iter()
            .filter(|app| self.is_running(app))
            .map(|app| app.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_snapshot_collect() {
        let snapshot = ProcessSnapshot::collect();
        assert!(!snapshot.names.is_empty() || !snapshot.cmdlines.is_empty());
        let app = AppEntry {
            id: "test-fake-app".to_string(),
            name: "Fake App".to_string(),
            exec_path: "/nonexistent/fake/binary".to_string(),
            ..AppEntry::default()
        };
        assert!(!snapshot.is_running(&app));
    }
}

