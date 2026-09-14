import { useState } from 'react';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useFontStore } from '../store/useFontStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useTranslation } from '../utils/i18n';

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const { themes, applyTheme } = useThemeStore();
    const { fonts } = useFontStore();
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [saved, setSaved] = useState(false);
    const save = async (next: typeof settings) => {
        try { setSaving(true); setError(null); setSaved(false); await updateSettings(next); setSaved(true); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setSaving(false); }
    };
    return (
        <div>
            <h1>{t('settings.title')}</h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)' }}>{t('settings.description')}</p>
                <div className="settings-grid">
                    <label>Language<select value={settings.language} onChange={(e) => void save({ ...settings, language: e.target.value })} disabled={saving}><option value="en">English</option><option value="vi">Vietnamese</option></select></label>
                    <label>Theme<select value={settings.theme} onChange={(e) => { applyTheme(e.target.value); void save({ ...settings, theme: e.target.value }); }} disabled={saving}>{themes.map((theme) => <option key={theme.id} value={theme.id}>{theme.name}</option>)}</select></label>
                    <label>Font<select value={settings.font} onChange={(e) => void save({ ...settings, font: e.target.value })} disabled={saving}>{fonts.map((font) => <option key={font.id} value={font.id}>{font.name}</option>)}</select></label>
                </div>
                {saving && <p className="status-message">{t('common.loading')}</p>}
                {saved && <p className="success-message">{t('common.save')}</p>}
                {error && <p className="error-message">{error}</p>}
            </div>
        </div>
    );
}
