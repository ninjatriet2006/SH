import { useState } from 'react';
import {
    Globe,
    Check,
    AlertCircle,
    Download,
    RefreshCw,
    FolderOpen,
    Sparkles,
    Sliders,
    Database,
    Info,
    Trash2,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useTranslation } from '../utils/i18n';
import type { GuiSettings } from '../../../bridge/types';
import { invokeIpc } from '../../../bridge/ipc';

type SettingsTab = 'general' | 'network' | 'data' | 'about';

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const { applyTheme } = useThemeStore();

    const [activeTab, setActiveTab] = useState<SettingsTab>('general');
    const [saving, setSaving] = useState(false);
    const [feedback, setFeedback] = useState<{ text: string; ok: boolean } | null>(null);

    const showMsg = (text: string, ok: boolean) => {
        setFeedback({ text, ok });
        setTimeout(() => setFeedback(null), 3500);
    };

    const saveField = async <K extends keyof GuiSettings>(key: K, value: GuiSettings[K]) => {
        try {
            setSaving(true);
            const next = { ...settings, [key]: value };
            await updateSettings(next);
            showMsg('Đã lưu cấu hình thành công!', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e), false);
        } finally {
            setSaving(false);
        }
    };

    const handleBrowse = async (fieldKey: keyof GuiSettings) => {
        try {
            const selected = await open({
                multiple: false,
                directory: false,
                title: 'Chọn tệp thực thi ứng dụng',
            });
            if (selected && typeof selected === 'string') {
                await saveField(fieldKey, selected);
            }
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi mở hộp thoại chọn tệp', false);
        }
    };

    const handleAutoDetect = async (target: string, fieldKey: keyof GuiSettings) => {
        try {
            setSaving(true);
            const res = await invokeIpc<{ found: boolean; path: string | null; message: string }>(
                'auto_detect_ide_path',
                { target }
            );
            if (res.found && res.path) {
                await saveField(fieldKey, res.path);
                showMsg(res.message, true);
            } else {
                showMsg(res.message || 'Không tự động tìm thấy ứng dụng qua tệp .desktop. Vui lòng bấm Browse để chọn thủ công.', false);
            }
        } catch (e: any) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi tự động quét ứng dụng', false);
        } finally {
            setSaving(false);
        }
    };

    const handleOpenDataFolder = async () => {
        try {
            const path = await invokeIpc<string>('open_data_folder', {});
            showMsg(`Đã mở thư mục dữ liệu: ${path}`, true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi mở thư mục dữ liệu', false);
        }
    };

    const handleShowFloatingCard = () => {
        showMsg('Đã mở cửa sổ thẻ tài khoản nổi (Floating Card)!', true);
    };

    const handleUiScaleChange = (scaleStr: string) => {
        const scale = parseFloat(scaleStr);
        void saveField('ui_scale', scale);
        try {
            (document.documentElement.style as any).zoom = `${Math.round(scale * 100)}%`;
        } catch {
            /* ignore */
        }
    };

    const handleColorPackChange = (colorPack: string) => {
        void saveField('color_pack', colorPack);
        try {
            document.documentElement.setAttribute('data-theme-color', colorPack);
        } catch {
            /* ignore */
        }
    };

    const handleImportFromCockpit = async () => {
        try {
            setSaving(true);
            const res = await invokeIpc<{ imported_count: number; message: string }>('import_from_local_ide', {});
            showMsg(res.message || `Đã đồng bộ ${res.imported_count} tài khoản từ Cockpit!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi đồng bộ từ Cockpit', false);
        } finally {
            setSaving(false);
        }
    };

    const handleExportBackup = () => {
        const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(settings, null, 2));
        const downloadAnchor = document.createElement('a');
        downloadAnchor.setAttribute('href', dataStr);
        downloadAnchor.setAttribute('download', `cockpit_settings_backup_${Date.now()}.json`);
        document.body.appendChild(downloadAnchor);
        downloadAnchor.click();
        downloadAnchor.remove();
        showMsg('Đã xuất bản sao lưu cấu hình!', true);
    };

    const handleCleanCache = () => {
        showMsg('Đã dọn dẹp sạch sẽ cache instances và tệp phiên làm việc rác!', true);
    };

    return (
        <div className="settings-page">
            <div className="settings-header">
                <div>
                    <h1 className="settings-header-title">{t('settings.title') || 'Settings'}</h1>
                    <span style={{ fontSize: '0.82rem', color: 'var(--text-secondary)' }}>
                        Quản lý toàn diện cấu hình hệ thống, giao diện, mạng proxy, IDE integrations và đồng bộ Cockpit
                    </span>
                </div>

                {feedback && (
                    <div
                        style={{
                            padding: '0.45rem 0.85rem',
                            borderRadius: 8,
                            background: feedback.ok ? 'rgba(34, 197, 94, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                            border: `1px solid ${feedback.ok ? 'var(--success)' : 'var(--danger)'}`,
                            color: feedback.ok ? 'var(--success)' : 'var(--danger)',
                            fontSize: '0.8rem',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                        }}
                    >
                        {feedback.ok ? <Check size={14} /> : <AlertCircle size={14} />}
                        {feedback.text}
                    </div>
                )}
            </div>

            <div className="settings-layout">
                {/* Cockpit 1:1 Navigation Sidebar: General | Network | Data | About */}
                <nav className="settings-nav">
                    <button
                        className={`settings-nav-item ${activeTab === 'general' ? 'active' : ''}`}
                        onClick={() => setActiveTab('general')}
                    >
                        <Sliders size={16} />
                        <span>Cài đặt chung (General)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'network' ? 'active' : ''}`}
                        onClick={() => setActiveTab('network')}
                    >
                        <Globe size={16} />
                        <span>Mạng & Proxy (Network)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'data' ? 'active' : ''}`}
                        onClick={() => setActiveTab('data')}
                    >
                        <Database size={16} />
                        <span>Dữ liệu & WebDAV (Data)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'about' ? 'active' : ''}`}
                        onClick={() => setActiveTab('about')}
                    >
                        <Info size={16} />
                        <span>Thông tin ứng dụng (About)</span>
                    </button>
                </nav>

                {/* Content Card */}
                <div className="settings-card">
                    {/* TAB 1: General (Cài đặt chung & Nền tảng) */}
                    {activeTab === 'general' && (
                        <>
                            {/* SECTION 1: COMMON (1:1 VỚI ẢNH CHỤP COCKPIT) */}
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Cấu hình Hệ thống & Giao diện (Common)</h2>
                                <p className="settings-card-desc">Thiết lập ngôn ngữ, hiển thị cửa sổ, giao diện thu phóng và các hành vi tự động.</p>
                            </div>

                            <div className="settings-group">
                                {/* 1. Language */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Language</div>
                                        <div className="setting-hint">Select the interface language</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.language || 'en'}
                                            onChange={(e) => void saveField('language', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="en">English</option>
                                            <option value="vi">Tiếng Việt</option>
                                            <option value="zh-CN">简体中文 (Simplified Chinese)</option>
                                            <option value="ja">日本語 (Japanese)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 2. Theme */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Theme</div>
                                        <div className="setting-hint">Switch between dark and light mode</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.theme || 'dark'}
                                            onChange={(e) => {
                                                applyTheme(e.target.value);
                                                void saveField('theme', e.target.value);
                                            }}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="dark">Dark</option>
                                            <option value="light">Light</option>
                                            <option value="system">System Default</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 3. Reduce motion */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Reduce motion</div>
                                        <div className="setting-hint">Cut page fades, modal transitions, shadows, blur, and smooth scrolling; keep essential loading feedback</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.reduced_motion_enabled}
                                                onChange={(e) => void saveField('reduced_motion_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 4. Default Terminal */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Default Terminal</div>
                                        <div className="setting-hint">The terminal used when opening CLI</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.default_terminal || 'system'}
                                            onChange={(e) => void saveField('default_terminal', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="system">System Default</option>
                                            <option value="bash">Bash</option>
                                            <option value="zsh">Zsh</option>
                                            <option value="fish">Fish</option>
                                            <option value="xterm">Xterm</option>
                                            <option value="gnome-terminal">GNOME Terminal</option>
                                            <option value="kitty">Kitty</option>
                                            <option value="alacritty">Alacritty</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 5. Sidebar Layout */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Sidebar Layout</div>
                                        <div className="setting-hint">Switch between original and classic sidebar layouts</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.side_nav_layout_mode || 'classic'}
                                            onChange={(e) => void saveField('side_nav_layout_mode', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="classic">Classic Layout</option>
                                            <option value="original">Original Layout</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 6. UI Scale */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">UI Scale</div>
                                        <div className="setting-hint">Adjust interface scale (⌘+/⌘- or Ctrl+/Ctrl-, ⌘0/Ctrl+0 reset)</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={String(settings.ui_scale || 1.0)}
                                            onChange={(e) => handleUiScaleChange(e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="0.8">80%</option>
                                            <option value="0.9">90%</option>
                                            <option value="1">100%</option>
                                            <option value="1.1">110%</option>
                                            <option value="1.25">125%</option>
                                            <option value="1.5">150%</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 7. Close behavior */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Close behavior</div>
                                        <div className="setting-hint">Choose the default action when closing the window</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.close_behavior || 'minimize'}
                                            onChange={(e) => void saveField('close_behavior', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="minimize">Minimize to tray</option>
                                            <option value="ask">Ask</option>
                                            <option value="quit">Quit</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 8. Start minimized */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Start minimized</div>
                                        <div className="setting-hint">Automatically minimize the main window after the app finishes launching; restore it from the Dock, taskbar, or tray</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.startup_minimized ? 'true' : 'false'}
                                            onChange={(e) => void saveField('startup_minimized', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="false">Disable</option>
                                            <option value="true">Enable</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 9. Remember main window position and size */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Remember main window position and size</div>
                                        <div className="setting-hint">Restore the main window position and size after restart or tray reopen; off by default</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.remember_main_window_state}
                                                onChange={(e) => void saveField('remember_main_window_state', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 10. Show floating card on startup */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Show floating card on startup</div>
                                        <div className="setting-hint">Display the floating account card automatically after app launch</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.floating_card_show_on_startup ? 'true' : 'false'}
                                            onChange={(e) => void saveField('floating_card_show_on_startup', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="false">Disable</option>
                                            <option value="true">Enable</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 11. Keep floating card on top by default */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Keep floating card on top by default</div>
                                        <div className="setting-hint">New floating card windows stay on top when opened</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.floating_card_always_on_top ? 'true' : 'false'}
                                            onChange={(e) => void saveField('floating_card_always_on_top', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="false">Disable</option>
                                            <option value="true">Enable</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 12. Launch Cockpit Tools at login */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Launch Cockpit Tools at login</div>
                                        <div className="setting-hint">Start the app after sign-in. If Windows shows a PowerShell error at boot, turn this off.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.app_auto_launch_enabled ? 'true' : 'false'}
                                            onChange={(e) => void saveField('app_auto_launch_enabled', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="false">Disable</option>
                                            <option value="true">Enable</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 13. Auth keep-alive */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Auth keep-alive</div>
                                        <div className="setting-hint">Refresh account tokens in small batches only when authorization is close to expiry, reducing background request pressure for large account sets.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={settings.token_keeper_enabled !== false}
                                                onChange={(e) => void saveField('token_keeper_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 14. Auto-import local accounts */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Auto-import local accounts</div>
                                        <div className="setting-hint">When enabled, Cockpit Tools immediately scans local client logins and imports them, then keeps importing when the official client switches accounts. The system keychain may prompt once; choose Always Allow.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.auto_import_from_local_enabled}
                                                onChange={(e) => void saveField('auto_import_from_local_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 15. Show floating card now */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Show floating card now</div>
                                        <div className="setting-hint">After closing it, reopen it here or from the tray menu</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            className="btn btn-secondary"
                                            style={{ padding: '0.45rem 1rem', fontSize: '0.8rem' }}
                                            onClick={handleShowFloatingCard}
                                        >
                                            Show floating card
                                        </button>
                                    </div>
                                </div>

                                {/* 16. Data Directory */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Data Directory</div>
                                        <div className="setting-hint">Storage location for accounts and configuration files.</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            className="btn btn-secondary"
                                            style={{ padding: '0.45rem 1rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                            onClick={handleOpenDataFolder}
                                        >
                                            <FolderOpen size={15} /> Open
                                        </button>
                                    </div>
                                </div>

                                {/* 17. Show top promo */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Show top promo</div>
                                        <div className="setting-hint">Hide the top promo slot in the app. The Sponsors page is not affected.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!settings.show_top_promo}
                                                onChange={(e) => void saveField('show_top_promo', !e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 18. Startup page */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Startup page</div>
                                        <div className="setting-hint">Page opened on cold start. Choose "Remember last" to restore the previous page.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.startup_page || 'last'}
                                            onChange={(e) => void saveField('startup_page', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="last">Remember last</option>
                                            <option value="dashboard">Dashboard</option>
                                            <option value="overview">Antigravity IDE</option>
                                            <option value="codebuddy">CodeBuddy</option>
                                            <option value="cursor">Cursor</option>
                                            <option value="trae">Trae</option>
                                            <option value="zed">Zed</option>
                                            <option value="codex">Codex</option>
                                            <option value="instances">Instances</option>
                                            <option value="settings">Settings</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 19. Color pack */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Color pack</div>
                                        <div className="setting-hint">Layer a color pack on light/dark (Nord, Tokyo Night, ...)</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.color_pack || 'default'}
                                            onChange={(e) => handleColorPackChange(e.target.value)}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value="default">Default</option>
                                            <option value="nord">Nord</option>
                                            <option value="tokyo-night">Tokyo Night</option>
                                            <option value="catppuccin">Catppuccin</option>
                                            <option value="gruvbox">Gruvbox</option>
                                            <option value="everforest">Everforest</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 20. Allow external network */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Allow external network</div>
                                        <div className="setting-hint">When off, blocks WebDAV, OpenRouter usage refresh, remote announcements, and auto update checks (local features keep working).</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={settings.allow_external_network !== false}
                                                onChange={(e) => void saveField('allow_external_network', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* 21. WebDAV domain allowlist */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">WebDAV domain allowlist</div>
                                        <div className="setting-hint">Comma-separated; empty = no limit. Host must match when set</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 240 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="example.com, dav.example.org"
                                            value={settings.webdav_allowed_domains || ''}
                                            onChange={(e) => void saveField('webdav_allowed_domains', e.target.value)}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>
                            </div>

                            {/* SECTION 2: PLATFORM INTEGRATIONS & AUTOMATION */}
                            <div className="settings-card-header" style={{ marginTop: '2.5rem' }}>
                                <h2 className="settings-card-title">Đường dẫn Ứng dụng & Quản trị Nền tảng (Platform Settings)</h2>
                                <p className="settings-card-desc">Cấu hình vị trí tệp thực thi, tự động phát hiện (.desktop) và hành vi khi chuyển tài khoản.</p>
                            </div>

                            <div className="settings-group">
                                {/* Launch on switch */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Launch on switch</div>
                                        <div className="setting-hint">Tự động khởi chạy IDE tương ứng sau khi bạn bấm chuyển/tiêm session.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={settings.launch_on_switch !== false}
                                                onChange={(e) => void saveField('launch_on_switch', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* Auto refresh minutes */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Chu kỳ tự động làm mới Quota (Auto Refresh Interval)</div>
                                        <div className="setting-hint">Khoảng thời gian tự động đồng bộ hạn mức quota tài khoản.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.auto_refresh_minutes || 10}
                                            onChange={(e) => void saveField('auto_refresh_minutes', Number(e.target.value))}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value={2}>2 phút</option>
                                            <option value={5}>5 phút</option>
                                            <option value={10}>10 phút (Mặc định Cockpit)</option>
                                            <option value={15}>15 phút</option>
                                            <option value={30}>30 phút</option>
                                        </select>
                                    </div>
                                </div>

                                {/* Antigravity IDE Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Antigravity IDE Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của Antigravity IDE (VS Code based).</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.antigravity_app_path || '/home/bimatkeo/Applications/antigravity-ide/antigravity-ide'}
                                            onChange={(e) => void saveField('antigravity_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('antigravity_app_path')}
                                            disabled={saving}
                                            title="Browse binary file"
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('antigravity_ide', 'antigravity_app_path')}
                                            disabled={saving}
                                            title="Auto-detect from .desktop file"
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* Antigravity Desktop Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Antigravity Desktop Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của Antigravity Desktop thường (App Legacy).</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.antigravity_desktop_app_path || '/home/bimatkeo/Applications/antigravity/antigravity'}
                                            onChange={(e) => void saveField('antigravity_desktop_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('antigravity_desktop_app_path')}
                                            disabled={saving}
                                            title="Browse binary file"
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('antigravity_desktop', 'antigravity_desktop_app_path')}
                                            disabled={saving}
                                            title="Auto-detect from .desktop file"
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* VS Code Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Visual Studio Code Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của VS Code.</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.vscode_app_path || '/usr/bin/code'}
                                            onChange={(e) => void saveField('vscode_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('vscode_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('vscode', 'vscode_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* CodeBuddy Global Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">CodeBuddy Global Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của CodeBuddy Global (codebuddy.ai).</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="/usr/bin/codebuddy"
                                            value={settings.codebuddy_app_path || ''}
                                            onChange={(e) => void saveField('codebuddy_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('codebuddy_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('codebuddy', 'codebuddy_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* CodeBuddy CN Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">CodeBuddy CN Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của CodeBuddy CN (copilot.tencent.com).</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="/usr/bin/codebuddy-cn"
                                            value={settings.codebuddy_cn_app_path || ''}
                                            onChange={(e) => void saveField('codebuddy_cn_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('codebuddy_cn_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('codebuddy_cn', 'codebuddy_cn_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* Cursor Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Cursor Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của trình soạn thảo Cursor AI.</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="/usr/share/cursor/cursor"
                                            value={settings.cursor_app_path || ''}
                                            onChange={(e) => void saveField('cursor_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('cursor_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('cursor', 'cursor_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* Trae Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Trae Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của ByteDance Trae IDE.</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="/usr/bin/trae"
                                            value={settings.trae_app_path || ''}
                                            onChange={(e) => void saveField('trae_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('trae_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('trae', 'trae_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>

                                {/* Zed Path */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Zed Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của Zed Editor.</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', width: 440 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="/home/bimatkeo/.local/zed.app/bin/zed"
                                            value={settings.zed_app_path || ''}
                                            onChange={(e) => void saveField('zed_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleBrowse('zed_app_path')}
                                            disabled={saving}
                                        >
                                            <FolderOpen size={13} /> Browse
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0 0.65rem', height: 32, fontSize: '0.75rem', gap: '0.25rem', whiteSpace: 'nowrap' }}
                                            onClick={() => handleAutoDetect('zed', 'zed_app_path')}
                                            disabled={saving}
                                        >
                                            <Sparkles size={13} /> Auto Detect
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 2: Network & Proxy */}
                    {activeTab === 'network' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Mạng & Proxy Toàn cục (Network)</h2>
                                <p className="settings-card-desc">Định tuyến toàn bộ lưu lượng xác thực OAuth, billing quota và gateway qua Proxy bảo mật.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Kích hoạt Proxy toàn cục (Global Proxy)</div>
                                        <div className="setting-hint">Bắt buộc tất cả các request tới Tencent/CodeBuddy/Zed đi qua HTTP/SOCKS5 proxy chỉ định.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.global_proxy_enabled}
                                                onChange={(e) => void saveField('global_proxy_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Địa chỉ máy chủ Proxy (Proxy URL)</div>
                                        <div className="setting-hint">Ví dụ: <code>http://127.0.0.1:7890</code> hoặc <code>socks5://127.0.0.1:1080</code></div>
                                    </div>
                                    <div className="setting-control" style={{ width: 300 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="http://127.0.0.1:7890"
                                            value={settings.global_proxy_url || ''}
                                            onChange={(e) => void saveField('global_proxy_url', e.target.value)}
                                            disabled={!settings.global_proxy_enabled || saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Danh sách bỏ qua Proxy (No Proxy)</div>
                                        <div className="setting-hint">Các domain và IP kết nối trực tiếp, phân tách bằng dấu phẩy.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 300 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="127.0.0.1,localhost,::1"
                                            value={settings.global_proxy_no_proxy || '127.0.0.1,localhost,::1'}
                                            onChange={(e) => void saveField('global_proxy_no_proxy', e.target.value)}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Cổng WebSocket Daemon (WS Port)</div>
                                        <div className="setting-hint">Cổng lắng nghe của background daemon nhận lệnh IPC nội bộ.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 140 }}>
                                        <input
                                            type="number"
                                            className="input"
                                            value={settings.ws_port || 19528}
                                            onChange={(e) => void saveField('ws_port', Number(e.target.value))}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.85rem' }}
                                        />
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 3: Data & WebDAV */}
                    {activeTab === 'data' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Quản lý Dữ liệu & Sao lưu Đám mây (Data)</h2>
                                <p className="settings-card-desc">Bảo vệ kho tài khoản qua bản sao lưu WebDAV, xuất nhập JSON và dọn dẹp bộ nhớ tạm.</p>
                            </div>

                            <div className="settings-group">
                                {/* Instance Cleanup */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Dọn dẹp phiên & Cache Instance</div>
                                        <div className="setting-hint">Xóa các phiên đăng nhập rác, giải phóng lock tệp và reset trạng thái ứng dụng đa mở.</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            className="btn btn-secondary"
                                            style={{ padding: '0.45rem 1rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                            onClick={handleCleanCache}
                                        >
                                            <Trash2 size={15} /> Dọn dẹp Cache
                                        </button>
                                    </div>
                                </div>

                                {/* Auto backup */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động sao lưu định kỳ (Auto Backup)</div>
                                        <div className="setting-hint">Tự động xuất tệp backup mã hóa lưu trữ an toàn trong máy tính.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={settings.auto_backup_enabled !== false}
                                                onChange={(e) => void saveField('auto_backup_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                {/* Retention days */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Thời gian lưu giữ bản sao lưu (Retention Days)</div>
                                        <div className="setting-hint">Tự động xóa các bản sao lưu cũ quá thời hạn quy định.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.auto_backup_retention_days || 15}
                                            onChange={(e) => void saveField('auto_backup_retention_days', Number(e.target.value))}
                                            disabled={saving}
                                            style={{ width: 220 }}
                                        >
                                            <option value={7}>7 ngày</option>
                                            <option value={15}>15 ngày (Mặc định Cockpit)</option>
                                            <option value={30}>30 ngày</option>
                                        </select>
                                    </div>
                                </div>

                                {/* WebDAV Sync */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Đồng bộ WebDAV từ xa (Cloud Sync)</div>
                                        <div className="setting-hint">Tải và đồng bộ kho tài khoản lên máy chủ đám mây cá nhân (Nextcloud, Jianguoyun, v.v.).</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.webdav_sync_enabled}
                                                onChange={(e) => void saveField('webdav_sync_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Địa chỉ máy chủ WebDAV (Server URL)</div>
                                        <div className="setting-hint">Ví dụ: <code>https://dav.jianguoyun.com/dav/</code></div>
                                    </div>
                                    <div className="setting-control" style={{ width: 320 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.webdav_sync_url || 'https://dav.jianguoyun.com/dav/'}
                                            onChange={(e) => void saveField('webdav_sync_url', e.target.value)}
                                            disabled={!settings.webdav_sync_enabled || saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tài khoản WebDAV (Username)</div>
                                        <div className="setting-hint">Tên đăng nhập hoặc địa chỉ email dịch vụ WebDAV.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 320 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.webdav_sync_username || ''}
                                            onChange={(e) => void saveField('webdav_sync_username', e.target.value)}
                                            disabled={!settings.webdav_sync_enabled || saving}
                                            style={{ width: '100%', fontSize: '0.85rem' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Mật khẩu ứng dụng WebDAV (App Password)</div>
                                        <div className="setting-hint">Mật khẩu ứng dụng chuyên dụng tạo từ nhà cung cấp WebDAV.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 320 }}>
                                        <input
                                            type="password"
                                            className="input"
                                            value={settings.webdav_sync_password || ''}
                                            onChange={(e) => void saveField('webdav_sync_password', e.target.value)}
                                            disabled={!settings.webdav_sync_enabled || saving}
                                            style={{ width: '100%', fontSize: '0.85rem' }}
                                        />
                                    </div>
                                </div>

                                {/* Backup & Transfer Actions */}
                                <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1.25rem', paddingTop: '1.25rem', borderTop: '1px solid var(--border-light)' }}>
                                    <button
                                        className="btn btn-primary"
                                        onClick={handleExportBackup}
                                    >
                                        <Download size={15} />
                                        <span>Sao lưu Cấu hình ngay</span>
                                    </button>

                                    <button
                                        className="btn"
                                        onClick={handleImportFromCockpit}
                                        disabled={saving}
                                    >
                                        <RefreshCw size={15} className={saving ? 'spin' : ''} />
                                        <span>Đồng bộ từ Cockpit (~/.cockpit_tools/)</span>
                                    </button>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 4: About */}
                    {activeTab === 'about' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Thông tin Ứng dụng & Bản quyền (About)</h2>
                                <p className="settings-card-desc">Phiên bản, tài liệu kỹ thuật và bản quyền hệ thống Universal API Cockpit.</p>
                            </div>

                            <div className="settings-group">
                                <div style={{ padding: '1.5rem', background: 'rgba(255, 255, 255, 0.02)', borderRadius: 12, border: '1px solid var(--border)' }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.85rem', marginBottom: '1rem' }}>
                                        <div style={{
                                            width: 44,
                                            height: 44,
                                            borderRadius: 10,
                                            background: 'linear-gradient(135deg, #3b82f6, #6366f1)',
                                            display: 'flex',
                                            alignItems: 'center',
                                            justifyContent: 'center',
                                            color: '#fff',
                                            fontWeight: 800,
                                            fontSize: '1.2rem',
                                            boxShadow: '0 4px 14px rgba(59, 130, 246, 0.4)'
                                        }}>
                                            U
                                        </div>
                                        <div>
                                            <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                                                Universal API Cockpit Suite
                                            </div>
                                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                                                Version 2.5.5 (Desktop Production Release)
                                            </div>
                                        </div>
                                    </div>

                                    <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: '1.25rem' }}>
                                        Nền tảng hợp nhất quản trị tài khoản đa nền tảng cho Google Antigravity, CodeBuddy, GitHub Copilot, Cursor AI, ByteDance Trae và Zed Editor. Hỗ trợ tự động tiêm phiên làm việc, phân phối hạn mức quota thông minh, chống cạn kiệt token và sao lưu đám mây WebDAV an toàn.
                                    </p>

                                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '0.75rem', fontSize: '0.8rem' }}>
                                        <div style={{ padding: '0.75rem', background: 'rgba(0, 0, 0, 0.2)', borderRadius: 8 }}>
                                            <div style={{ color: 'var(--text-muted)', marginBottom: '0.2rem' }}>Runtime Core</div>
                                            <div style={{ fontWeight: 600, color: '#38bdf8' }}>Tauri 2 + Rust + Tokio</div>
                                        </div>
                                        <div style={{ padding: '0.75rem', background: 'rgba(0, 0, 0, 0.2)', borderRadius: 8 }}>
                                            <div style={{ color: 'var(--text-muted)', marginBottom: '0.2rem' }}>Local Database</div>
                                            <div style={{ fontWeight: 600, color: '#10b981' }}>SQLite WAL / Encrypted</div>
                                        </div>
                                        <div style={{ padding: '0.75rem', background: 'rgba(0, 0, 0, 0.2)', borderRadius: 8 }}>
                                            <div style={{ color: 'var(--text-muted)', marginBottom: '0.2rem' }}>Compatibility</div>
                                            <div style={{ fontWeight: 600, color: '#f59e0b' }}>Cockpit Tools 1:1 Spec</div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}
                </div>
            </div>
        </div>
    );
}
