import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";
import { AppTable } from "../components/AppTable";

export function AppManagerPage() {
  const { t } = useTranslation();
  const apps = useAppStore(s => s.config?.apps ?? []);
  const busy = useAppStore(s => s.busy);
  const configSaved = useAppStore(s => s.configSaved);
  const startApp = useAppStore(s => s.startApp);
  const stopApp = useAppStore(s => s.stopApp);
  const disabled = busy || !configSaved;

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

  return (
    <section className="glass-panel">
      <AppTable
        apps={apps}
        actions
        disabled={disabled}
        onStart={handleStart}
        onStop={handleStop}
      />
    </section>
  );
}
