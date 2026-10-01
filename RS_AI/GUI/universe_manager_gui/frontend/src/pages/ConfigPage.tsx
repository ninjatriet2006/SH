import { useState } from "react";
import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";

export function ConfigPage() {
  const { t } = useTranslation();
  const config = useAppStore(s => s.config);
  const busy = useAppStore(s => s.busy);
  const setManagedDir = useAppStore(s => s.setManagedDir);
  const pickManagedDir = useAppStore(s => s.pickManagedDir);
  const saveConfig = useAppStore(s => s.saveConfig);
  const [savedBadge, setSavedBadge] = useState(false);

  const handleSave = async () => {
    await saveConfig();
    if (!useAppStore.getState().error) {
      setSavedBadge(true);
      setTimeout(() => setSavedBadge(false), 3000);
    }
  };

  return (
    <section className="glass-panel">
      <label className="field-label">💼 {t("config.managed_dir")}</label>
      <p style={{ fontSize: "0.85rem", color: "var(--panel-foreground)", marginBottom: "0.6rem", lineHeight: 1.5 }}>
        {t("config.managed_dir_desc") ?? "Thư mục chứa ứng dụng Portable / AppImage tự quản lý (tùy chọn). Các phần mềm cài qua Flatpak, Snap và hệ thống sẽ luôn được tự động phát hiện mà không cần cấu hình thư mục này."}
      </p>
      <div className="row">
        <input
          id="managed"
          value={config?.settings.managed_dir ?? ""}
          onChange={e => setManagedDir(e.target.value)}
          placeholder="/home/username/Applications (tùy chọn)"
          className="input"
        />
        <button className="btn btn-primary" disabled={busy} onClick={() => void pickManagedDir()}>
          {t("config.select")}
        </button>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: "1rem", marginTop: "1rem" }}>
        <button
          className="btn btn-primary"
          disabled={!config || busy}
          onClick={() => void handleSave()}
        >
          {busy ? "…" : t("config.save")}
        </button>
        {savedBadge && (
          <span className="badge-saved">
            ✓ {t("status.saved")}
          </span>
        )}
      </div>
    </section>
  );
}
