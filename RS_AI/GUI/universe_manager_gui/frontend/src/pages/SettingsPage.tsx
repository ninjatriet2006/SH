import { useState } from "react";
import { useSettingsStore } from "../store/useSettingsStore";
import { useTranslation } from "../utils/i18n";
import type { FontId, Language, Preferences, Theme } from "../utils/contract";

export function SettingsPage() {
  const { t } = useTranslation();
  const preferences = useSettingsStore(s => s.preferences);
  const updateSettings = useSettingsStore(s => s.updateSettings);
  const [language, setLanguage] = useState(preferences.language);
  const [theme, setTheme] = useState(preferences.theme);
  const [fontId, setFontId] = useState(preferences.font_id);
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      const next: Preferences = {
        language: language as Preferences["language"],
        theme: theme as Preferences["theme"],
        font_id: fontId as Preferences["font_id"],
      };
      await updateSettings(next);
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
        <select className="input" value={language} onChange={e => setLanguage(e.target.value as Language)}>
          <option value="en">English</option>
          <option value="vi">Tiếng Việt</option>
        </select>
      </label>

      <label className="field-label">
        {t("settings.theme")}
        <select className="input" value={theme} onChange={e => setTheme(e.target.value as Theme)}>
          <option value="system">{t("settings.system")}</option>
          <option value="light">{t("settings.light")}</option>
          <option value="dark">{t("settings.dark")}</option>
        </select>
      </label>

      <label className="field-label">
        {t("settings.font")}
        <select className="input" value={fontId} onChange={e => setFontId(e.target.value as FontId)}>
          <option value="system-default">{t("settings.system_default")}</option>
          <option value="dejavusans">DejaVu Sans</option>
        </select>
      </label>

      <button
        className="btn btn-primary"
        disabled={saving}
        onClick={() => void handleSave()}
      >
        {saving ? "…" : t("config.save")}
      </button>
    </section>
  );
}
