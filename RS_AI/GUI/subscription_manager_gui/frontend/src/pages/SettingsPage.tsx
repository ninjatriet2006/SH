/*
[INTEGRITY NOTES]
- Mục đích: Trang quản lý cài đặt hệ thống (Ngôn ngữ, Múi giờ).
- Trách nhiệm: Cho phép người dùng cấu hình ứng dụng.
- Tương tác: Kết nối với `useSettingsStore` và i18n hook.
*/

import React, { useState } from 'react';
import { useSettingsStore } from '../store/useSettingsStore';
import { useTranslation } from '../utils/i18n';
import { Settings as SettingsIcon, Globe, Clock, Save, Palette, Type } from 'lucide-react';
import { useThemeStore } from '../store/useThemeStore';
import { useFontStore } from '../store/useFontStore';

export const SettingsPage: React.FC = () => {
    const { t } = useTranslation();
    const { language, timezone, theme_id, font_id, availableLangs, updateSettings } = useSettingsStore();
    const { themes, setActiveTheme } = useThemeStore();
    const { fonts, setActiveFont } = useFontStore();
    
    const [localLang, setLocalLang] = useState(language);
    const [localTz, setLocalTz] = useState(timezone);
    const [localTheme, setLocalTheme] = useState(theme_id);
    const [localFont, setLocalFont] = useState(font_id);
    const [message, setMessage] = useState('');
    const [isSaving, setIsSaving] = useState(false);

    const timezones = [
        "Asia/Ho_Chi_Minh",
        "Asia/Bangkok",
        "Asia/Tokyo",
        "Asia/Seoul",
        "Europe/London",
        "America/New_York",
        "America/Los_Angeles",
        "UTC"
    ];

    const handleSave = async () => {
        setIsSaving(true);
        await updateSettings(localLang, localTz, localTheme, localFont);
        setActiveTheme(localTheme);
        setActiveFont(localFont);
        setMessage(t('settings.save_success'));
        setIsSaving(false);
        
        // Tắt thông báo sau 3s
        setTimeout(() => setMessage(''), 3000);
    };

    return (
        <div className="animate-fade-in" style={{ maxWidth: '600px', margin: '0 auto' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '2rem' }}>
                <SettingsIcon size={28} color="var(--primary)" />
                <h1>{t('settings.title')}</h1>
            </div>

            {message && (
                <div style={{ padding: '1rem', background: 'rgba(52, 211, 153, 0.2)', borderLeft: '4px solid var(--success-color)', marginBottom: '1.5rem', borderRadius: '4px' }}>
                    {message}
                </div>
            )}

            <div style={{ background: 'var(--bg-panel)', padding: '2rem', borderRadius: '12px', border: '1px solid var(--border)' }}>
                {/* Chọn Ngôn ngữ */}
                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Globe size={18} /> {t('settings.lbl_lang') || t('settings.language')}
                    </label>
                    <select 
                        className="form-control" 
                        value={localLang}
                        onChange={(e) => setLocalLang(e.target.value)}
                    >
                        {availableLangs.map(l => (
                            <option key={l} value={l}>
                                {l === 'vi' ? 'Tiếng Việt (vi)' : l === 'en' ? 'English (en)' : l}
                            </option>
                        ))}
                    </select>
                </div>

                {/* Chọn Múi giờ */}
                <div className="form-group" style={{ marginBottom: '2rem' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Clock size={18} /> {t('settings.lbl_tz') || t('settings.timezone')}
                    </label>
                    <select 
                        className="form-control" 
                        value={localTz}
                        onChange={(e) => setLocalTz(e.target.value)}
                    >
                        {timezones.map(tz => (
                            <option key={tz} value={tz}>{tz}</option>
                        ))}
                    </select>
                </div>

                {/* Chọn Theme */}
                <div className="form-group" style={{ marginBottom: '1.5rem' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Palette size={18} /> Giao diện (Theme)
                    </label>
                    <select 
                        className="form-control" 
                        value={localTheme}
                        onChange={(e) => {
                            setLocalTheme(e.target.value);
                            setActiveTheme(e.target.value); // Preview ngay
                        }}
                    >
                        {themes.map(t => (
                            <option key={t.id} value={t.id}>
                                {t.name} ({t.type})
                            </option>
                        ))}
                    </select>
                </div>

                {/* Chọn Font */}
                <div className="form-group" style={{ marginBottom: '2rem' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Type size={18} /> Kiểu chữ (Typography)
                    </label>
                    <select 
                        className="form-control" 
                        value={localFont}
                        onChange={(e) => {
                            setLocalFont(e.target.value);
                            setActiveFont(e.target.value); // Preview ngay
                        }}
                    >
                        {fonts.map(f => (
                            <option key={f.id} value={f.id}>
                                {f.name}
                            </option>
                        ))}
                    </select>
                    <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                        💡 Tải font (.ttf, .otf, .woff2) và thả vào thư mục fonts/ để hiển thị tại đây.
                    </small>
                </div>

                <button 
                    className="btn btn-primary" 
                    onClick={handleSave} 
                    disabled={isSaving}
                    style={{ width: '100%', padding: '0.8rem', fontSize: '1rem', display: 'flex', justifyContent: 'center', gap: '0.5rem' }}
                >
                    <Save size={20} />
                    {isSaving ? t('common.loading') : t('common.save')}
                </button>
            </div>
        </div>
    );
};
