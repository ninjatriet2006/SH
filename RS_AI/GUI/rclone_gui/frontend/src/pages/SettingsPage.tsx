/*
[INTEGRITY NOTES]
- Mục đích: Trang Cài đặt hệ thống, Giao diện, và Chẩn đoán (Settings View).
- Trách nhiệm: Tuỳ biến theme/font/ngôn ngữ, hiệu chỉnh cờ engine rclone, quản lý snapshot, đọc file backend.log theo yêu cầu.
- Sửa lỗi tuỳ biến giao diện:
  1. Theme: Hiển thị đúng màu từ biến token `colors-neon-cyan` và `colors-surface-canvas`.
  2. Font: Hiển thị xem trước trực tiếp (Live Preview) font chữ và hỗ trợ nạp font local qua @font-face.
  3. Language: Chuẩn hoá 100% qua `useTranslation()` và `data-lang-id`.
- Tương tác: Dùng `useAppearanceStore`, `useSettingsStore`, và `useTranslation()`.
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
  Trash2,
} from 'lucide-react';
import React, { useEffect, useRef, useState } from 'react';
import type { EngineSettings } from '../../../bridge/types';
import { useAppearanceStore } from '../store/useAppearanceStore';
import { useSettingsStore } from '../store/useSettingsStore';
import { useTranslation } from '../utils/i18n';

export const SettingsPage: React.FC = () => {
  const { t } = useTranslation();
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
  const clearLog = useSettingsStore((state) => state.clearLog);
  const restoreSnapshot = useSettingsStore((state) => state.restoreSnapshot);

  const logPreRef = useRef<HTMLPreElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);

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

  useEffect(() => {
    if (autoScroll && logPreRef.current) {
      logPreRef.current.scrollTop = logPreRef.current.scrollHeight;
    }
  }, [backendLog, autoScroll]);

  useEffect(() => {
    if (activeTab !== 'logs' || !autoRefresh) return;
    const timer = setInterval(() => {
      fetchBackendLog();
    }, 2000);
    return () => clearInterval(timer);
  }, [activeTab, autoRefresh, fetchBackendLog]);

  const handleClearLog = async () => {
    if (confirm('Bạn có chắc muốn xoá toàn bộ nội dung nhật ký backend.log?')) {
      await clearLog();
    }
  };

  const handleSaveEngine = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    setSaveMsg(null);
    try {
      await updateEngineFlags(engineForm);
      await updateDebugSettings({ log_rotate_mb: Number(logRotateMb) || 10 });
      setSaveMsg(t('settings_save_success', 'Đã lưu cấu hình thành công!'));
    } catch (err) {
      setSaveMsg(`Lỗi lưu cấu hình: ${String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRestoreSnapshot = async (name: string) => {
    const confirmPrompt = t(
      'settings_snapshot_confirm',
      `Bạn có chắc muốn khôi phục cấu hình từ bản sao lưu "${name}"?`,
    );
    if (confirm(confirmPrompt)) {
      try {
        await restoreSnapshot(name);
        alert(t('settings_snapshot_success', 'Khôi phục snapshot thành công!'));
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
          <span data-lang-id="settings_title">{t('settings_title', 'Cài đặt & Chẩn đoán')}</span>
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
            data-lang-id="settings_tab_appearance"
          >
            <Palette size={16} />
            <span>{t('settings_tab_appearance', 'Giao diện & Ngôn ngữ')}</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'engine' ? 'active' : ''}`}
            onClick={() => setActiveTab('engine')}
            data-lang-id="settings_tab_engine"
          >
            <Cpu size={16} />
            <span>{t('settings_tab_engine', 'Cờ Engine Rclone')}</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'snapshots' ? 'active' : ''}`}
            onClick={() => setActiveTab('snapshots')}
            data-lang-id="settings_tab_snapshots"
          >
            <Database size={16} />
            <span>{t('settings_tab_snapshots', 'Bản sao lưu rclone.conf')}</span>
          </button>

          <button
            className={`nav-link ${activeTab === 'logs' ? 'active' : ''}`}
            onClick={() => setActiveTab('logs')}
            data-lang-id="settings_tab_logs"
          >
            <Terminal size={16} />
            <span>{t('settings_tab_logs', 'Nhật ký Backend')}</span>
          </button>
        </div>

        {/* Tab Content */}
        <div className="glass-panel" style={{ flex: 1, padding: '1.25rem', overflowY: 'auto' }}>
          {/* TAB 1: Giao diện */}
          {activeTab === 'appearance' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem', maxWidth: '640px' }}>
              <h3 style={{ fontSize: '1.1rem', margin: 0 }} data-lang-id="settings_appearance_title">
                {t('settings_appearance_title', 'Tuỳ biến Giao diện')}
              </h3>

              {/* Ngôn ngữ */}
              <div className="form-group">
                <label className="form-label" data-lang-id="settings_language">
                  {t('settings_language', 'Ngôn ngữ hiển thị:')}
                </label>
                <select
                  className="input-text"
                  value={appearance.lang}
                  onChange={(e) => changeLanguage(e.target.value)}
                >
                  {availableLangs.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang.toUpperCase()} {lang === 'test5555' ? '(Canary Test 5555)' : ''}
                    </option>
                  ))}
                </select>
              </div>

              {/* Themes */}
              <div className="form-group">
                <label className="form-label" data-lang-id="settings_theme">
                  {t('settings_theme', 'Chủ đề màu sắc (Theme):')}
                </label>
                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
                    gap: '0.75rem',
                  }}
                >
                  {themes.map((th) => {
                    const isCurrent = appearance.theme === th.id;
                    const primaryColor =
                      th.variables?.['colors-neon-cyan'] || th.variables?.['--primary'] || '#6366f1';
                    const bgColor =
                      th.variables?.['colors-surface-canvas'] ||
                      th.variables?.['--bg-dark'] ||
                      '#0f172a';

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
                          <div
                            style={{
                              width: '16px',
                              height: '16px',
                              borderRadius: '50%',
                              background: primaryColor,
                            }}
                          />
                          <div
                            style={{
                              width: '16px',
                              height: '16px',
                              borderRadius: '50%',
                              background: bgColor,
                              border: '1px solid rgba(255,255,255,0.2)',
                            }}
                          />
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
                <label className="form-label" data-lang-id="settings_font">
                  {t('settings_font', 'Phông chữ (Font Family):')}
                </label>
                <select
                  className="input-text"
                  value={appearance.font}
                  onChange={(e) => changeFont(e.target.value)}
                >
                  <option value="system" data-lang-id="settings_font_system">
                    {t('settings_font_system', 'Hệ thống (Inter / Roboto / System)')}
                  </option>
                  {fonts.map((f) => (
                    <option key={f.id} value={f.id}>
                      {f.name} ({f.family})
                    </option>
                  ))}
                </select>
              </div>

              {/* Font Live Preview Box */}
              <div className="form-group">
                <label className="form-label" data-lang-id="settings_font_preview_label">
                  {t('settings_font_preview_label', 'Xem trước phông chữ:')}
                </label>
                <div
                  style={{
                    padding: '1rem',
                    borderRadius: '8px',
                    background: 'var(--bg-input)',
                    border: '1px solid var(--border)',
                    fontFamily: 'var(--font-family-base)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.4rem',
                  }}
                >
                  <div style={{ fontSize: '0.88rem', fontWeight: 600, color: 'var(--primary)' }}>
                    {fonts.find((f) => f.id === appearance.font)?.name || 'Inter / System Font'}
                  </div>
                  <div
                    style={{ fontSize: '0.85rem', color: 'var(--text-primary)', lineHeight: 1.5 }}
                    data-lang-id="settings_font_preview_sample"
                  >
                    {t(
                      'settings_font_preview_sample',
                      'The quick brown fox jumps over the lazy dog 1234567890 — Thử nghiệm tiếng Việt có dấu: Cần Thơ, Đà Nẵng, Hà Nội, TP. Hồ Chí Minh',
                    )}
                  </div>
                </div>
              </div>

              <div
                style={{ fontSize: '0.78rem', color: 'var(--text-muted)', lineHeight: 1.4 }}
                data-lang-id="settings_resource_hint"
              >
                {t(
                  'settings_resource_hint',
                  'Thêm tài nguyên local vào langs/, themes/ hoặc fonts/ rồi khởi động lại để cập nhật danh sách.',
                )}
              </div>
            </div>
          )}

          {/* TAB 2: Cờ Engine */}
          {activeTab === 'engine' && (
            <form
              onSubmit={handleSaveEngine}
              style={{ maxWidth: '640px', display: 'flex', flexDirection: 'column', gap: '1rem' }}
            >
              <h3 style={{ fontSize: '1.1rem', margin: 0 }} data-lang-id="settings_engine_title">
                {t('settings_engine_title', 'Hiệu năng Rclone Engine')}
              </h3>

              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' }}>
                <div className="form-group">
                  <label className="form-label" data-lang-id="settings_engine_queue_concurrency">
                    {t('settings_engine_queue_concurrency', 'Số tác vụ hàng đợi song song:')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={32}
                    className="input-text"
                    value={engineForm.queue_concurrency ?? 4}
                    onChange={(e) => setEngineForm({ ...engineForm, queue_concurrency: Number(e.target.value) })}
                  />
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    {t('settings_engine_queue_concurrency_hint', 'Số tác vụ chạy đồng thời (1..32)')}
                  </span>
                </div>

                <div className="form-group">
                  <label className="form-label" data-lang-id="settings_engine_transfers">
                    {t('settings_engine_transfers', 'Số luồng truyền (--transfers):')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={64}
                    className="input-text"
                    value={engineForm.transfers}
                    onChange={(e) => setEngineForm({ ...engineForm, transfers: Number(e.target.value) })}
                  />
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    {t('settings_engine_transfers_hint', 'Tối đa 64 luồng rclone')}
                  </span>
                </div>

                <div className="form-group">
                  <label className="form-label" data-lang-id="settings_engine_checkers">
                    {t('settings_engine_checkers', 'Số luồng kiểm tra (--checkers):')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={128}
                    className="input-text"
                    value={engineForm.checkers}
                    onChange={(e) => setEngineForm({ ...engineForm, checkers: Number(e.target.value) })}
                  />
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    {t('settings_engine_checkers_hint', 'Tối đa 128 luồng kiểm tra')}
                  </span>
                </div>
              </div>

              <div className="form-group">
                <label className="form-label" data-lang-id="settings_engine_backup_dir">
                  {t('settings_engine_backup_dir', 'Thư mục backup (trống = tắt):')}
                </label>
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
                <label
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    cursor: 'pointer',
                    fontSize: '0.85rem',
                  }}
                  data-lang-id="settings_engine_fast_list"
                >
                  <input
                    type="checkbox"
                    checked={engineForm.fast_list}
                    onChange={(e) => setEngineForm({ ...engineForm, fast_list: e.target.checked })}
                  />
                  <span>{t('settings_engine_fast_list', 'Fast list (liệt kê đệ quy nhanh):')}</span>
                </label>

                <label
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    cursor: 'pointer',
                    fontSize: '0.85rem',
                  }}
                  data-lang-id="settings_engine_across"
                >
                  <input
                    type="checkbox"
                    checked={engineForm.server_side_across}
                    onChange={(e) =>
                      setEngineForm({ ...engineForm, server_side_across: e.target.checked })
                    }
                  />
                  <span>
                    {t('settings_engine_across', 'Server-side xuyên config (cùng hãng):')}
                  </span>
                </label>

                <label
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    cursor: 'pointer',
                    fontSize: '0.85rem',
                  }}
                  data-lang-id="settings_engine_dry_run"
                >
                  <input
                    type="checkbox"
                    checked={engineForm.dry_run}
                    onChange={(e) => setEngineForm({ ...engineForm, dry_run: e.target.checked })}
                  />
                  <span style={{ color: engineForm.dry_run ? 'var(--warning)' : undefined }}>
                    {t('settings_engine_dry_run', 'Chạy thử (dry run):')}
                  </span>
                </label>
              </div>

              <div className="form-group" style={{ marginTop: '0.5rem' }}>
                <label className="form-label" data-lang-id="settings_log_rotate">
                  {t('settings_log_rotate', 'Kích thước xoay vòng file log (MB):')}
                </label>
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
                <button
                  className="btn btn-primary btn-sm"
                  type="submit"
                  disabled={isSaving}
                  data-lang-id="settings_engine_save"
                >
                  <Save size={14} />
                  <span>{isSaving ? 'Đang lưu...' : t('settings_engine_save', 'Lưu Engine')}</span>
                </button>
              </div>
            </form>
          )}

          {/* TAB 3: Snapshots */}
          {activeTab === 'snapshots' && (
            <div style={{ maxWidth: '640px', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <h3 style={{ fontSize: '1.1rem', margin: 0 }} data-lang-id="settings_tab_snapshots">
                {t('settings_tab_snapshots', 'Bản sao lưu rclone.conf')}
              </h3>

              {snapshots.length === 0 ? (
                <div
                  style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}
                  data-lang-id="settings_no_snapshots"
                >
                  {t('settings_no_snapshots', 'Chưa có bản sao lưu cấu hình nào.')}
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
                        <span style={{ fontSize: '0.85rem', fontFamily: 'var(--font-mono)' }}>
                          {snap}
                        </span>
                      </div>
                      <button
                        className="btn btn-secondary btn-sm"
                        onClick={() => handleRestoreSnapshot(snap)}
                        data-lang-id="settings_restore_snapshot"
                      >
                        <RotateCcw size={12} color="#34d399" />
                        <span>{t('settings_restore_snapshot', 'Khôi phục')}</span>
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
                  <h3 style={{ fontSize: '1.1rem', margin: 0 }} data-lang-id="settings_tab_logs">
                    {t('settings_tab_logs', 'Nhật ký Backend (backend.log)')}
                  </h3>
                </div>

                <div style={{ display: 'flex', gap: '0.65rem', alignItems: 'center', flexWrap: 'wrap' }}>
                  <label style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', fontSize: '0.8rem', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                    <input
                      type="checkbox"
                      checked={autoScroll}
                      onChange={(e) => setAutoScroll(e.target.checked)}
                    />
                    <span>Cuộn xuống cuối</span>
                  </label>

                  <label style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', fontSize: '0.8rem', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                    <input
                      type="checkbox"
                      checked={autoRefresh}
                      onChange={(e) => setAutoRefresh(e.target.checked)}
                    />
                    <span>Tự làm mới (2s)</span>
                  </label>

                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => {
                      navigator.clipboard.writeText(backendLog);
                      alert('Đã sao chép toàn bộ log vào Clipboard!');
                    }}
                    disabled={!backendLog}
                    data-lang-id="settings_copy_log"
                  >
                    <Copy size={13} />
                    <span>{t('settings_copy_log', 'Sao chép toàn bộ Log')}</span>
                  </button>

                  <button
                    className="btn btn-primary btn-sm"
                    onClick={() => fetchBackendLog()}
                    data-lang-id="settings_refresh_log"
                  >
                    <RefreshCw size={13} />
                    <span>{t('settings_refresh_log', 'Làm mới Log')}</span>
                  </button>

                  <button
                    className="btn btn-danger btn-sm"
                    onClick={handleClearLog}
                    title="Xoá sạch file log"
                  >
                    <Trash2 size={13} />
                    <span>Xoá Log</span>
                  </button>
                </div>
              </div>

              <pre
                ref={logPreRef}
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
                data-lang-id="settings_log_empty"
              >
                {backendLog || t('settings_log_empty', 'Chưa có log backend hoặc file log trống.')}
              </pre>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
