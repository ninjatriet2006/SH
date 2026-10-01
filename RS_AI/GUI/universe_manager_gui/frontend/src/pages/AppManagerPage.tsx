import { useState } from "react";
import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";
import { AppTable } from "../components/AppTable";

export function AppManagerPage() {
  const { t } = useTranslation();
  const apps = useAppStore(s => s.config?.apps ?? []);
  const busy = useAppStore(s => s.busy);
  const isSyncing = useAppStore(s => s.isSyncing);
  const scanApps = useAppStore(s => s.scanApps);
  const startApp = useAppStore(s => s.startApp);
  const stopApp = useAppStore(s => s.stopApp);
  const restartApp = useAppStore(s => s.restartApp);

  const [filterSource, setFilterSource] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");

  const handleStart = (appId: string) => {
    const app = apps.find(a => a.id === appId);
    if (!app) return;
    const msg = t("confirm.start").replace("{name}", app.name);
    if (window.confirm(msg)) void startApp(appId);
  };

  const handleStop = (appId: string) => {
    const app = apps.find(a => a.id === appId);
    if (!app) return;
    const msg = t("confirm.stop").replace("{name}", app.name);
    if (window.confirm(msg)) void stopApp(appId);
  };

  const handleRestart = (appId: string) => {
    const app = apps.find(a => a.id === appId);
    if (!app) return;
    if (window.confirm(`Khởi động lại ứng dụng "${app.name}"?`)) void restartApp(appId);
  };

  const filteredApps = apps.filter(app => {
    const ptype = app.package_type?.toLowerCase() ?? "";
    const isFlatpak = ptype === "flatpak" || app.id.endsWith("-flatpak");
    const isSnap = ptype === "snap" || app.id.endsWith("-snap");
    const isPortable = ptype === "local";
    const isSystem = !isFlatpak && !isSnap && !isPortable;

    if (filterSource === "portable" && !isPortable) return false;
    if (filterSource === "flatpak" && !isFlatpak) return false;
    if (filterSource === "snap" && !isSnap) return false;
    if (filterSource === "system" && !isSystem) return false;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return (
        app.name.toLowerCase().includes(q) ||
        app.id.toLowerCase().includes(q) ||
        (app.category?.toLowerCase() ?? "").includes(q)
      );
    }
    return true;
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <section className="glass-panel" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "0.75rem" }}>
        <div className="filter-pills">
          <button
            className={`filter-pill ${filterSource === "all" ? "active" : ""}`}
            onClick={() => setFilterSource("all")}
          >
            Tất cả ({apps.length})
          </button>
          <button
            className={`filter-pill ${filterSource === "portable" ? "active" : ""}`}
            onClick={() => setFilterSource("portable")}
          >
            💼 Portable ({apps.filter(a => a.package_type?.toLowerCase() === "local").length})
          </button>
          <button
            className={`filter-pill ${filterSource === "flatpak" ? "active" : ""}`}
            onClick={() => setFilterSource("flatpak")}
          >
            🌐 Flatpak ({apps.filter(a => a.package_type?.toLowerCase() === "flatpak" || a.id.endsWith("-flatpak")).length})
          </button>
          <button
            className={`filter-pill ${filterSource === "snap" ? "active" : ""}`}
            onClick={() => setFilterSource("snap")}
          >
            ⚡ Snap ({apps.filter(a => a.package_type?.toLowerCase() === "snap" || a.id.endsWith("-snap")).length})
          </button>
          <button
            className={`filter-pill ${filterSource === "system" ? "active" : ""}`}
            onClick={() => setFilterSource("system")}
          >
            📦 Hệ thống ({apps.filter(a => a.package_type?.toLowerCase() === "system" || (!a.package_type?.toLowerCase().includes("flatpak") && !a.package_type?.toLowerCase().includes("snap") && a.package_type?.toLowerCase() !== "local")).length})
          </button>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
          <span className={`sync-indicator ${isSyncing ? "syncing" : ""}`}>
            <span className="sync-dot" />
            {isSyncing ? "Đang đồng bộ…" : "Đồng bộ tự động"}
          </span>
          <button
            className="btn btn-sm btn-ghost"
            disabled={busy || isSyncing}
            onClick={() => void scanApps(false)}
            title="Làm mới danh sách ứng dụng"
            style={{ padding: "0.35rem 0.6rem" }}
          >
            ⟳
          </button>
        </div>
      </section>

      <section className="glass-panel" style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <input
            className="input"
            style={{ maxWidth: "320px", fontSize: "0.88rem", padding: "0.45rem 0.75rem" }}
            placeholder="🔍 Tìm kiếm ứng dụng…"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
          />
          <span style={{ fontSize: "0.85rem", color: "var(--panel-foreground)" }}>
            Hiển thị <strong>{filteredApps.length}</strong> / {apps.length} ứng dụng
          </span>
        </div>

        <AppTable
          apps={filteredApps}
          actions
          disabled={busy}
          onStart={handleStart}
          onStop={handleStop}
          onRestart={handleRestart}
        />
      </section>
    </div>
  );
}
