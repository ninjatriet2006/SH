import { useTranslation } from "../utils/i18n";
import type { AppEntry } from "../utils/contract";

interface AppTableProps {
  apps: AppEntry[];
  actions?: boolean;
  disabled?: boolean;
  onStart?: (appId: string) => void;
  onStop?: (appId: string) => void;
  onRestart?: (appId: string) => void;
}

export function AppTable({ apps, actions = false, disabled = false, onStart, onStop, onRestart }: AppTableProps) {
  const { t } = useTranslation();

  if (!apps.length) {
    return <p className="empty">{t("status.no_apps")}</p>;
  }

  const renderSourceBadge = (app: AppEntry) => {
    const ptype = app.package_type?.toLowerCase() ?? "";
    if (ptype === "flatpak" || app.id.endsWith("-flatpak")) {
      return <span className="source-badge flatpak">Flatpak</span>;
    }
    if (ptype === "snap" || app.id.endsWith("-snap")) {
      return <span className="source-badge snap">Snap</span>;
    }
    if (ptype === "local") {
      return <span className="source-badge portable">Portable</span>;
    }
    return <span className="source-badge system">System</span>;
  };

  const renderStatusBadge = (status?: string | null) => {
    const isRunning = status === "Running";
    return (
      <span className={`status-badge ${isRunning ? "running" : "stopped"}`}>
        <span className="dot" />
        {isRunning ? (t("status.running") ?? "Đang chạy") : (t("status.stopped") ?? "Đã dừng")}
      </span>
    );
  };

  return (
    <div className="data-table">
      <div className="table-row heading">
        <span>{t("table.name")}</span>
        <span>{t("table.status") ?? "Trạng thái"}</span>
        <span>{t("table.category") ?? "Danh mục"}</span>
        {actions && <span>{t("table.actions")}</span>}
      </div>
      {apps.map((app) => {
        const isRunning = app.status === "Running";
        return (
          <div className="table-row" key={app.id}>
            <span style={{ display: "flex", alignItems: "center", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              <span style={{ fontWeight: 600 }}>{app.name}</span>
              {renderSourceBadge(app)}
            </span>
            <span>{renderStatusBadge(app.status)}</span>
            <span style={{ color: "var(--panel-foreground)" }}>{app.category ?? "Utility"}</span>
            {actions && (
              <span className="action-buttons">
                {!isRunning ? (
                  <button
                    className="btn btn-sm btn-primary"
                    disabled={disabled}
                    onClick={() => onStart?.(app.id)}
                    title={t("manager.start")}
                  >
                    ▶ {t("manager.start")}
                  </button>
                ) : (
                  <>
                    <button
                      className="btn btn-sm btn-ghost"
                      style={{ color: "var(--error)" }}
                      disabled={disabled}
                      onClick={() => onStop?.(app.id)}
                      title={t("manager.stop")}
                    >
                      ⏹ {t("manager.stop")}
                    </button>
                    {onRestart && (
                      <button
                        className="btn btn-sm btn-ghost"
                        disabled={disabled}
                        onClick={() => onRestart?.(app.id)}
                        title="Khởi động lại"
                      >
                        ⟳ {t("manager.restart") ?? "Khởi động lại"}
                      </button>
                    )}
                  </>
                )}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
