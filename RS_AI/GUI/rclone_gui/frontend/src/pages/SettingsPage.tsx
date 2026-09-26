/*
[INTEGRITY NOTES]
- Mục đích: Trang Cài đặt hệ thống, Giao diện, và Chẩn đoán (Settings View).
- Trách nhiệm: Tuỳ biến theme/font/ngôn ngữ, hiệu chỉnh cờ engine rclone, quản lý snapshot, đọc file backend.log theo yêu cầu.
- Tương tác: Dùng `useAppearanceStore` và `useSettingsStore`.
*/

import {
  Copy,
  Cpu,
  Database,
  FileCode,
  Palette,
  RefreshCw,
  RotateCcw,
  Save,
  Settings,
  Terminal,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import type { EngineSettings } from '../../../bridge/types';
import { useAppearanceStore } from '../store/useAppearanceStore';
import { useSettingsStore } from '../store/useSettingsStore';

export const SettingsPage: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'appearance' | 'engine' | 'snapshots' | 'logs'>('appearance');

  // Appearance
  const appearance = useAppearanceStore((state) => state.appearance);
  const themes = useAppearanceStore((state) => state.themes);
  const fonts = useAppearanceStore((state) => state.fonts);
  const availableLangs = useAppearanceStore((state) => state.availableLangs);
  const changeLanguage = useAppearanceStore((state) => state.changeLanguage);
  const changeTheme = useAppearanceStore((state) => state.changeTheme);
  const changeFont = useAppearanceStore((state) => state.changeFont);

  // Settings & Engine
  const engineFlags = useSettingsStore((state) => state.engineFlags);
  const debugSettings = useSettingsStore((state) => state.debugSettings);
  const snapshots = useSettingsStore((state) => state.snapshots);
  const backendLog = useSettingsStore((state) => state.backendLog);
  const loadSettings = useSettingsStore((state) => state.loadSettings);
  const updateEngineFlags = useSettingsStore((state) => state.updateEngineFlags);
  const updateDebugSettings = useSettingsStore((state) => state.updateDebugSettings);
  const fetchBackendLog = useSettingsStore((state) => state.fetchBackendLog);
  const restoreSnapshot = useSettingsStore((state) => state.restoreSnapshot);

  // Local form state for Engine
  const [engineForm, setEngineForm] = useState<EngineSettings>(engineFlags);
  const [logRotateMb, setLogRotateMb] = useState(debugSettings.log_rotate_mb);
  const [isSaving, setIsSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const [prevFlags, setPrevFlags] = useState(engineFlags);
  if (prevFlags !== engineFlags) {
    setPrevFlags(engineFlags);
    setEngineForm(engineFlags);
  }

  const [prevDebug, setPrevDebug] = useState(debugSettings);
  if (prevDebug !== debugSettings) {
    setPrevDebug(debugSettings);
    setLogRotateMb(debugSettings.log_rotate_mb);
  }

  useEffect(() => {
    if (activeTab === 'logs') {
      fetchBackendLog();
    }
  }, [activeTab, fetchBackendLog]);

  const handleSaveEngine = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    setSaveMsg(null);
    try {
      await updateEngineFlags(engineForm);
      await updateDebugSettings({ log_rotate_mb: Number(logRotateMb) || 10 });
      setSaveMsg('Đã lưu cấu hình thành công!');
    } catch (err) {
      setSaveMsg(`Lỗi lưu cấu hình: ${String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRestoreSnapshot = async (name: string) => {
    if (confirm(`Bạn có chắc muốn khôi phục cấu hình từ bản sao lưu "${name}"?`)) {
      try {
        await restoreSnapshot(name);
        alert('Khôi phục snapshot thành công!');
      } catch (err) {
        alert(`Lỗi khôi phục snapshot: ${String(err)}`);
      }
    }
  };

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="page-title">
          <Settings size={24} color="#a5b4fc" />
          <span>Cài đặt & Chẩn đoán</span>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '1rem', flex: 1, overflow: 'hidden' }}>
        {/* Navigation Tabs on Left */}
        <div
          className="glass-panel"
          style={{ width: '220px', padding: '0.75rem', gap: '0.35rem', flexShrink: 0 }}
        >
          <button
            className={`nav-link ${activeTab === 'appearance' ? 'active' : ''}`}
            onClick={() => setActiveTab('appearance')}
          >
            <Palette size={16} />
            <span>Giao diện & Ngôn ngữ</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'engine' ? 'active' : ''}`}
            onClick={() => setActiveTab('engine')}
          >
            <Cpu size={16} />
            <span>Cờ Engine Rclone</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'snapshots' ? 'active' : ''}`}
            onClick={() => setActiveTab('snapshots')}
          >
            <Database size={16} />
            <span>Bản sao lưu rclone.conf</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'logs' ? 'active' : ''}`}
            onClick={() => setActiveTab('logs')}
          >
            <Terminal size={16} />
            <span>Nhật ký Backend</span>
          </button>
        </div>

        {/* Tab Content */}
        <div className="glass-panel" style={{ flex: 1, padding: '1.25rem', overflowY: 'auto' }}>
          {/* TAB 1: Giao diện */}
          {activeTab === 'appearance' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem', maxWidth: '640px' }}>
              <h3 style={{ fontSize: '1.1rem', margin: 0 }}>Tuỳ biến Giao diện</h3>

              {/* Ngôn ngữ */}
              <div className="form-group">
                <label className="form-label">Ngôn ngữ hiển thị:</label>
                <select
                  className="input-text"
                  value={appearance.lang}
                  onChange={(e) => changeLanguage(e.target.value)}
                >
                  {availableLangs.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang.toUpperCase()}
                    </option>
                  ))}
                </select>
              </div>

              {/* Themes */}
              <div className="form-group">
                <label className="form-label">Chủ đề màu sắc (Theme):</label>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))', gap: '0.75rem' }}>
                  {themes.map((th) => {
                    const isCurrent = appearance.theme === th.id;
                    const primaryColor = th.variables?.['--primary'] || '#6366f1';
                    const bgColor = th.variables?.['--bg-dark'] || '#0f172a';

                    return (
                      <div
                        key={th.id}
                        style={{
                          padding: '0.75rem',
                          borderRadius: '8px',
                          border: `2px solid ${isCurrent ? 'var(--primary)' : 'var(--border)'}`,
                          background: 'rgba(0,0,0,0.3)',
                          cursor: 'pointer',
                          display: 'flex',
                          flexDirection: 'column',
                          gap: '0.45rem',
                        }}
                        onClick={() => changeTheme(th.id)}
                      >
                        <div style={{ display: 'flex', gap: '0.35rem' }}>
                          <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: primaryColor }} />
                          <div style={{ width: '16px', height: '16px', borderRadius: '50%', background: bgColor, border: '1px solid rgba(255,255,255,0.2)' }} />
                        </div>
                        <span style={{ fontSize: '0.85rem', fontWeight: isCurrent ? 600 : 400 }}>
                          {th.name || th.id}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Fonts */}
              <div className="form-group">
                <label className="form-label">Phông chữ (Font Family):</label>
                <select
                  className="input-text"
                  value={appearance.font}
                  onChange={(e) => changeFont(e.target.value)}
                >
                  <option value="system">Hệ thống (Inter / Roboto / System)</option>
                  {fonts.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.name} ({f.family})
                    </option>
                  ))}
                </select>
              </div>
            </div>
          )}

          {/* TAB 2: Cờ Engine */}
          {activeTab === 'engine' && (
            <form onSubmit={handleSaveEngine} style={{ maxWidth: '640px', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <h3 style={{ fontSize: '1.1rem', margin: 0 }}>Hiệu năng Rclone Engine</h3>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                <div className="form-group">
                  <label className="form-label">Số luồng truyền song song (--transfers):</label>
                  <input
                    type="number"
                    min={1}
                    max={64}
                    className="input-text"
                    value={engineForm.transfers}
                    onChange={(e) => setEngineForm({ ...engineForm, transfers: Number(e.target.value) })}
                  />
                </div>

                <div className="form-group">
                  <label className="form-label">Số luồng kiểm tra mã băm (--checkers):</label>
                  <input
                    type="number"
                    min={1}
                    max={128}
                    className="input-text"
                    value={engineForm.checkers}
                    onChange={(e) => setEngineForm({ ...engineForm, checkers: Number(e.target.value) })}
                  />
                </div>
              </div>

              <div className="form-group">
                <label className="form-label">Thư mục sao lưu (--backup-dir):</label>
                <input
                  type="text"
                  className="input-text"
                  placeholder="Để trống nếu không dùng"
                  value={engineForm.backup_dir || ''}
                  onChange={(e) =>
                    setEngineForm({ ...engineForm, backup_dir: e.target.value.trim() || null })
                  }
                />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.65rem' }}>
                <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.85rem' }}>
                  <input
                    type="checkbox"
                    checked={engineForm.fast_list}
                    onChange={(e) => setEngineForm({ ...engineForm, fast_list: e.target.checked })}
                  />
                  <span>Bật --fast-list (Giảm request API lên Cloud, tốn RAM hơn)</span>
                </label>

                <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.85rem' }}>
                  <input
                    type="checkbox"
                    checked={engineForm.server_side_across}
                    onChange={(e) =>
                      setEngineForm({ ...engineForm, server_side_across: e.target.checked })
                    }
                  />
                  <span>Bật --server-side-across-configs (Copy server-side giữa các tài khoản cùng provider)</span>
                </label>

                <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.85rem' }}>
                  <input
                    type="checkbox"
                    checked={engineForm.dry_run}
                    onChange={(e) => setEngineForm({ ...engineForm, dry_run: e.target.checked })}
                  />
                  <span style={{ color: engineForm.dry_run ? 'var(--warning)' : undefined }}>
                    Bật --dry-run (Chạy thử nghiệm không thực sự copy/xoá dữ liệu)
                  </span>
                </label>
              </div>

              <div className="form-group" style={{ marginTop: '0.5rem' }}>
                <label className="form-label">Kích thước xoay vòng file log (MB):</label>
                <input
                  type="number"
                  min={1}
                  max={100}
                  className="input-text"
                  style={{ width: '140px' }}
                  value={logRotateMb}
                  onChange={(e) => setLogRotateMb(Number(e.target.value))}
                />
              </div>

              {saveMsg && (
                <div
                  style={{
                    padding: '0.5rem 0.75rem',
                    borderRadius: '6px',
                    fontSize: '0.85rem',
                    background: saveMsg.startsWith('Lỗi')
                      ? 'rgba(239, 68, 68, 0.15)'
                      : 'rgba(16, 185, 129, 0.15)',
                    color: saveMsg.startsWith('Lỗi') ? '#fca5a5' : '#6ee7b7',
                  }}
                >
                  {saveMsg}
                </div>
              )}

              <div>
                <button className="btn btn-primary btn-sm" type="submit" disabled={isSaving}>
                  <Save size={14} />
                  <span>{isSaving ? 'Đang lưu...' : 'Lưu thiết lập'}</span>
                </button>
              </div>
            </form>
          )}

          {/* TAB 3: Snapshots */}
          {activeTab === 'snapshots' && (
            <div style={{ maxWidth: '640px', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <h3 style={{ fontSize: '1.1rem', margin: 0 }}>Các bản sao lưu cấu hình (Snapshots)</h3>
              <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                Rclone GUI tự động tạo bản sao lưu rclone.conf mỗi khi bạn thêm, sửa hoặc xoá remote.
              </p>

              {snapshots.length === 0 ? (
                <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                  Chưa có bản sao lưu nào.
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                  {snapshots.map((snap) => (
                    <div
                      key={snap}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '0.65rem 0.85rem',
                        background: 'rgba(0,0,0,0.25)',
                        border: '1px solid var(--border)',
                        borderRadius: '8px',
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <FileCode size={16} color="#818cf8" />
                        <span style={{ fontSize: '0.85rem', fontFamily: 'var(--font-mono)' }}>{snap}</span>
                      </div>
                      <button
                        className="btn btn-secondary btn-sm"
                        onClick={() => handleRestoreSnapshot(snap)}
                      >
                        <RotateCcw size={12} color="#34d399" />
                        <span>Khôi phục</span>
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* TAB 4: Logs (Pull Model - On-demand) */}
          {activeTab === 'logs' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem', height: '100%' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                  <h3 style={{ fontSize: '1.1rem', margin: 0 }}>Nhật ký Backend (backend.log)</h3>
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    Mô hình Pull theo yêu cầu — an toàn tài nguyên, không spam luồng sự kiện.
                  </span>
                </div>

                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => {
                      navigator.clipboard.writeText(backendLog);
                      alert('Đã sao chép toàn bộ log vào Clipboard!');
                    }}
                    disabled={!backendLog}
                  >
                    <Copy size={13} />
                    <span>Sao chép</span>
                  </button>

                  <button className="btn btn-primary btn-sm" onClick={() => fetchBackendLog()}>
                    <RefreshCw size={13} />
                    <span>Tải lại</span>
                  </button>
                </div>
              </div>

              <pre
                style={{
                  flex: 1,
                  background: 'rgba(0,0,0,0.5)',
                  border: '1px solid var(--border)',
                  borderRadius: '8px',
                  padding: '1rem',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.78rem',
                  color: '#94a3b8',
                  overflow: 'auto',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                }}
              >
                {backendLog || '(Chưa có dữ liệu log hoặc file log rỗng)'}
              </pre>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
