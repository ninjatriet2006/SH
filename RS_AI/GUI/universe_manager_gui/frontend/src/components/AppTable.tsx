import { useTranslation } from "../utils/i18n";
import type { AppEntry } from "../utils/contract";

interface AppTableProps {
  apps: AppEntry[];
  actions?: boolean;
  disabled?: boolean;
  selectedAppId?: string;
  onSelectApp?: (app: AppEntry) => void;
  checkedAppIds?: Set<string>;
  onToggleCheck?: (appId: string) => void;
  onToggleCheckAll?: () => void;
  onStart?: (appId: string) => void;
  onStop?: (appId: string) => void;
  onRestart?: (appId: string) => void;
}

export function AppTable({
  apps,
  actions = false,
  disabled = false,
  selectedAppId,
  onSelectApp,
  checkedAppIds,
  onToggleCheck,
  onToggleCheckAll,
  onStart,
  onStop,
  onRestart,
}: AppTableProps) {
  const { t } = useTranslation();

  if (!apps.length) {
    return <p className="empty">{t("status.no_apps")}</p>;
  }

  const allChecked = checkedAppIds && apps.length > 0 && apps.every((a) => checkedAppIds.has(a.id));

  const renderSourceBadge = (app: AppEntry) => {
    const ptype = app.package_type?.toLowerCase() ?? "";
    if (ptype === "flatpak" || app.id.endsWith("-flatpak")) {
      return <span className="source-badge flatpak">Flatpak</span>;
    }
    if (ptype === "snap" || app.id.endsWith("-snap")) {
      return <span className="source-badge snap">Snap</span>;
    }
    if (ptype === "local" || ptype === "portable") {
      return <span className="source-badge portable">Local</span>;
    }
    if (ptype === "apt") {
      return <span className="source-badge apt">APT</span>;
    }
    if (ptype === "cli" || app.id.startsWith("cli-")) {
      return <span className="source-badge cli">CLI</span>;
    }
    return <span className="source-badge system">{app.package_type || "System"}</span>;
  };

  const renderStatusBadge = (status?: string | null) => {
    const isRunning = status === "Running";
    return (
      <span className={`status-badge ${isRunning ? "running" : "stopped"}`}>
        <span className="dot" />
        {isRunning ? "RUNNING" : "STOPPED"}
      </span>
    );
  };

  return (
    <div className="data-table">
      <div className={`table-row heading ${actions ? "with-actions" : ""}`}>
        {checkedAppIds && (
          <span className="col-check">
            <input
              type="checkbox"
              checked={Boolean(allChecked)}
              onChange={onToggleCheckAll}
              title="Chọn tất cả ứng dụng"
            />
          </span>
        )}
        <span className="col-name">{t("table.name") || "Tên ứng dụng"}</span>
        <span className="col-category">{t("table.category") || "Chuyên mục"}</span>
        <span className="col-source">{t("table.source") || "Nguồn"}</span>
        <span className="col-status">{t("table.status") || "Trạng thái"}</span>
        {actions && <span className="col-actions">{t("table.actions") || "Thao tác"}</span>}
      </div>

      <div className="table-body-scroll">
        {apps.map((app) => {
          const isRunning = app.status === "Running";
          const isSelected = selectedAppId === app.id;
          const isChecked = checkedAppIds ? checkedAppIds.has(app.id) : false;

          return (
            <div
              className={`table-row ${isSelected ? "selected-row" : ""} ${actions ? "with-actions" : ""}`}
              key={app.id}
              onClick={() => onSelectApp?.(app)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  onSelectApp?.(app);
                }
              }}
            >
              {checkedAppIds && (
                <span
                  className="col-check"
                  onClick={(e) => {
                    e.stopPropagation();
                  }}
                >
                  <input
                    type="checkbox"
                    checked={isChecked}
                    onChange={() => onToggleCheck?.(app.id)}
                  />
                </span>
              )}

              <span className="col-name" title={app.name}>
                <span className="app-name-text">{app.name}</span>
              </span>

              <span className="col-category" title={app.category ?? "Utility"}>
                <span className="category-text">{app.category ?? "Other"}</span>
              </span>

              <span className="col-source">{renderSourceBadge(app)}</span>

              <span className="col-status">{renderStatusBadge(app.status)}</span>

              {actions && (
                <span
                  className="col-actions action-buttons"
                  onClick={(e) => e.stopPropagation()}
                >
                  {!isRunning ? (
                    <button
                      type="button"
                      className="btn btn-sm btn-primary"
                      disabled={disabled}
                      onClick={() => onStart?.(app.id)}
                      title={t("manager.start") || "Khởi động"}
                    >
                      ▶
                    </button>
                  ) : (
                    <>
                      <button
                        type="button"
                        className="btn btn-sm btn-danger"
                        disabled={disabled}
                        onClick={() => onStop?.(app.id)}
                        title={t("manager.stop") || "Dừng"}
                      >
                        ⏹
                      </button>
                      {onRestart && (
                        <button
                          type="button"
                          className="btn btn-sm btn-ghost"
                          disabled={disabled}
                          onClick={() => onRestart?.(app.id)}
                          title="Khởi động lại"
                        >
                          ⟳
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
    </div>
  );
}
