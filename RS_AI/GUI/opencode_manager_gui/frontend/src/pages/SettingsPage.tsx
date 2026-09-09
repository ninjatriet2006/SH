/*
[INTEGRITY NOTES]
 - Mục đích: Trang cài đặt — ngôn ngữ, theme, font và đường dẫn file cấu hình.
 - Trách nhiệm: Cho chọn ngôn ngữ/theme/font (preview ngay), lưu xuống backend,
  và hiển thị app đang đọc/ghi vào file nào.
- Tương tác: `store/useSettingsStore.ts`, `store/useThemeStore.ts`.
*/

import React, { useEffect, useState } from 'react';
import { Settings as SettingsIcon, Globe, Palette, Save, FolderOpen, Type } from 'lucide-react';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useFontStore } from '../store/useFontStore';
import { useTranslation, langLabel } from '../utils/i18n';

export function SettingsPage() {
    const { t } = useTranslation();
    const { language, theme_id, font_id, availableLangs, updateSettings, paths, fetchPaths } = useSettingsStore();
    const { themes, setActiveTheme } = useThemeStore();
    const { fonts, setActiveFont } = useFontStore();

    const [localLang, setLocalLang] = useState(language);
    const [localTheme, setLocalTheme] = useState(theme_id);
    const [localFont, setLocalFont] = useState(font_id);
    const [message, setMessage] = useState('');
    const [isSaving, setIsSaving] = useState(false);

    // Đồng bộ form theo store khi `initSettings()` async hoàn tất — nếu chỉ khởi
    // tạo state một lần, form sẽ mãi hiện giá trị mặc định cũ.
    useEffect(() => { setLocalLang(language); }, [language]);
    useEffect(() => { setLocalTheme(theme_id); }, [theme_id]);
    useEffect(() => { setLocalFont(font_id); }, [font_id]);
    useEffect(() => { fetchPaths(); }, [fetchPaths]);

    // Dọn timer khi unmount để không setState trên component đã tháo.
    useEffect(() => {
        if (!message) return;
        const timer = setTimeout(() => setMessage(''), 3000);
        return () => clearTimeout(timer);
    }, [message]);

    const handleSave = async (e: React.FormEvent) => {
        e.preventDefault();
        setIsSaving(true);
        try {
            await updateSettings(localLang, localTheme, localFont);
            setActiveTheme(localTheme);
            setMessage(t('settings.save_success'));
        } catch (err) {
            alert(`Lưu cài đặt thất bại: ${err instanceof Error ? err.message : String(err)}`);
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div className="animate-fade-in" style={{ maxWidth: '640px' }}>
            <h1 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <SettingsIcon size={24} /> {t('settings.title')}
            </h1>

            {message && (
                <div style={{ padding: '0.75rem 1rem', background: 'rgba(16,185,129,0.15)', borderLeft: '4px solid var(--success)', borderRadius: '4px', margin: '1rem 0' }}>
                    {message}
                </div>
            )}

            <form onSubmit={handleSave} className="glass-panel" style={{ marginTop: '1.5rem' }}>
                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label htmlFor="settings-language" className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Globe size={16} /> {t('settings.lbl_lang')}
                    </label>
                    <select
                        id="settings-language"
                        className="input-field"
                        value={localLang}
                        onChange={event => setLocalLang(event.target.value)}
                        disabled={availableLangs.length === 0}
                    >
                        {availableLangs.length === 0 && <option value="">{t('settings.no_lang_file')}</option>}
                        {availableLangs.map(lang => <option key={lang} value={lang}>{langLabel(lang)}</option>)}
                    </select>
                </div>

                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label htmlFor="settings-theme" className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Palette size={16} /> {t('settings.lbl_theme')}
                    </label>
                    <select
                        id="settings-theme"
                        className="input-field"
                        value={localTheme}
                        onChange={event => {
                            const v = event.target.value;
                            setLocalTheme(v);
                            setActiveTheme(v); // preview ngay
                        }}
                    >
                        {themes.map(theme => <option key={theme.id} value={theme.id}>{theme.name} ({theme.type})</option>)}
                    </select>
                </div>

                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label htmlFor="settings-font" className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Type size={16} /> {t('settings.lbl_font')}
                    </label>
                    <select
                        id="settings-font"
                        className="input-field"
                        value={localFont}
                        onChange={event => {
                            const value = event.target.value;
                            setLocalFont(value);
                            setActiveFont(value);
                        }}
                    >
                        {fonts.map(font => <option key={font.id} value={font.id}>{font.name}</option>)}
                    </select>
                    <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                        {t('settings.font_hint')}
                    </small>
                </div>

                <button type="submit" className="btn btn-primary" disabled={isSaving} style={{ width: '100%', justifyContent: 'center' }}>
                    <Save size={18} /> {isSaving ? t('common.loading') : t('common.save')}
                </button>
            </form>

            <div className="glass-panel" style={{ marginTop: '1.5rem' }}>
                <h3 style={{ marginTop: 0, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FolderOpen size={18} /> {t('settings.paths_title')}
                </h3>
                {/* Phân biệt rõ file OpenCode sở hữu với trạng thái riêng của Manager,
                    để sửa tay hoặc dùng đồng thời với TUI không nhầm thư mục. */}
                <div style={{ fontSize: '0.85rem' }}>
                    <div style={{ marginBottom: '0.5rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.dir_opencode')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.opencode_config_dir ?? '—'}
                        </code>
                    </div>
                    <div style={{ marginBottom: '0.5rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.dir_manager')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.manager_config_dir ?? '—'}
                        </code>
                    </div>
                    <div style={{ marginBottom: '0.5rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.path_opencode')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.opencode_json ?? '—'}
                        </code>
                    </div>
                    <div>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.path_auth')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.auth_json ?? '—'}
                        </code>
                    </div>
                    <div style={{ marginTop: '0.5rem', marginBottom: '0.5rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.path_ckey')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.ckey_json ?? '—'}
                        </code>
                    </div>
                    <div style={{ marginBottom: '0.5rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.path_arbiter')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.arbiter_json ?? '—'}
                        </code>
                    </div>
                    <div>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('settings.path_manager_settings')}: </span>
                        <code style={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
                            {paths?.settings_json ?? '—'}
                        </code>
                    </div>
                </div>
            </div>
        </div>
    );
}
