import { useState } from "react";
import { useSettingsStore } from "../store/useSettingsStore";
import { useTranslation } from "../utils/i18n";
import type { FontId, Language, Preferences, Theme } from "../utils/contract";

export function SettingsPage() {
  const { t } = useTranslation();
  const preferences = useSettingsStore(s => s.preferences);
  const updateSettings = useSettingsStore(s => s.updateSettings);
  const previewTheme = useSettingsStore(s => s.previewTheme);
  const previewLanguage = useSettingsStore(s => s.previewLanguage);

  const [saving, setSaving] = useState(false);
  const [savedBadge, setSavedBadge] = useState(false);

  const handleThemeChange = (nextTheme: Theme) => {
    void previewTheme(nextTheme, preferences.font_id);
  };

  const handleLanguageChange = (nextLang: Language) => {
    void previewLanguage(nextLang);
  };

  const handleFontChange = (nextFont: FontId) => {
    void previewTheme(preferences.theme, nextFont);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const next: Preferences = {
        language: preferences.language,
        theme: preferences.theme,
        font_id: preferences.font_id,
      };
      await updateSettings(next);
      setSavedBadge(true);
      setTimeout(() => setSavedBadge(false), 3000);
    } catch {
      /* error already logged in store */
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="glass-panel form-grid">
      <label className="field-label">
        {t("settings.language")}
        <select className="input" value={preferences.language} onChange={e => handleLanguageChange(e.target.value as Language)}>
          <option value="vi">Tiếng Việt</option>
          <option value="en">English</option>
        </select>
      </label>

      <label className="field-label">
        {t("settings.theme")}
        <select className="input" value={preferences.theme} onChange={e => handleThemeChange(e.target.value as Theme)}>
          <option value="system">{t("settings.system")}</option>
          <option value="dark">{t("settings.dark")}</option>
          <option value="light">{t("settings.light")}</option>
        </select>
      </label>

      <label className="field-label">
        {t("settings.font")}
        <select className="input" value={preferences.font_id} onChange={e => handleFontChange(e.target.value as FontId)}>
          <option value="system-default">{t("settings.system_default")}</option>
          <option value="dejavusans">DejaVu Sans</option>
        </select>
      </label>

      <div style={{ display: "flex", alignItems: "center", gap: "1rem", marginTop: "0.5rem" }}>
        <button
          className="btn btn-primary"
          disabled={saving}
          onClick={() => void handleSave()}
        >
          {saving ? "…" : t("config.save")}
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
