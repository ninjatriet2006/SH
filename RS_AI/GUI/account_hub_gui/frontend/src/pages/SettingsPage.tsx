import React from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { Settings as SettingsIcon, Globe, Palette } from 'lucide-react';

export const SettingsPage: React.FC = () => {
  const { data, saveSettings } = useAppStore();
  const { lang, setLang } = useTranslation();

  if (!data) return null;

  const handleLangChange = (newLang: string) => {
    setLang(newLang);
    saveSettings({
      ...data.settings,
      lang: newLang,
    });
  };

  const handleThemeChange = (newTheme: string) => {
    saveSettings({
      ...data.settings,
      theme: newTheme,
    });
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', maxWidth: '700px' }}>
      <div>
        <h1 style={{ fontSize: '1.8rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
          <SettingsIcon size={24} color="var(--primary)" /> Cài đặt Ứng dụng
        </h1>
        <p style={{ color: 'var(--text-secondary)' }}>Tùy chỉnh ngôn ngữ hiển thị, giao diện và cấu hình hệ thống</p>
      </div>

      <div className="glass-card" style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
        <div className="form-group" style={{ marginBottom: 0 }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Globe size={16} /> Ngôn ngữ (Language)
          </label>
          <select
            className="form-control"
            value={lang}
            onChange={e => handleLangChange(e.target.value)}
          >
            <option value="vi">Tiếng Việt</option>
            <option value="en">English</option>
          </select>
        </div>

        <div className="form-group" style={{ marginBottom: 0 }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Palette size={16} /> Chủ đề giao diện (Theme)
          </label>
          <select
            className="form-control"
            value={data.settings.theme}
            onChange={e => handleThemeChange(e.target.value)}
          >
            <option value="default">Default Dark</option>
            <option value="nordic">Nordic Frost</option>
          </select>
        </div>
      </div>

      <div className="glass-card">
        <h3 style={{ marginBottom: '0.5rem' }}>Thông tin Project</h3>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
          <strong>Tên ứng dụng:</strong> Account Hub GUI<br />
          <strong>Nền tảng:</strong> Tauri v2 (Rust Backend + React 19 Frontend)<br />
          <strong>Lưu trữ:</strong> Local Persistent Storage JSON
        </p>
      </div>
    </div>
  );
};
