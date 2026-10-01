import { Routes, Route, NavLink } from "react-router-dom";
import { LayoutDashboard, Settings, FolderCog, ScanSearch, AppWindow, Search } from "lucide-react";
import { useEffect } from "react";

import { DashboardPage } from "./pages/DashboardPage";
import { ConfigPage } from "./pages/ConfigPage";
import { ScanPage } from "./pages/ScanPage";
import { AppManagerPage } from "./pages/AppManagerPage";
import { SearchPage } from "./pages/SearchPage";
import { SettingsPage } from "./pages/SettingsPage";

import { useSettingsStore } from "./store/useSettingsStore";
import { useAppStore } from "./store/useAppStore";
import { useTranslation } from "./utils/i18n";

export default function App() {
  const { initSettings, isLoading } = useSettingsStore();
  const { loadConfig, scanApps, busy, progress, error, clearError, dispose } = useAppStore();
  const { t } = useTranslation();

  useEffect(() => {
    let syncInterval: ReturnType<typeof setInterval> | null = null;
    const init = async () => {
      await initSettings();
      await loadConfig();
      await scanApps(true);

      syncInterval = setInterval(() => {
        if (!document.hidden) {
          void scanApps(true);
        }
      }, 4000);
    };
    void init();
    return () => {
      if (syncInterval) clearInterval(syncInterval);
      dispose();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- mount-only: zustand actions are stable refs
  }, []);

  if (isLoading) {
    return (
      <div className="loading-screen">
        <div className="spinner" />
      </div>
    );
  }

  return (
    <div className="app-layout">
      <nav className="sidebar">
        <h2 className="sidebar-title">{t("app.title")}</h2>

        <NavLink to="/" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`} end>
          <LayoutDashboard size={18} /> {t("nav.dashboard")}
        </NavLink>
        <NavLink to="/config" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`}>
          <FolderCog size={18} /> {t("nav.config")}
        </NavLink>
        <NavLink to="/scan" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`}>
          <ScanSearch size={18} /> {t("nav.scan")}
        </NavLink>
        <NavLink to="/manager" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`}>
          <AppWindow size={18} /> {t("nav.manager")}
        </NavLink>
        <NavLink to="/search" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`}>
          <Search size={18} /> {t("nav.search")}
        </NavLink>
        <NavLink to="/settings" className={({ isActive }) => `nav-link ${isActive ? "active" : ""}`}>
          <Settings size={18} /> {t("nav.settings")}
        </NavLink>
      </nav>

      <main className="main-content">
        {error && (
          <div className="error-banner" role="alert" onClick={clearError}>
            {error}
          </div>
        )}
        {busy && <div className="progress-bar"><span>{progress}</span></div>}

        <Routes>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/config" element={<ConfigPage />} />
          <Route path="/scan" element={<ScanPage />} />
          <Route path="/manager" element={<AppManagerPage />} />
          <Route path="/search" element={<SearchPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}
