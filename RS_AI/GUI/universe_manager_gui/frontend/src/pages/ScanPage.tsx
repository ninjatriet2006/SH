import { Link } from "react-router-dom";
import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";
import { AppTable } from "../components/AppTable";
import { DetectionReport } from "../components/DetectionReport";

export function ScanPage() {
  const { t } = useTranslation();
  const config = useAppStore(s => s.config);
  const apps = config?.apps ?? [];
  const busy = useAppStore(s => s.busy);
  const detection = useAppStore(s => s.detection);
  const clearDetection = useAppStore(s => s.clearDetection);
  const scanApps = useAppStore(s => s.scanApps);
  const pickAndDetect = useAppStore(s => s.pickAndDetect);

  const managedDir = config?.settings.managed_dir?.trim() ?? "";
  const isConfigured = managedDir.length > 0;

  return (
    <section className="glass-panel" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <p className="description">{t("scan.description")}</p>

      <div className="managed-path-badge" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>
          📁 <strong>Thư mục Portable:</strong> {isConfigured ? managedDir : "Chưa cấu hình (Ứng dụng hệ thống vẫn tự động quét)"}
        </span>
        <Link to="/config" className="btn btn-sm btn-ghost" style={{ textDecoration: "none", fontSize: "0.8rem" }}>
          Đổi thư mục →
        </Link>
      </div>

      <div className="row" style={{ margin: "0.25rem 0" }}>
        <button
          className="btn btn-primary"
          disabled={busy}
          onClick={() => void scanApps(false)}
        >
          {busy ? "…" : "⟳ Quét lại toàn bộ ứng dụng"}
        </button>
        <button className="btn btn-ghost" disabled={busy} onClick={() => void pickAndDetect()}>
          {t("scan.detect_btn")}
        </button>
        {detection && (
          <button className="btn btn-ghost" onClick={clearDetection}>
            ✕ {t("manager.stop") ?? "Đóng kiểm tra"}
          </button>
        )}
      </div>

      {detection ? (
        <DetectionReport report={detection} />
      ) : (
        <div>
          {apps.length > 0 && (
            <p style={{ fontSize: "0.9rem", color: "var(--panel-foreground)", marginBottom: "0.5rem" }}>
              {t("dashboard.managed_apps")}: <strong>{apps.length}</strong>
            </p>
          )}
          <AppTable apps={apps} />
        </div>
      )}
    </section>
  );
}
