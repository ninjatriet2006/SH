import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";
import { AppTable } from "../components/AppTable";
import { DetectionReport } from "../components/DetectionReport";

export function ScanPage() {
  const { t } = useTranslation();
  const apps = useAppStore(s => s.config?.apps ?? []);
  const busy = useAppStore(s => s.busy);
  const detection = useAppStore(s => s.detection);
  const scanApps = useAppStore(s => s.scanApps);
  const pickAndDetect = useAppStore(s => s.pickAndDetect);

  return (
    <section className="glass-panel">
      <p className="description">{t("scan.description")}</p>
      <div className="row">
        <button className="btn btn-primary" disabled={busy} onClick={() => void scanApps()}>
          {t("scan.scan_btn")}
        </button>
        <button className="btn btn-ghost" disabled={busy} onClick={() => void pickAndDetect()}>
          {t("scan.detect_btn")}
        </button>
      </div>
      {detection ? <DetectionReport report={detection} /> : <AppTable apps={apps} />}
    </section>
  );
}
