import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";

export function DashboardPage() {
  const { t } = useTranslation();
  const config = useAppStore(s => s.config);
  const apps = config?.apps ?? [];

  return (
    <section className="cards">
      <article className="glass-panel card">
        <strong>{apps.length}</strong>
        <span>{t("dashboard.managed_apps")}</span>
      </article>
      <article className="glass-panel card">
        <strong>{apps.filter(a => a.is_custom).length}</strong>
        <span>{t("dashboard.custom_apps")}</span>
      </article>
      <article className="glass-panel card">
        <strong className="break-word">{config?.settings.managed_dir || t("dashboard.not_configured")}</strong>
        <span>{t("dashboard.managed_dir")}</span>
      </article>
    </section>
  );
}
