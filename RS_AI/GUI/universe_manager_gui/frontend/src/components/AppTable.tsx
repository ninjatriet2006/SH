import { useTranslation } from "../utils/i18n";
import type { AppEntry } from "../utils/contract";

interface AppTableProps {
  apps: AppEntry[];
  actions?: boolean;
  disabled?: boolean;
  onStart?: (appId: string) => void;
  onStop?: (appId: string) => void;
}

export function AppTable({ apps, actions = false, disabled = false, onStart, onStop }: AppTableProps) {
  const { t } = useTranslation();

  if (!apps.length) {
    return <p className="empty">{t("status.no_apps")}</p>;
  }

  return (
    <div className="data-table">
      <div className="table-row heading">
        <span>{t("table.name")}</span>
        <span>{t("table.version")}</span>
        <span>{t("table.type")}</span>
        {actions && <span>{t("table.actions")}</span>}
      </div>
      {apps.map((app) => (
        <div className="table-row" key={app.id}>
          <span>{app.name}</span>
          <span>{app.version ?? "—"}</span>
          <span>{app.install_type}</span>
          {actions && (
            <span className="action-buttons">
              <button
                className="btn btn-sm btn-primary"
                disabled={disabled}
                onClick={() => onStart?.(app.id)}
              >
                {t("manager.start")}
              </button>
              <button
                className="btn btn-sm btn-ghost"
                disabled={disabled}
                onClick={() => onStop?.(app.id)}
              >
                {t("manager.stop")}
              </button>
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
