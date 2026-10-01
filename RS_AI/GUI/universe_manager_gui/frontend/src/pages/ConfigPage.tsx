import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";

export function ConfigPage() {
  const { t } = useTranslation();
  const config = useAppStore(s => s.config);
  const busy = useAppStore(s => s.busy);
  const pickManagedDir = useAppStore(s => s.pickManagedDir);
  const saveConfig = useAppStore(s => s.saveConfig);

  return (
    <section className="glass-panel">
      <label className="field-label">{t("config.managed_dir")}</label>
      <div className="row">
        <input
          id="managed"
          readOnly
          value={config?.settings.managed_dir ?? ""}
          className="input"
        />
        <button className="btn btn-primary" onClick={() => void pickManagedDir()}>
          {t("config.select")}
        </button>
      </div>
      <button
        className="btn btn-primary"
        disabled={!config || busy}
        onClick={() => void saveConfig()}
      >
        {t("config.save")}
      </button>
    </section>
  );
}
