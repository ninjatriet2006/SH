/*
[INTEGRITY NOTES]
- Mục đích: Trang cài đặt — ngôn ngữ, theme, và đường dẫn file cấu hình.
- Trách nhiệm: Cho chọn ngôn ngữ/theme (preview theme ngay), lưu xuống backend,
  và hiển thị app đang đọc/ghi vào file nào.
- Tương tác: `store/useSettingsStore.ts`, `store/useThemeStore.ts`.
*/

import React, { useEffect, useState } from 'react';
import { Settings as SettingsIcon, Globe, Palette, Save, FolderOpen } from 'lucide-react';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useTranslation, langLabel } from '../utils/i18n';

export function SettingsPage() {
    const { t } = useTranslation();
    const { language, theme_id, availableLangs, updateSettings, paths, fetchPaths } = useSettingsStore();
    const { themes, setActiveTheme } = useThemeStore();

    const [localLang, setLocalLang] = useState(language);
    const [localTheme, setLocalTheme] = useState(theme_id);
    const [message, setMessage] = useState('');
    const [isSaving, setIsSaving] = useState(false);

    // Đồng bộ form theo store khi `initSettings()` async hoàn tất — nếu chỉ khởi
    // tạo state một lần, form sẽ mãi hiện giá trị mặc định cũ.
    useEffect(() => { setLocalLang(language); }, [language]);
    useEffect(() => { setLocalTheme(theme_id); }, [theme_id]);
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
            await updateSettings(localLang, localTheme);
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
                    <label className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Globe size={16} /> {t('settings.lbl_lang')}
                    </label>
                    <select
                        className="input-field"
                        value={localLang}
                        onChange={e => setLocalLang(e.target.value)}
                    >
                        {availableLangs.length === 0 && (
                            <option value="">{t('settings.no_lang_file')}</option>
                        )}
                        {availableLangs.map(l => (
                            <option key={l} value={l}>{langLabel(l)}</option>
                        ))}
                    </select>
                </div>

                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label className="form-label" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Palette size={16} /> {t('settings.lbl_theme')}
                    </label>
                    <select
                        className="input-field"
                        value={localTheme}
                        onChange={e => {
                            setLocalTheme(e.target.value);
                            setActiveTheme(e.target.value); // preview ngay
                        }}
                    >
                        {themes.map(th => (
                            <option key={th.id} value={th.id}>{th.name} ({th.type})</option>
                        ))}
                    </select>
                </div>

                <button type="submit" className="btn btn-primary" disabled={isSaving} style={{ width: '100%', justifyContent: 'center' }}>
                    <Save size={18} /> {isSaving ? t('common.loading') : t('common.save')}
                </button>
            </form>

            <div className="glass-panel" style={{ marginTop: '1.5rem' }}>
                <h3 style={{ marginTop: 0, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FolderOpen size={18} /> {t('settings.paths_title')}
                </h3>
                {/* Cho người dùng biết app đang đọc/ghi vào đâu — thiết yếu khi họ
                    sửa file bằng tay hoặc dùng song song với bản TUI. */}
                <div style={{ fontSize: '0.85rem' }}>
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
                </div>
            </div>
        </div>
    );
}
