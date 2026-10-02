import { useState, useMemo, useEffect, useRef, useDeferredValue } from "react";
import { Search, RefreshCw, Layers, Terminal, PackagePlus } from "lucide-react";
import { NavLink } from "react-router-dom";
import { useAppStore } from "../store/useAppStore";
import { useNotificationStore } from "../store/useNotificationStore";
import { useTranslation } from "../utils/i18n";
import { AppTable } from "../components/AppTable";
import { AppDetailsPanel } from "../components/AppDetailsPanel";
import type { AppEntry } from "../utils/contract";

export function AppManagerPage() {
  const { t } = useTranslation();
  const apps = useAppStore((s) => s.config?.apps ?? []);
  const busy = useAppStore((s) => s.busy);
  const isSyncing = useAppStore((s) => s.isSyncing);
  const scanApps = useAppStore((s) => s.scanApps);
  const startApp = useAppStore((s) => s.startApp);
  const stopApp = useAppStore((s) => s.stopApp);
  const restartApp = useAppStore((s) => s.restartApp);

  const logs = useNotificationStore((s) => s.logs);
  const recentLogs = useMemo(() => logs.slice(-4), [logs]);

  const [filterSource, setFilterSource] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const deferredQuery = useDeferredValue(searchQuery);
  const [selectedAppId, setSelectedAppId] = useState<string | undefined>(undefined);
  const [checkedAppIds, setCheckedAppIds] = useState<Set<string>>(new Set());

  const searchInputRef = useRef<HTMLInputElement>(null);

  // Global shortcut '/' to focus search, 'Escape' to clear
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "/" && document.activeElement !== searchInputRef.current) {
        e.preventDefault();
        searchInputRef.current?.focus();
      } else if (e.key === "Escape" && document.activeElement === searchInputRef.current) {
        setSearchQuery("");
        searchInputRef.current?.blur();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleStart = (appId: string) => {
    const app = apps.find((a) => a.id === appId);
    if (!app) return;
    const msg = t("confirm.start").replace("{name}", app.name);
    if (window.confirm(msg)) void startApp(appId);
  };

  const handleStop = (appId: string) => {
    const app = apps.find((a) => a.id === appId);
    if (!app) return;
    const msg = t("confirm.stop").replace("{name}", app.name);
    if (window.confirm(msg)) void stopApp(appId);
  };

  const handleRestart = (appId: string) => {
    const app = apps.find((a) => a.id === appId);
    if (!app) return;
    if (window.confirm(`Khởi động lại ứng dụng "${app.name}"?`)) void restartApp(appId);
  };

  const filteredApps = useMemo(() => {
    return apps.filter((app) => {
      const ptype = (app.package_type ?? "").toLowerCase();
      const isCli = ptype === "cli" || app.id.startsWith("cli-");
      const isFlatpak = ptype === "flatpak" || app.id.endsWith("-flatpak");
      const isSnap = ptype === "snap" || app.id.endsWith("-snap");
      const isPortable = ptype === "local" || ptype === "portable";
      const isApt = ptype === "apt" || (!isFlatpak && !isSnap && !isPortable && !isCli);

      if (filterSource === "portable" && !isPortable) return false;
      if (filterSource === "cli" && !isCli) return false;
      if (filterSource === "flatpak" && !isFlatpak) return false;
      if (filterSource === "snap" && !isSnap) return false;
      if (filterSource === "apt" && !isApt) return false;

      if (deferredQuery.trim()) {
        const q = deferredQuery.toLowerCase();
        return (
          app.name.toLowerCase().includes(q) ||
          app.id.toLowerCase().includes(q) ||
          (app.category?.toLowerCase() ?? "").includes(q)
        );
      }
      return true;
    });
  }, [apps, filterSource, deferredQuery]);

  const effectiveSelectedId = selectedAppId && filteredApps.some((a) => a.id === selectedAppId)
    ? selectedAppId
    : filteredApps[0]?.id;

  const selectedApp = useMemo(() => {
    return apps.find((a) => a.id === effectiveSelectedId) ?? null;
  }, [apps, effectiveSelectedId]);

  const handleToggleCheck = (appId: string) => {
    setCheckedAppIds((prev) => {
      const next = new Set(prev);
      if (next.has(appId)) {
        next.delete(appId);
      } else {
        next.add(appId);
      }
      return next;
    });
  };

  const handleToggleCheckAll = () => {
    if (checkedAppIds.size === filteredApps.length && filteredApps.length > 0) {
      setCheckedAppIds(new Set());
    } else {
      setCheckedAppIds(new Set(filteredApps.map((a) => a.id)));
    }
  };

  const handleBatchStart = async () => {
    const ids = Array.from(checkedAppIds);
    for (const id of ids) {
      await startApp(id);
    }
  };

  const handleBatchStop = async () => {
    const ids = Array.from(checkedAppIds);
    for (const id of ids) {
      await stopApp(id);
    }
  };

  const aptCount = apps.filter((a) => {
    const p = (a.package_type ?? "").toLowerCase();
    const isCli = p === "cli" || a.id.startsWith("cli-");
    return (p === "apt" || (!p.includes("flatpak") && !p.includes("snap") && p !== "local")) && !isCli;
  }).length;
  const portableCount = apps.filter((a) => (a.package_type ?? "").toLowerCase() === "local").length;
  const cliCount = apps.filter((a) => (a.package_type ?? "").toLowerCase() === "cli" || a.id.startsWith("cli-")).length;
  const flatpakCount = apps.filter((a) => (a.package_type ?? "").toLowerCase() === "flatpak" || a.id.endsWith("-flatpak")).length;
  const snapCount = apps.filter((a) => (a.package_type ?? "").toLowerCase() === "snap" || a.id.endsWith("-snap")).length;

  return (
    <div className="manager-page-layout">
      {/* Unified Command & Filter Toolbar (Raycast/Linear style) */}
      <header className="glass-panel manager-header-toolbar">
        <div className="toolbar-top-row">
          <div className="search-box command-search">
            <Search size={16} className="search-icon" />
            <input
              ref={searchInputRef}
              type="text"
              className="command-search-input"
              placeholder="Tìm kiếm ứng dụng theo tên, ID, chuyên mục… (Nhấn '/' để tìm)"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery ? (
              <button
                type="button"
                className="clear-search-btn"
                onClick={() => setSearchQuery("")}
                title="Xóa tìm kiếm (Esc)"
              >
                ×
              </button>
            ) : (
              <kbd className="search-kbd" title="Nhấn phím '/' để kích hoạt tìm kiếm nhanh">/</kbd>
            )}
          </div>

          <div className="sync-actions-group">
            <NavLink
              to="/add"
              className="btn btn-sm btn-primary"
              style={{ display: "inline-flex", alignItems: "center", gap: "0.35rem", textDecoration: "none", fontWeight: 600 }}
              title="Thêm & Tích hợp ứng dụng mới từ thư mục, AppImage hoặc binary"
            >
              <PackagePlus size={14} /> <span>Thêm ứng dụng</span>
            </NavLink>
            <span className={`sync-indicator ${isSyncing ? "syncing" : ""}`}>
              <span className="sync-dot" />
              <span>{isSyncing ? "Đang đồng bộ…" : "Tự động đồng bộ (6s)"}</span>
            </span>
            <button
              type="button"
              className="btn btn-sm btn-ghost refresh-btn"
              disabled={busy}
              onClick={() => void scanApps(false)}
              title="Quét lại toàn bộ ứng dụng hệ thống"
            >
              <RefreshCw size={13} className={busy ? "spin" : ""} /> Quét lại
            </button>
          </div>
        </div>

        <div className="toolbar-bottom-row">
          <div className="filter-pills">
            <button
              type="button"
              className={`filter-pill ${filterSource === "all" ? "active" : ""}`}
              onClick={() => setFilterSource("all")}
            >
              Tất cả <span className="pill-badge">{apps.length}</span>
            </button>
            <button
              type="button"
              className={`filter-pill ${filterSource === "apt" ? "active" : ""}`}
              onClick={() => setFilterSource("apt")}
            >
              📦 APT <span className="pill-badge">{aptCount}</span>
            </button>
            <button
              type="button"
              className={`filter-pill ${filterSource === "portable" ? "active" : ""}`}
              onClick={() => setFilterSource("portable")}
            >
              💼 Portable <span className="pill-badge">{portableCount}</span>
            </button>
            <button
              type="button"
              className={`filter-pill ${filterSource === "flatpak" ? "active" : ""}`}
              onClick={() => setFilterSource("flatpak")}
            >
              🌐 Flatpak <span className="pill-badge">{flatpakCount}</span>
            </button>
            <button
              type="button"
              className={`filter-pill ${filterSource === "snap" ? "active" : ""}`}
              onClick={() => setFilterSource("snap")}
            >
              ⚡ Snap <span className="pill-badge">{snapCount}</span>
            </button>
            <button
              type="button"
              className={`filter-pill ${filterSource === "cli" ? "active" : ""}`}
              onClick={() => setFilterSource("cli")}
            >
              💻 CLI <span className="pill-badge">{cliCount}</span>
            </button>
          </div>

          {checkedAppIds.size > 0 ? (
            <div className="batch-actions-strip">
              <span className="batch-count">
                <Layers size={14} /> Đã chọn {checkedAppIds.size} ứng dụng
              </span>
              <button
                type="button"
                className="btn btn-sm btn-primary"
                disabled={busy}
                onClick={handleBatchStart}
              >
                ▶ Khởi động
              </button>
              <button
                type="button"
                className="btn btn-sm btn-danger"
                disabled={busy}
                onClick={handleBatchStop}
              >
                ⏹ Dừng
              </button>
              <button
                type="button"
                className="btn btn-sm btn-ghost"
                onClick={() => setCheckedAppIds(new Set())}
              >
                Bỏ chọn
              </button>
            </div>
          ) : (
            searchQuery.trim() && (
              <div className="search-stats-badge">
                Hiển thị <strong>{filteredApps.length}</strong> / {apps.length} ứng dụng
              </div>
            )
          )}
        </div>
      </header>

      {/* Master - Detail Split View (TUI Parity) */}
      <section className="manager-split-container">
        <div className="manager-table-column">
          <AppTable
            apps={filteredApps}
            actions={false}
            disabled={busy}
            selectedAppId={effectiveSelectedId}
            onSelectApp={(app: AppEntry) => setSelectedAppId(app.id)}
            checkedAppIds={checkedAppIds}
            onToggleCheck={handleToggleCheck}
            onToggleCheckAll={handleToggleCheckAll}
            onStart={handleStart}
            onStop={handleStop}
            onRestart={handleRestart}
          />

          {/* Activity Journal beneath table (matching TUI bottom panel) */}
          <div className="activity-journal-panel">
            <div className="journal-header">
              <span className="journal-title">
                <Terminal size={13} /> Nhật ký hoạt động
              </span>
              <NavLink to="/debug" className="journal-view-all">
                Xem tất cả ({logs.length})
              </NavLink>
            </div>
            <div className="journal-entries">
              {recentLogs.length === 0 ? (
                <div className="journal-empty">Chưa có nhật ký hoạt động nào.</div>
              ) : (
                recentLogs.map((log) => (
                  <div key={log.id} className={`journal-row log-${log.level}`}>
                    <span className="journal-time">
                      [{new Date(log.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}]
                    </span>
                    <span className={`journal-badge log-${log.level}`}>
                      [{log.level.toUpperCase()}]
                    </span>
                    <span className="journal-text">{log.message}</span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        <div className="manager-detail-column">
          <AppDetailsPanel
            app={selectedApp}
            disabled={busy}
            onStart={handleStart}
            onStop={handleStop}
            onRestart={handleRestart}
          />
        </div>
      </section>
    </div>
  );
}
