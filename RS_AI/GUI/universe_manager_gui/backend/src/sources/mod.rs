pub mod flatpak;
pub mod portable;
pub mod snap;
pub mod system;
pub mod unknown;
pub mod windows;

use crate::cancellation::CancellationToken;
use crate::config::{AppEntry, ManagerConfig};
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    Portable,
    Flatpak,
    Snap,
    System,
    Windows,
    Unknown,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Portable => "Local",
            Self::Flatpak => "Flatpak",
            Self::Snap => "Snap",
            Self::System => "System",
            Self::Windows => "Windows",
            Self::Unknown => "Unknown",
        }
    }

    pub fn from_app(app: &AppEntry) -> Self {
        match app.package_type.as_deref() {
            Some(ptype) => match ptype.to_ascii_lowercase().as_str() {
                "local" | "portable" => Self::Portable,
                "flatpak" => Self::Flatpak,
                "snap" => Self::Snap,
                "system" | "apt" => Self::System,
                "windows" | "registry" | "winget" | "msix" => Self::Windows,
                _ => match app.id.as_str() {
                    id if id.ends_with("-flatpak") => Self::Flatpak,
                    id if id.ends_with("-snap") => Self::Snap,
                    _ => Self::Unknown,
                },
            },
            None => match app.id.as_str() {
                id if id.ends_with("-flatpak") => Self::Flatpak,
                id if id.ends_with("-snap") => Self::Snap,
                _ => Self::Unknown,
            },
        }
    }
}

pub trait AppSource: Send + Sync {
    fn kind(&self) -> SourceKind;

    fn scan(
        &self,
        config: &ManagerConfig,
        managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>>;

    fn is_running(&self, app: &AppEntry, snapshot: &ProcessSnapshot) -> bool;

    fn start(&self, app: &AppEntry) -> Result<()>;

    fn stop(&self, app: &AppEntry) -> Result<()>;

    fn restart(&self, app: &AppEntry) -> Result<()> {
        let _ = self.stop(app);
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.start(app)
    }
}

pub struct SourceRegistry {
    sources: Vec<Box<dyn AppSource>>,
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self {
            sources: vec![
                Box::new(portable::PortableSource),
                Box::new(flatpak::FlatpakSource),
                Box::new(snap::SnapSource),
                Box::new(system::SystemSource),
                Box::new(windows::WindowsSource),
                Box::new(unknown::UnknownSource),
            ],
        }
    }
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find(&self, kind: SourceKind) -> &dyn AppSource {
        for source in &self.sources {
            if source.kind() == kind {
                return source.as_ref();
            }
        }
        &unknown::UnknownSource
    }

    pub fn scan_all(
        &self,
        config: &ManagerConfig,
        managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        let mut all_apps = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for source in &self.sources {
            cancellation.check()?;
            match source.scan(config, managed_dir, cancellation) {
                Ok(apps) => {
                    for app in apps {
                        if seen_ids.insert(app.id.clone()) {
                            all_apps.push(app);
                        }
                    }
                }
                Err(_) => {
                    // Fail-safe: if one provider errors, keep results from other providers
                }
            }
        }

        all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(all_apps)
    }

    pub fn is_app_running(&self, app: &AppEntry, snapshot: &ProcessSnapshot) -> bool {
        let kind = SourceKind::from_app(app);
        self.find(kind).is_running(app, snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_kind_mapping() {
        let mut app = AppEntry::default();

        app.package_type = Some("Flatpak".to_string());
        assert_eq!(SourceKind::from_app(&app), SourceKind::Flatpak);

        app.package_type = Some("Snap".to_string());
        assert_eq!(SourceKind::from_app(&app), SourceKind::Snap);

        app.package_type = Some("Local".to_string());
        assert_eq!(SourceKind::from_app(&app), SourceKind::Portable);

        app.package_type = Some("System".to_string());
        assert_eq!(SourceKind::from_app(&app), SourceKind::System);

        app.package_type = Some("Windows".to_string());
        assert_eq!(SourceKind::from_app(&app), SourceKind::Windows);

        app.package_type = None;
        app.id = "org.mozilla.firefox-flatpak".to_string();
        assert_eq!(SourceKind::from_app(&app), SourceKind::Flatpak);

        app.id = "code-snap".to_string();
        assert_eq!(SourceKind::from_app(&app), SourceKind::Snap);

        app.id = "custom-app".to_string();
        assert_eq!(SourceKind::from_app(&app), SourceKind::Unknown);
    }

    #[test]
    fn test_source_registry_find() {
        let registry = SourceRegistry::default();
        assert_eq!(registry.find(SourceKind::Portable).kind(), SourceKind::Portable);
        assert_eq!(registry.find(SourceKind::Flatpak).kind(), SourceKind::Flatpak);
        assert_eq!(registry.find(SourceKind::Snap).kind(), SourceKind::Snap);
        assert_eq!(registry.find(SourceKind::System).kind(), SourceKind::System);
        assert_eq!(registry.find(SourceKind::Windows).kind(), SourceKind::Windows);
        assert_eq!(registry.find(SourceKind::Unknown).kind(), SourceKind::Unknown);
    }

    #[test]
    fn test_source_registry_scan_safety() {
        let registry = SourceRegistry::default();
        let config = ManagerConfig::default();
        let cancellation = CancellationToken::default();
        let res = registry.scan_all(&config, None, &cancellation);
        assert!(res.is_ok());
    }
}
