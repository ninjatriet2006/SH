import { useState, useEffect } from 'react';
import {
    Sliders,
    Layers,
    Globe,
    Database,
    Info,
    Check,
    AlertCircle,
    RotateCcw,
    Search,
    FolderOpen,
    Download,
    Sparkles,
    Trash2,
    RefreshCw,
    Cloud,
    HardDrive,
    UploadCloud,
    CheckCircle2,
    XCircle,
    Calendar,
    Play,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useTranslation } from '../utils/i18n';
import type {
    GuiSettings,
    WebdavSettings,
    WebdavRemoteFile,
    StorageScanReport,
    StorageCleanReport,
    AutoCheckinStatusResponse,
    AutoCheckinConfig,
    AutoCheckinLogRecord,
    TokenKeeperReport,
} from '../../../bridge/types';
import { invokeIpc } from '../../../bridge/ipc';

function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

type SettingsTab = 'general' | 'platforms' | 'network' | 'data' | 'about';

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const { applyTheme } = useThemeStore();

    const [activeTab, setActiveTab] = useState<SettingsTab>('general');
    const [saving, setSaving] = useState(false);
    const [feedback, setFeedback] = useState<{ text: string; ok: boolean } | null>(null);
    const [platformSearch, setPlatformSearch] = useState('');

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

    const handleResetDefault = async (fieldKey: keyof GuiSettings, defaultVal: string = '') => {
        await saveField(fieldKey, defaultVal);
        showMsg('Đã đặt lại về đường dẫn mặc định', true);
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
                showMsg(res.message || 'Không tự động tìm thấy ứng dụng qua tệp .desktop. Vui lòng bấm Select để chọn thủ công.', false);
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

    // WebDAV State & Handlers
    const [webdavConfig, setWebdavConfig] = useState<WebdavSettings>({
        enabled: false,
        url: '',
        username: '',
        password: '',
        remoteDir: 'cockpit-backups',
    });
    const [webdavTesting, setWebdavTesting] = useState(false);
    const [webdavBackingUp, setWebdavBackingUp] = useState(false);
    const [webdavFiles, setWebdavFiles] = useState<WebdavRemoteFile[]>([]);
    const [webdavLoadingFiles, setWebdavLoadingFiles] = useState(false);
    const [webdavRestoring, setWebdavRestoring] = useState<string | null>(null);

    const loadWebdavConfig = async () => {
        try {
            const res = await invokeIpc<WebdavSettings>('get_webdav_config', {});
            setWebdavConfig(res);
        } catch {
            /* ignore */
        }
    };

    const handleSaveWebdav = async () => {
        try {
            setSaving(true);
            await invokeIpc<boolean, WebdavSettings>('save_webdav_config', webdavConfig);
            showMsg('Đã lưu cấu hình WebDAV thành công!', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi lưu cấu hình WebDAV', false);
        } finally {
            setSaving(false);
        }
    };

    const handleTestWebdav = async () => {
        try {
            setWebdavTesting(true);
            const msg = await invokeIpc<string, WebdavSettings>('test_webdav_connection_cmd', webdavConfig);
            showMsg(msg || 'Kết nối WebDAV thành công!', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi kiểm tra kết nối WebDAV', false);
        } finally {
            setWebdavTesting(false);
        }
    };

    const handleBackupWebdavNow = async () => {
        try {
            setWebdavBackingUp(true);
            const msg = await invokeIpc<string>('backup_to_webdav_now', {});
            showMsg(msg || 'Đã sao lưu lên máy chủ WebDAV!', true);
            void handleListWebdavBackups();
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi sao lưu lên WebDAV', false);
        } finally {
            setWebdavBackingUp(false);
        }
    };

    const handleListWebdavBackups = async () => {
        try {
            setWebdavLoadingFiles(true);
            const files = await invokeIpc<WebdavRemoteFile[]>('list_webdav_backups_cmd', {});
            setWebdavFiles(files);
            showMsg(`Tìm thấy ${files.length} bản sao lưu trên WebDAV!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi lấy danh sách bản sao lưu WebDAV', false);
        } finally {
            setWebdavLoadingFiles(false);
        }
    };

    const handleRestoreWebdav = async (fileName: string) => {
        if (!confirm(`Bạn có chắc chắn muốn phục hồi từ bản sao lưu "${fileName}"?\nDữ liệu tài khoản và cấu hình hiện tại sẽ được cập nhật an toàn.`)) {
            return;
        }
        try {
            setWebdavRestoring(fileName);
            const count = await invokeIpc<number, { file_name: string }>('restore_from_webdav_cmd', { file_name: fileName });
            showMsg(`Đã phục hồi thành công ${count} tệp từ WebDAV!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi phục hồi bản sao lưu WebDAV', false);
        } finally {
            setWebdavRestoring(null);
        }
    };

    // Auto Check-in State & Handlers
    const [checkinConfig, setCheckinConfig] = useState<AutoCheckinConfig>({ enabled: false });
    const [checkinLogs, setCheckinLogs] = useState<AutoCheckinLogRecord[]>([]);
    const [checkinRunning, setCheckinRunning] = useState(false);

    const loadCheckinStatus = async () => {
        try {
            const res = await invokeIpc<AutoCheckinStatusResponse>('get_auto_checkin_status', {});
            setCheckinConfig(res.config);
            setCheckinLogs(res.logs);
        } catch {
            /* ignore */
        }
    };

    const handleToggleCheckin = async (enabled: boolean) => {
        try {
            await invokeIpc<boolean, { enabled: boolean }>('toggle_auto_checkin', { enabled });
            setCheckinConfig((prev) => ({ ...prev, enabled }));
            showMsg(enabled ? 'Đã bật tự động điểm danh hàng ngày lúc 08:00 AM!' : 'Đã tắt tự động điểm danh hàng ngày.', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi đổi trạng thái tự động điểm danh', false);
        }
    };

    const handleRunCheckinNow = async () => {
        try {
            setCheckinRunning(true);
            const logs = await invokeIpc<AutoCheckinLogRecord[]>('run_auto_checkin_now', {});
            setCheckinLogs(logs);
            const successCount = logs.filter((l) => l.status === 'success' || l.status === 'already_checked').length;
            showMsg(`Điểm danh hoàn tất: ${successCount}/${logs.length} tài khoản thành công!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi thực hiện điểm danh ngay', false);
        } finally {
            setCheckinRunning(false);
        }
    };

    // Storage Cleanup State & Handlers
    const [storageScanning, setStorageScanning] = useState(false);
    const [storageCleaning, setStorageCleaning] = useState(false);
    const [scanReport, setScanReport] = useState<StorageScanReport | null>(null);
    const [cleanReport, setCleanReport] = useState<StorageCleanReport | null>(null);
    const [cleanOrphans, setCleanOrphans] = useState(true);
    const [cleanCaches, setCleanCaches] = useState(true);

    const handleScanStorage = async () => {
        try {
            setStorageScanning(true);
            const report = await invokeIpc<StorageScanReport>('scan_instance_storage', {});
            setScanReport(report);
            const orphanCount = report.orphanDirectories?.length || 0;
            const cacheCount = report.cacheFolders?.length || 0;
            showMsg(`Quét xong: ${orphanCount} orphan instances, ${cacheCount} thư mục cache!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi quét dung lượng rác', false);
        } finally {
            setStorageScanning(false);
        }
    };

    const handleCleanStorage = async () => {
        if (!scanReport) {
            await handleScanStorage();
            return;
        }
        try {
            setStorageCleaning(true);
            const report = await invokeIpc<StorageCleanReport, { delete_orphans: boolean; clean_caches: boolean }>(
                'execute_instance_storage_clean',
                { delete_orphans: cleanOrphans, clean_caches: cleanCaches }
            );
            setCleanReport(report);
            setScanReport(null);
            showMsg(`Đã dọn dẹp ${report.deletedPaths?.length || 0} thư mục, giải phóng ${formatBytes(report.freedBytes || 0)}!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi dọn dẹp dung lượng rác', false);
        } finally {
            setStorageCleaning(false);
        }
    };

    // Token Keeper State & Handlers
    const [tokenKeeperRunning, setTokenKeeperRunning] = useState(false);
    const [tokenKeeperReport, setTokenKeeperReport] = useState<TokenKeeperReport | null>(null);

    const handleTriggerTokenKeeper = async () => {
        try {
            setTokenKeeperRunning(true);
            const rep = await invokeIpc<TokenKeeperReport>('trigger_token_keeper', {});
            setTokenKeeperReport(rep);
            showMsg(`Token Keeper: Đã làm mới ${rep.refreshedCount}, đã kiểm tra ${rep.checkedCount}, lỗi ${rep.failedCount}`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi làm mới token keeper', false);
        } finally {
            setTokenKeeperRunning(false);
        }
    };

    useEffect(() => {
        if (activeTab === 'data') {
            void loadWebdavConfig();
            void loadCheckinStatus();
        }
    }, [activeTab]);

    // Reusable UI components styled strictly with index.css
    const Toggle = ({
        checked,
        onChange,
        disabled,
    }: {
        checked: boolean;
        onChange: (val: boolean) => void;
        disabled?: boolean;
    }) => (
        <label className="toggle-switch">
            <input
                type="checkbox"
                checked={checked}
                onChange={(e) => onChange(e.target.checked)}
                disabled={disabled}
            />
            <span className="toggle-slider" />
        </label>
    );

    const PathRow = ({
        label,
        desc,
        fieldKey,
        currentPath,
        placeholder = 'Default path',
        detectTarget,
        defaultPath = '',
    }: {
        label: string;
        desc?: string;
        fieldKey: keyof GuiSettings;
        currentPath?: string;
        placeholder?: string;
        detectTarget?: string;
        defaultPath?: string;
    }) => (
        <div className="setting-row">
            <div className="setting-info">
                <div className="setting-label">{label}</div>
                {desc && <div className="setting-hint">{desc}</div>}
            </div>
            <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', flexWrap: 'wrap', justifyContent: 'flex-end', minWidth: '320px', maxWidth: '65%' }}>
                <input
                    type="text"
                    readOnly
                    placeholder={placeholder}
                    value={currentPath || ''}
                    className="input"
                    style={{ flex: 1, minWidth: '180px', fontSize: '0.78rem', fontFamily: 'var(--font-mono)' }}
                />
                <button
                    type="button"
                    className="btn"
                    style={{ fontSize: '0.78rem', padding: '0.42rem 0.75rem', height: '32px' }}
                    onClick={() => handleBrowse(fieldKey)}
                    disabled={saving}
                >
                    Select
                </button>
                <button
                    type="button"
                    className="btn"
                    style={{ fontSize: '0.78rem', padding: '0.42rem 0.75rem', height: '32px', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                    title="Đặt lại về mặc định"
                    onClick={() => handleResetDefault(fieldKey, defaultPath)}
                    disabled={saving}
                >
                    <RotateCcw size={12} />
                    <span>Reset to default</span>
                </button>
                {detectTarget && (
                    <button
                        type="button"
                        className="btn btn-primary"
                        style={{ fontSize: '0.78rem', padding: '0.42rem 0.75rem', height: '32px', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                        title="Tự động quét tệp .desktop để tìm ứng dụng"
                        onClick={() => handleAutoDetect(detectTarget, fieldKey)}
                        disabled={saving}
                    >
                        <Search size={12} />
                        <span>Quét .desktop</span>
                    </button>
                )}
            </div>
        </div>
    );

    const quotaRefreshOptions = [
        { label: 'Disabled', value: 0 },
        { label: '1 min', value: 1 },
        { label: '2 min', value: 2 },
        { label: '5 min', value: 5 },
        { label: '10 min', value: 10 },
        { label: '15 min', value: 15 },
        { label: '30 min', value: 30 },
        { label: '1 hour', value: 60 },
    ];

    const currentAccountRefreshOptions = [
        { label: '1 min', value: 1 },
        { label: '2 min', value: 2 },
        { label: '5 min', value: 5 },
        { label: '10 min', value: 10 },
    ];

    const filterMatch = (text: string) => {
        if (!platformSearch.trim()) return true;
        return text.toLowerCase().includes(platformSearch.toLowerCase().trim());
    };

    return (
        <div className="settings-page">
            {/* Header */}
            <div className="settings-header">
                <div>
                    <h1 className="settings-header-title">{t('settings.title') || 'Cài đặt Hệ thống'}</h1>
                    <span style={{ fontSize: '0.82rem', color: 'var(--text-secondary)' }}>
                        Tùy chỉnh giao diện, mạng, đồng bộ dữ liệu và cấu hình chi tiết cho từng nền tảng AI
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

            {/* Layout: Left Sidebar + Right Card */}
            <div className="settings-layout">
                {/* Navigation Sidebar */}
                <nav className="settings-nav">
                    <button
                        className={`settings-nav-item ${activeTab === 'general' ? 'active' : ''}`}
                        onClick={() => setActiveTab('general')}
                    >
                        <Sliders size={16} />
                        <span>Chung (General)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'platforms' ? 'active' : ''}`}
                        onClick={() => setActiveTab('platforms')}
                    >
                        <Layers size={16} />
                        <span>Nền tảng (Platforms)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'network' ? 'active' : ''}`}
                        onClick={() => setActiveTab('network')}
                    >
                        <Globe size={16} />
                        <span>Mạng (Network)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'data' ? 'active' : ''}`}
                        onClick={() => setActiveTab('data')}
                    >
                        <Database size={16} />
                        <span>Dữ liệu & Sao lưu (Data)</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'about' ? 'active' : ''}`}
                        onClick={() => setActiveTab('about')}
                    >
                        <Info size={16} />
                        <span>Thông tin (About)</span>
                    </button>
                </nav>

                {/* Content Card */}
                <div className="settings-card">
                    {/* TAB 1: GENERAL */}
                    {activeTab === 'general' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Giao diện & Trải nghiệm (Common)</h2>
                                <p className="settings-card-desc">Thiết lập ngôn ngữ, chủ đề màu sắc, tỉ lệ thu phóng và hành vi cửa sổ ứng dụng.</p>
                            </div>

                            <div className="settings-group">
                                {/* 1. Language */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Ngôn ngữ (Language)</div>
                                        <div className="setting-hint">Chọn ngôn ngữ hiển thị giao diện</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.language || 'en'}
                                            onChange={(e) => void saveField('language', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
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
                                        <div className="setting-label">Chủ đề (Theme)</div>
                                        <div className="setting-hint">Chuyển đổi giữa giao diện Sáng, Tối hoặc Hệ thống</div>
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
                                            style={{ width: 200 }}
                                        >
                                            <option value="dark">Tối (Dark)</option>
                                            <option value="light">Sáng (Light)</option>
                                            <option value="system">Hệ thống (System)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 3. Color pack */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Gói màu sắc giao diện (Color Pack)</div>
                                        <div className="setting-hint">Tùy biến bảng màu sắc chủ đạo của ứng dụng (Nord, Tokyo Night, ...)</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.color_pack || 'default'}
                                            onChange={(e) => handleColorPackChange(e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="default">Mặc định (Default)</option>
                                            <option value="nord">Nord</option>
                                            <option value="tokyo-night">Tokyo Night</option>
                                            <option value="catppuccin">Catppuccin</option>
                                            <option value="gruvbox">Gruvbox</option>
                                            <option value="everforest">Everforest</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 4. UI Scale */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tỉ lệ giao diện (UI Scale)</div>
                                        <div className="setting-hint">Điều chỉnh kích thước tổng thể các thành phần giao diện (⌘+/⌘- hoặc Ctrl+/Ctrl-)</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={String(settings.ui_scale || 1.0)}
                                            onChange={(e) => handleUiScaleChange(e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="0.8">80%</option>
                                            <option value="0.9">90%</option>
                                            <option value="1">100% (Chuẩn)</option>
                                            <option value="1.1">110%</option>
                                            <option value="1.25">125%</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 5. Reduce motion */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Giảm hiệu ứng động (Reduce Motion)</div>
                                        <div className="setting-hint">Tắt hoạt ảnh chuyển trang và chuyển động để tối ưu hiệu năng</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!!settings.reduced_motion_enabled}
                                            onChange={(val) => void saveField('reduced_motion_enabled', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* 6. Sidebar Layout */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Bố cục thanh bên (Sidebar Layout)</div>
                                        <div className="setting-hint">Lựa chọn hiển thị thanh điều hướng bên trái</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.side_nav_layout_mode || 'classic'}
                                            onChange={(e) => void saveField('side_nav_layout_mode', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="classic">Cổ điển (Classic)</option>
                                            <option value="original">Nguyên bản (Original)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 7. Startup page */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Trang mở đầu (Startup Page)</div>
                                        <div className="setting-hint">Trang hiển thị mặc định khi ứng dụng khởi chạy</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.startup_page || 'last'}
                                            onChange={(e) => void saveField('startup_page', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="last">Ghi nhớ trang trước (Remember last)</option>
                                            <option value="dashboard">Bảng điều khiển (Dashboard)</option>
                                            <option value="overview">Antigravity IDE</option>
                                            <option value="codebuddy">CodeBuddy</option>
                                            <option value="cursor">Cursor</option>
                                            <option value="trae">Trae</option>
                                            <option value="zed">Zed</option>
                                            <option value="codex">Codex</option>
                                            <option value="instances">Quản lý phiên (Instances)</option>
                                            <option value="settings">Cài đặt (Settings)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 8. Show top promo */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Hiển thị thông báo đỉnh trang (Show Top Promo)</div>
                                        <div className="setting-hint">Bật/tắt thanh banner quảng bá tính năng mới ở trên cùng</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!settings.show_top_promo}
                                            onChange={(val) => void saveField('show_top_promo', !val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* SECTION: CỬA SỔ & HỆ THỐNG */}
                                <div className="sub-section-header">
                                    <span>Cửa sổ & Hệ thống</span>
                                </div>

                                {/* 9. Close behavior */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Hành vi khi đóng cửa sổ (Close Behavior)</div>
                                        <div className="setting-hint">Lựa chọn đóng cửa sổ vào khay hệ thống hay thoát hoàn toàn</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.close_behavior || 'tray'}
                                            onChange={(e) => void saveField('close_behavior', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="tray">Thu nhỏ vào khay (Minimize to tray)</option>
                                            <option value="exit">Thoát ứng dụng (Exit)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 10. Launch Cockpit at login */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Khởi động cùng máy tính (Launch at Login)</div>
                                        <div className="setting-hint">Tự động khởi động ứng dụng ngầm sau khi đăng nhập hệ thống</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.app_auto_launch_enabled ? 'true' : 'false'}
                                            onChange={(e) => void saveField('app_auto_launch_enabled', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="false">Tắt (Disable)</option>
                                            <option value="true">Bật (Enable)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 11. Start minimized */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Khởi động ở chế độ thu nhỏ (Start Minimized)</div>
                                        <div className="setting-hint">Ẩn cửa sổ chính vào khay hệ thống khi ứng dụng bắt đầu</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.startup_minimized ? 'true' : 'false'}
                                            onChange={(e) => void saveField('startup_minimized', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="false">Tắt (Disable)</option>
                                            <option value="true">Bật (Enable)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 12. Remember main window position and size */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Ghi nhớ kích thước & vị trí cửa sổ (Remember Window State)</div>
                                        <div className="setting-hint">Khôi phục đúng kích thước và tọa độ cửa sổ từ phiên trước</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!!settings.remember_main_window_state}
                                            onChange={(val) => void saveField('remember_main_window_state', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* 13. Default Terminal */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Terminal mặc định (Default Terminal)</div>
                                        <div className="setting-hint">Trình giả lập dòng lệnh dùng khi mở terminal từ ứng dụng</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.default_terminal || 'system'}
                                            onChange={(e) => void saveField('default_terminal', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="system">Mặc định hệ thống (System Default)</option>
                                            <option value="bash">Bash</option>
                                            <option value="zsh">Zsh</option>
                                            <option value="fish">Fish</option>
                                            <option value="gnome-terminal">GNOME Terminal</option>
                                            <option value="kitty">Kitty</option>
                                            <option value="alacritty">Alacritty</option>
                                        </select>
                                    </div>
                                </div>

                                {/* SECTION: TÀI KHOẢN & DỮ LIỆU NỀN */}
                                <div className="sub-section-header">
                                    <span>Tài khoản & Dữ liệu nền</span>
                                </div>

                                {/* 14. Auth keep-alive */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Duy trì ủy quyền token ngầm (Token Keeper Daemon)</div>
                                        <div className="setting-hint">Tự động làm mới phiên đăng nhập các tài khoản (Google, Copilot, Cursor, Codebuddy) trước khi hết hạn</div>
                                        {tokenKeeperReport && (
                                            <div style={{ marginTop: '0.4rem', fontSize: '0.78rem', color: tokenKeeperReport.failedCount > 0 ? 'var(--warning)' : 'var(--success)' }}>
                                                Lần chạy gần nhất: Đã làm mới {tokenKeeperReport.refreshedCount} token, đã kiểm tra {tokenKeeperReport.checkedCount}, lỗi {tokenKeeperReport.failedCount}
                                            </div>
                                        )}
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.78rem', padding: '0.35rem 0.65rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                                            onClick={handleTriggerTokenKeeper}
                                            disabled={tokenKeeperRunning}
                                            title="Kiểm tra hạn dùng và làm mới token các tài khoản ngay bây giờ"
                                        >
                                            <RefreshCw size={12} className={tokenKeeperRunning ? 'spin' : ''} />
                                            <span>{tokenKeeperRunning ? 'Đang chạy...' : 'Làm mới ngay'}</span>
                                        </button>
                                        <Toggle
                                            checked={settings.token_keeper_enabled !== false}
                                            onChange={(val) => void saveField('token_keeper_enabled', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* 15. Auto-import local accounts */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động phát hiện tài khoản cục bộ (Auto-import local accounts)</div>
                                        <div className="setting-hint">Tự quét và nhận diện tài khoản khi IDE chính thức đăng nhập</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!!settings.auto_import_from_local_enabled}
                                            onChange={(val) => void saveField('auto_import_from_local_enabled', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* 16. Show floating card on startup */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Hiển thị thẻ nổi khi khởi động (Show floating card on startup)</div>
                                        <div className="setting-hint">Tự động mở cửa sổ mini thẻ tài khoản nổi trên màn hình</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.floating_card_show_on_startup ? 'true' : 'false'}
                                            onChange={(e) => void saveField('floating_card_show_on_startup', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="false">Tắt (Disable)</option>
                                            <option value="true">Bật (Enable)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 17. Keep floating card on top */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Luôn ghim thẻ nổi lên trên (Keep floating card on top)</div>
                                        <div className="setting-hint">Cửa sổ thẻ tài khoản nổi luôn hiển thị phía trên các ứng dụng khác</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.floating_card_always_on_top ? 'true' : 'false'}
                                            onChange={(e) => void saveField('floating_card_always_on_top', e.target.value === 'true')}
                                            disabled={saving}
                                            style={{ width: 200 }}
                                        >
                                            <option value="false">Tắt (Disable)</option>
                                            <option value="true">Bật (Enable)</option>
                                        </select>
                                    </div>
                                </div>

                                {/* 18. Action buttons for Floating Card & Data directory */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Thẻ tài khoản nổi ngay bây giờ (Floating Card)</div>
                                        <div className="setting-hint">Kích hoạt cửa sổ mini xem nhanh quota và chuyển đổi tài khoản</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            type="button"
                                            className="btn"
                                            onClick={handleShowFloatingCard}
                                        >
                                            Mở thẻ nổi ngay (Show floating card)
                                        </button>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Thư mục dữ liệu ứng dụng (Data Directory)</div>
                                        <div className="setting-hint">Nơi lưu trữ cơ sở dữ liệu tài khoản, cấu hình và instances</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            type="button"
                                            className="btn"
                                            onClick={handleOpenDataFolder}
                                            style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                        >
                                            <FolderOpen size={14} />
                                            <span>Mở thư mục (Open)</span>
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 2: PLATFORMS */}
                    {activeTab === 'platforms' && (
                        <>
                            <div className="settings-card-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
                                <div>
                                    <h2 className="settings-card-title">Cấu hình Nền tảng AI & IDE (Platforms)</h2>
                                    <p className="settings-card-desc">Tần suất làm mới quota, cảnh báo giới hạn, và đường dẫn tệp thực thi theo đúng chuẩn Cockpit.</p>
                                </div>

                                <div style={{ position: 'relative', width: '220px' }}>
                                    <Search size={14} style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-muted)' }} />
                                    <input
                                        type="text"
                                        placeholder="Tìm nền tảng..."
                                        value={platformSearch}
                                        onChange={(e) => setPlatformSearch(e.target.value)}
                                        className="input"
                                        style={{ paddingLeft: '2rem', width: '100%', fontSize: '0.8rem' }}
                                    />
                                </div>
                            </div>

                            <div className="settings-group">
                                {/* PLATFORM 1: Antigravity IDE */}
                                {filterMatch('Antigravity') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Antigravity Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Antigravity Auto Refresh Quota</div>
                                                    <div className="setting-hint">Tần suất làm mới dữ liệu hạn mức tự động</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.antigravity_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('antigravity_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Current Account Refresh</div>
                                                    <div className="setting-hint">Tần suất làm mới riêng tài khoản đang chọn</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.antigravity_current_account_refresh_minutes ?? 1}
                                                        onChange={(e) => void saveField('antigravity_current_account_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {currentAccountRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Antigravity IDE Launch Target"
                                                desc="Đường dẫn tệp thực thi Antigravity IDE (VS Code based)"
                                                fieldKey="antigravity_app_path"
                                                currentPath={settings.antigravity_app_path}
                                                detectTarget="antigravity_ide"
                                                defaultPath="/home/bimatkeo/Applications/antigravity-ide/antigravity-ide"
                                            />

                                            <PathRow
                                                label="Antigravity Desktop Launch Target"
                                                desc="Đường dẫn tệp thực thi Antigravity Desktop (App Legacy)"
                                                fieldKey="antigravity_desktop_app_path"
                                                currentPath={settings.antigravity_desktop_app_path}
                                                detectTarget="antigravity_desktop"
                                                defaultPath="/home/bimatkeo/Applications/antigravity/antigravity"
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">Gửi thông báo khi hạn mức mô hình tụt dưới ngưỡng cho phép</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.antigravity_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('antigravity_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Chuyển đổi tài khoản không cần khởi động lại (Dual-Switch No-Restart)</div>
                                                    <div className="setting-hint">Đồng bộ phiên đăng nhập trực tiếp qua SQLite/State mà không làm gián đoạn IDE</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={!!settings.antigravity_dual_switch_no_restart_enabled}
                                                        onChange={(val) => void saveField('antigravity_dual_switch_no_restart_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 2: Claude */}
                                {filterMatch('Claude') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Claude Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Claude quota auto refresh</div>
                                                    <div className="setting-hint">Automatically refresh cached quota for Claude accounts in the background.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.claude_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('claude_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Current Account Refresh</div>
                                                    <div className="setting-hint">Refresh current account only. Default is 1 minute.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.claude_current_account_refresh_minutes ?? 1}
                                                        onChange={(e) => void saveField('claude_current_account_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {currentAccountRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Claude quota as remaining %</div>
                                                    <div className="setting-hint">Default shows used percentage; enable to show remaining. Auto-switch and alerts still use used ratio.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={!!settings.claude_quota_display_remaining}
                                                        onChange={(val) => void saveField('claude_quota_display_remaining', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Claude Desktop launch target"
                                                desc="The default profile can use a Microsoft Store target; multi-instance profiles need the real Claude executable."
                                                fieldKey="claude_app_path"
                                                currentPath={settings.claude_app_path}
                                                detectTarget="claude"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">When any current-account model quota drops below the threshold, send a native notification.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.claude_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('claude_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 3: Zed */}
                                {filterMatch('Zed') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Zed Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Zed Auto Refresh Quota</div>
                                                    <div className="setting-hint">Background update frequency</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.zed_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('zed_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Current Account Refresh</div>
                                                    <div className="setting-hint">Refresh current account only. Default is 1 minute.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.zed_current_account_refresh_minutes ?? 1}
                                                        onChange={(e) => void saveField('zed_current_account_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {currentAccountRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Zed Launch Path"
                                                desc="Leave empty to use default path"
                                                fieldKey="zed_app_path"
                                                currentPath={settings.zed_app_path}
                                                detectTarget="zed"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">When any current-account model quota drops below the threshold, send a native notification.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.zed_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('zed_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 4: GitHub Copilot & VS Code */}
                                {(filterMatch('Copilot') || filterMatch('VS Code') || filterMatch('GitHub')) && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">GitHub Copilot Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">GitHub Copilot Auto Refresh Quota</div>
                                                    <div className="setting-hint">Background update frequency</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.ghcp_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('ghcp_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Current Account Refresh</div>
                                                    <div className="setting-hint">Refresh current account only. Default is 1 minute.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.ghcp_current_account_refresh_minutes ?? 1}
                                                        onChange={(e) => void saveField('ghcp_current_account_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {currentAccountRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="VS Code App Path"
                                                desc="Leave empty to use default path (/usr/bin/code)"
                                                fieldKey="vscode_app_path"
                                                currentPath={settings.vscode_app_path}
                                                detectTarget="code"
                                                defaultPath="/usr/bin/code"
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">When any current-account model quota drops below the threshold, send a native notification.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.ghcp_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('ghcp_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Đồng bộ cấu hình OpenCode khi chuyển tài khoản</div>
                                                    <div className="setting-hint">Tự động cấu hình token cho plugin OpenCode / CLI khi chuyển đổi tài khoản</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={!!settings.ghcp_opencode_sync_on_switch}
                                                        onChange={(val) => void saveField('ghcp_opencode_sync_on_switch', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 5: CodeBuddy & CodeBuddy CN */}
                                {filterMatch('CodeBuddy') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">CodeBuddy Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">CodeBuddy Auto Refresh Quota</div>
                                                    <div className="setting-hint">Tần suất tự động làm mới quota</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.codebuddy_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('codebuddy_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="CodeBuddy App Path (Global)"
                                                desc="Đường dẫn tệp thực thi của CodeBuddy Quốc Tế"
                                                fieldKey="codebuddy_app_path"
                                                currentPath={settings.codebuddy_app_path}
                                                detectTarget="codebuddy"
                                                defaultPath=""
                                            />

                                            <PathRow
                                                label="CodeBuddy CN App Path"
                                                desc="Đường dẫn tệp thực thi của CodeBuddy Nội Địa (Tencent Cloud)"
                                                fieldKey="codebuddy_cn_app_path"
                                                currentPath={settings.codebuddy_cn_app_path}
                                                detectTarget="codebuddy_cn"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Chia sẻ phiên đăng nhập khi đổi tài khoản (Share sessions on switch)</div>
                                                    <div className="setting-hint">Giữ lại các context lịch sử chat hiện tại giữa các tài khoản</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={!!settings.codebuddy_share_sessions_on_switch}
                                                        onChange={(val) => void saveField('codebuddy_share_sessions_on_switch', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 6: Cursor AI */}
                                {filterMatch('Cursor') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Cursor Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Cursor Auto Refresh Quota</div>
                                                    <div className="setting-hint">Tần suất làm mới dữ liệu hạn mức tự động</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.cursor_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('cursor_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Cursor Launch Path"
                                                desc="Đường dẫn tệp thực thi của trình soạn thảo Cursor"
                                                fieldKey="cursor_app_path"
                                                currentPath={settings.cursor_app_path}
                                                detectTarget="cursor"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">Cảnh báo khi số lượt Fast Requests sắp hết</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.cursor_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('cursor_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 7: Trae */}
                                {filterMatch('Trae') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Trae Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Trae Auto Refresh Quota</div>
                                                    <div className="setting-hint">Tần suất làm mới hạn mức Claude/GPT trên Trae</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.trae_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('trae_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Trae IDE Launch Path"
                                                desc="Đường dẫn tệp thực thi Trae (ByteDance)"
                                                fieldKey="trae_app_path"
                                                currentPath={settings.trae_app_path}
                                                detectTarget="trae"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">Thông báo khi quota mô hình cao cấp của Trae cạn kiệt</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.trae_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('trae_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 8: Windsurf / Devin */}
                                {(filterMatch('Devin') || filterMatch('Windsurf')) && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Devin / Windsurf Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Devin / Windsurf Auto Refresh Quota</div>
                                                    <div className="setting-hint">Background update frequency</div>
                                                </div>
                                                <div className="setting-control">
                                                    <select
                                                        className="select"
                                                        value={settings.windsurf_auto_refresh_minutes ?? 10}
                                                        onChange={(e) => void saveField('windsurf_auto_refresh_minutes', Number(e.target.value))}
                                                        disabled={saving}
                                                        style={{ width: 140 }}
                                                    >
                                                        {quotaRefreshOptions.map((opt) => (
                                                            <option key={opt.value} value={opt.value}>{opt.label}</option>
                                                        ))}
                                                    </select>
                                                </div>
                                            </div>

                                            <PathRow
                                                label="Devin / Windsurf App Path"
                                                desc="Leave empty to use default path"
                                                fieldKey="windsurf_app_path"
                                                currentPath={settings.windsurf_app_path}
                                                detectTarget="windsurf"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Enable Quota Alert</div>
                                                    <div className="setting-hint">When any current-account model quota drops below the threshold, send a native notification.</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.windsurf_quota_alert_enabled !== false}
                                                        onChange={(val) => void saveField('windsurf_quota_alert_enabled', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 9: Codex */}
                                {filterMatch('Codex') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Codex Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <PathRow
                                                label="Codex CLI / App Path"
                                                desc="Đường dẫn tệp thực thi của OpenAI Codex CLI"
                                                fieldKey="codex_app_path"
                                                currentPath={settings.codex_app_path}
                                                detectTarget="codex"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Đồng bộ môi trường WSL (Codex Sync WSL)</div>
                                                    <div className="setting-hint">Đồng bộ auth tokens vào môi trường Windows Subsystem for Linux</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={!!settings.codex_sync_wsl}
                                                        onChange={(val) => void saveField('codex_sync_wsl', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* PLATFORM 10: Grok CLI */}
                                {filterMatch('Grok') && (
                                    <>
                                        <div className="platform-title-bar">
                                            <span className="platform-title-bar-bar" />
                                            <span className="platform-title-bar-text">Grok CLI Settings</span>
                                        </div>
                                        <div className="platform-settings-card">
                                            <PathRow
                                                label="Grok CLI Executable Path"
                                                desc="Đường dẫn nhị phân xAI Grok command line tool"
                                                fieldKey="grok_cli_path"
                                                currentPath={settings.grok_cli_path}
                                                detectTarget="grok"
                                                defaultPath=""
                                            />

                                            <div className="setting-row">
                                                <div className="setting-info">
                                                    <div className="setting-label">Sync Official Auth on Switch</div>
                                                    <div className="setting-hint">Tự động ghi đè token vào ~/.xai/config khi đổi tài khoản</div>
                                                </div>
                                                <div className="setting-control">
                                                    <Toggle
                                                        checked={settings.grok_sync_official_auth_on_switch !== false}
                                                        onChange={(val) => void saveField('grok_sync_official_auth_on_switch', val)}
                                                        disabled={saving}
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    </>
                                )}
                            </div>
                        </>
                    )}

                    {/* TAB 3: NETWORK */}
                    {activeTab === 'network' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Mạng & Proxy Toàn cục (Network)</h2>
                                <p className="settings-card-desc">Cấu hình kết nối Internet, proxy chuyển tiếp và danh sách cho phép kết nối ngoại vi.</p>
                            </div>

                            <div className="settings-group">
                                {/* Allow external network */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Cho phép kết nối mạng ngoại vi (Allow external network)</div>
                                        <div className="setting-hint">Khi tắt, ứng dụng sẽ chặn mọi kết nối WebDAV, OpenRouter, cập nhật và đồng bộ đám mây (các tính năng cục bộ vẫn hoạt động).</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={settings.allow_external_network !== false}
                                            onChange={(val) => void saveField('allow_external_network', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* Global Proxy Enabled */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Bật Proxy toàn cục (Global Proxy)</div>
                                        <div className="setting-hint">Chuyển hướng mọi yêu cầu gọi API và kiểm tra quota qua máy chủ proxy trung gian.</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!!settings.global_proxy_enabled}
                                            onChange={(val) => void saveField('global_proxy_enabled', val)}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                {/* Global Proxy URL */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Địa chỉ máy chủ Proxy (Proxy URL)</div>
                                        <div className="setting-hint">Hỗ trợ giao thức HTTP, HTTPS hoặc SOCKS5 (VD: http://127.0.0.1:7890 hoặc socks5://127.0.0.1:1080)</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 280 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="http://127.0.0.1:7890"
                                            value={settings.global_proxy_url || ''}
                                            onChange={(e) => void saveField('global_proxy_url', e.target.value)}
                                            disabled={saving || !settings.global_proxy_enabled}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>

                                {/* Global Proxy No-Proxy */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Bỏ qua Proxy (No-Proxy Allowlist)</div>
                                        <div className="setting-hint">Danh sách tên miền hoặc dải IP không đi qua proxy, ngăn cách bằng dấu phẩy.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 280 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="localhost, 127.0.0.1, ::1"
                                            value={settings.global_proxy_no_proxy || ''}
                                            onChange={(e) => void saveField('global_proxy_no_proxy', e.target.value)}
                                            disabled={saving || !settings.global_proxy_enabled}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>

                                {/* WebDAV Domain Allowlist */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Danh sách máy chủ WebDAV cho phép (WebDAV Domain Allowlist)</div>
                                        <div className="setting-hint">Phân tách bằng dấu phẩy; để trống = không giới hạn. Host phải khớp khi kích hoạt.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 280 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="example.com, dav.example.org"
                                            value={settings.webdav_allowed_domains || ''}
                                            onChange={(e) => void saveField('webdav_allowed_domains', e.target.value)}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>

                                {/* WebSocket Port */}
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Cổng WebSocket Daemon (WS Port)</div>
                                        <div className="setting-hint">Cổng nội bộ để kết nối giao tiếp giữa các tiến trình phụ trợ và giao diện UI.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 200 }}>
                                        <input
                                            type="number"
                                            className="input"
                                            value={settings.ws_port || 50005}
                                            onChange={(e) => void saveField('ws_port', Number(e.target.value))}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 4: DATA, SYNC & CLEANUP */}
                    {activeTab === 'data' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Dữ liệu, Đồng bộ & Dọn dẹp Hệ thống</h2>
                                <p className="settings-card-desc">Sao lưu đám mây WebDAV, tự động điểm danh tài nguyên hàng ngày, và dọn dẹp bộ nhớ đệm cache instances an toàn.</p>
                            </div>

                            <div className="settings-group">
                                {/* SECTION 1: WEBDAV CLOUD SYNC */}
                                <div className="sub-section-header">
                                    <Cloud size={16} />
                                    <span>Đồng bộ & Sao lưu Đám mây (WebDAV Cloud Sync)</span>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Kích hoạt Đồng bộ đám mây WebDAV (WebDAV Cloud Sync)</div>
                                        <div className="setting-hint">Tự động sao lưu và đồng bộ tài khoản mã hóa với Nextcloud, Jianguoyun, OwnCloud hoặc máy chủ WebDAV riêng.</div>
                                    </div>
                                    <div className="setting-control">
                                        <Toggle
                                            checked={!!webdavConfig.enabled}
                                            onChange={(val) => setWebdavConfig({ ...webdavConfig, enabled: val })}
                                            disabled={saving}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Địa chỉ máy chủ WebDAV (Server URL)</div>
                                        <div className="setting-hint">Đường dẫn đầy đủ tới máy chủ WebDAV (VD: https://dav.jianguoyun.com/dav/)</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 340 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="https://dav.jianguoyun.com/dav/"
                                            value={webdavConfig.url || ''}
                                            onChange={(e) => setWebdavConfig({ ...webdavConfig, url: e.target.value })}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tài khoản WebDAV (Username)</div>
                                        <div className="setting-hint">Tên đăng nhập hoặc địa chỉ email xác thực WebDAV</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 340 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="user@example.com"
                                            value={webdavConfig.username || ''}
                                            onChange={(e) => setWebdavConfig({ ...webdavConfig, username: e.target.value })}
                                            style={{ width: '100%' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Mật khẩu ứng dụng WebDAV (App Password)</div>
                                        <div className="setting-hint">Khuyến nghị tạo App Password/Token chuyên dụng thay cho mật khẩu chính</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 340 }}>
                                        <input
                                            type="password"
                                            className="input"
                                            placeholder="••••••••••••"
                                            value={webdavConfig.password || ''}
                                            onChange={(e) => setWebdavConfig({ ...webdavConfig, password: e.target.value })}
                                            style={{ width: '100%' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Thư mục sao lưu từ xa (Remote Backup Directory)</div>
                                        <div className="setting-hint">Thư mục trên máy chủ WebDAV lưu các bản sao lưu (mặc định: cockpit-backup)</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 340 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            placeholder="cockpit-backups"
                                            value={webdavConfig.remoteDir || 'cockpit-backups'}
                                            onChange={(e) => setWebdavConfig({ ...webdavConfig, remoteDir: e.target.value })}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Thao tác WebDAV</div>
                                        <div className="setting-hint">Kiểm tra kết nối, lưu cấu hình, sao lưu ngay hoặc xem các bản sao lưu trên đám mây</div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                            onClick={handleTestWebdav}
                                            disabled={webdavTesting}
                                        >
                                            <Cloud size={14} className={webdavTesting ? 'spin' : ''} />
                                            <span>{webdavTesting ? 'Đang kiểm tra...' : 'Kiểm tra kết nối'}</span>
                                        </button>

                                        <button
                                            type="button"
                                            className="btn btn-primary"
                                            style={{ fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                            onClick={handleSaveWebdav}
                                            disabled={saving}
                                        >
                                            <Check size={14} />
                                            <span>Lưu cấu hình</span>
                                        </button>

                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                            onClick={handleBackupWebdavNow}
                                            disabled={webdavBackingUp}
                                        >
                                            <UploadCloud size={14} className={webdavBackingUp ? 'spin' : ''} />
                                            <span>{webdavBackingUp ? 'Đang sao lưu...' : 'Sao lưu lên đám mây ngay'}</span>
                                        </button>

                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                            onClick={handleListWebdavBackups}
                                            disabled={webdavLoadingFiles}
                                        >
                                            <FolderOpen size={14} className={webdavLoadingFiles ? 'spin' : ''} />
                                            <span>{webdavLoadingFiles ? 'Đang tải...' : 'Xem danh sách bản sao lưu'}</span>
                                        </button>
                                    </div>
                                </div>

                                {/* WebDAV Backup Files List */}
                                {webdavFiles.length > 0 && (
                                    <div style={{ background: 'var(--bg-secondary)', borderRadius: '8px', padding: '0.85rem', marginTop: '0.5rem', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
                                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                                            <span style={{ fontSize: '0.85rem', fontWeight: 600 }}>Các bản sao lưu trên WebDAV ({webdavFiles.length})</span>
                                        </div>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', maxHeight: '220px', overflowY: 'auto' }}>
                                            {webdavFiles.map((f) => (
                                                <div
                                                    key={f.fileName}
                                                    style={{
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                        justifyContent: 'space-between',
                                                        padding: '0.45rem 0.65rem',
                                                        borderRadius: '6px',
                                                        background: 'rgba(255, 255, 255, 0.03)',
                                                        fontSize: '0.8rem',
                                                    }}
                                                >
                                                    <div style={{ display: 'flex', flexDirection: 'column' }}>
                                                        <span style={{ fontFamily: 'var(--font-mono)', fontWeight: 500 }}>{f.fileName}</span>
                                                        <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>
                                                            Kích thước: {formatBytes(f.sizeBytes)} {f.modifiedAt ? `• Ngày: ${f.modifiedAt}` : ''}
                                                        </span>
                                                    </div>
                                                    <button
                                                        type="button"
                                                        className="btn"
                                                        style={{ fontSize: '0.75rem', padding: '0.25rem 0.55rem' }}
                                                        onClick={() => void handleRestoreWebdav(f.fileName)}
                                                        disabled={webdavRestoring === f.fileName}
                                                    >
                                                        {webdavRestoring === f.fileName ? 'Đang phục hồi...' : 'Phục hồi (Restore)'}
                                                    </button>
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}

                                {/* SECTION 2: WORKBUDDY / CODEBUDDY AUTO CHECK-IN */}
                                <div className="sub-section-header">
                                    <Calendar size={16} />
                                    <span>Tự động điểm danh Tencent Workbuddy / Codebuddy (Auto Check-in)</span>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động điểm danh nhận lượt miễn phí hàng ngày</div>
                                        <div className="setting-hint">
                                            Tương thích hoàn toàn với tính năng Auto Check-in của Cockpit Tools. Daemon sẽ tự động gọi API Tencent vào mỗi sáng.
                                            {checkinConfig.lastCheckedDate && (
                                                <div style={{ marginTop: '0.35rem', color: 'var(--text-secondary)' }}>
                                                    Lần điểm danh gần nhất: <span style={{ fontFamily: 'var(--font-mono)' }}>{checkinConfig.lastCheckedDate}</span>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.78rem', padding: '0.35rem 0.65rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                                            onClick={handleRunCheckinNow}
                                            disabled={checkinRunning}
                                        >
                                            <Play size={12} className={checkinRunning ? 'spin' : ''} />
                                            <span>{checkinRunning ? 'Đang điểm danh...' : 'Điểm danh ngay'}</span>
                                        </button>
                                        <Toggle
                                            checked={!!checkinConfig.enabled}
                                            onChange={(val) => void handleToggleCheckin(val)}
                                        />
                                    </div>
                                </div>

                                {/* Check-in Recent Logs */}
                                {checkinLogs.length > 0 && (
                                    <div style={{ background: 'var(--bg-secondary)', borderRadius: '8px', padding: '0.85rem', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
                                        <span style={{ fontSize: '0.85rem', fontWeight: 600, display: 'block', marginBottom: '0.5rem' }}>
                                            Lịch sử điểm danh gần đây ({checkinLogs.length})
                                        </span>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.35rem', maxHeight: '160px', overflowY: 'auto' }}>
                                            {checkinLogs.slice(0, 10).map((log, idx) => {
                                                const isSuccess = log.status === 'success' || log.status === 'already_checked';
                                                return (
                                                    <div
                                                        key={idx}
                                                        style={{
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            justifyContent: 'space-between',
                                                            padding: '0.4rem 0.6rem',
                                                            borderRadius: '6px',
                                                            background: 'rgba(255, 255, 255, 0.03)',
                                                            fontSize: '0.78rem',
                                                        }}
                                                    >
                                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                                            {isSuccess ? (
                                                                <CheckCircle2 size={14} style={{ color: 'var(--success)' }} />
                                                            ) : (
                                                                <XCircle size={14} style={{ color: 'var(--danger)' }} />
                                                            )}
                                                            <span style={{ fontWeight: 600 }}>{log.email || log.accountId}</span>
                                                            <span style={{ color: 'var(--text-muted)' }}>{log.message}</span>
                                                        </div>
                                                        <span style={{ fontSize: '0.72rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                                                            {log.timestamp}
                                                        </span>
                                                    </div>
                                                );
                                            })}
                                        </div>
                                    </div>
                                )}

                                {/* SECTION 3: STORAGE & CACHE CLEANUP */}
                                <div className="sub-section-header">
                                    <HardDrive size={16} />
                                    <span>Dọn dẹp Lưu trữ Instances & Bộ nhớ đệm (Storage Cleanup)</span>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Quét dung lượng rác và instances mồ côi</div>
                                        <div className="setting-hint">
                                            Phát hiện các thư mục instance không còn sử dụng trong ~/.cockpit_tools/instances và các thư mục bộ nhớ đệm nặng (GPUCache, Code Cache, Service Worker) để dọn dẹp giải phóng dung lượng đĩa.
                                        </div>
                                    </div>
                                    <div className="setting-control" style={{ display: 'flex', gap: '0.5rem' }}>
                                        <button
                                            type="button"
                                            className="btn"
                                            style={{ fontSize: '0.78rem', padding: '0.42rem 0.75rem', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                            onClick={handleScanStorage}
                                            disabled={storageScanning}
                                        >
                                            <Search size={14} className={storageScanning ? 'spin' : ''} />
                                            <span>{storageScanning ? 'Đang quét...' : 'Quét dung lượng'}</span>
                                        </button>
                                    </div>
                                </div>

                                {/* Storage Scan Result Card */}
                                {scanReport && (
                                    <div style={{ background: 'var(--bg-secondary)', borderRadius: '8px', padding: '0.85rem', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
                                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                                            <span style={{ fontSize: '0.85rem', fontWeight: 600 }}>Kết quả quét dung lượng có thể thu hồi</span>
                                        </div>

                                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem', marginBottom: '0.75rem' }}>
                                            <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', cursor: 'pointer', background: 'rgba(255, 255, 255, 0.03)', padding: '0.6rem', borderRadius: '6px' }}>
                                                <input
                                                    type="checkbox"
                                                    checked={cleanOrphans}
                                                    onChange={(e) => setCleanOrphans(e.target.checked)}
                                                />
                                                <div>
                                                    <div style={{ fontWeight: 600 }}>Orphan Instances ({scanReport.orphanDirectories?.length || 0} mục)</div>
                                                    <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>Chiếm: {formatBytes(scanReport.totalOrphanBytes || 0)}</div>
                                                </div>
                                            </label>

                                            <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', cursor: 'pointer', background: 'rgba(255, 255, 255, 0.03)', padding: '0.6rem', borderRadius: '6px' }}>
                                                <input
                                                    type="checkbox"
                                                    checked={cleanCaches}
                                                    onChange={(e) => setCleanCaches(e.target.checked)}
                                                />
                                                <div>
                                                    <div style={{ fontWeight: 600 }}>Bộ nhớ đệm rác ({scanReport.cacheFolders?.length || 0} mục)</div>
                                                    <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>Chiếm: {formatBytes(scanReport.totalCacheBytes || 0)}</div>
                                                </div>
                                            </label>
                                        </div>

                                        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                                            <button
                                                type="button"
                                                className="btn"
                                                style={{ fontSize: '0.8rem', color: 'var(--danger)', borderColor: 'rgba(239, 68, 68, 0.3)', display: 'flex', alignItems: 'center', gap: '0.35rem' }}
                                                onClick={handleCleanStorage}
                                                disabled={storageCleaning || (!cleanOrphans && !cleanCaches)}
                                            >
                                                <Trash2 size={14} className={storageCleaning ? 'spin' : ''} />
                                                <span>{storageCleaning ? 'Đang dọn dẹp...' : 'Dọn dẹp các mục đã chọn ngay'}</span>
                                            </button>
                                        </div>
                                    </div>
                                )}

                                {/* Storage Clean Report Card */}
                                {cleanReport && (
                                    <div style={{ background: 'rgba(34, 197, 94, 0.1)', border: '1px solid var(--success)', borderRadius: '8px', padding: '0.85rem' }}>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--success)', fontWeight: 600, fontSize: '0.85rem' }}>
                                            <CheckCircle2 size={16} />
                                            <span>Dọn dẹp thành công! Đã xóa {cleanReport.deletedPaths?.length || 0} thư mục rác, giải phóng {formatBytes(cleanReport.freedBytes || 0)}.</span>
                                        </div>
                                    </div>
                                )}

                                {/* SECTION 4: LOCAL IMPORT & EXPORT */}
                                <div className="sub-section-header">
                                    <Sparkles size={16} />
                                    <span>Nhập & Xuất Cục bộ (Local Import & Export)</span>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Nhập dữ liệu từ Cockpit Tools (Import from Cockpit)</div>
                                        <div className="setting-hint">Tự động phát hiện và chuyển đổi tất cả cấu hình tài khoản từ bản cài đặt Cockpit gốc sang hệ thống mới.</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            type="button"
                                            className="btn btn-primary"
                                            onClick={handleImportFromCockpit}
                                            disabled={saving}
                                            style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                        >
                                            <Sparkles size={14} />
                                            <span>Nhập tài khoản từ Cockpit</span>
                                        </button>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Xuất tệp sao lưu cấu hình (Export Settings Backup)</div>
                                        <div className="setting-hint">Tải về tệp cấu hình JSON đầy đủ để phục hồi hoặc di chuyển sang thiết bị khác.</div>
                                    </div>
                                    <div className="setting-control">
                                        <button
                                            type="button"
                                            className="btn"
                                            onClick={handleExportBackup}
                                            style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                        >
                                            <Download size={14} />
                                            <span>Xuất bản sao lưu (.json)</span>
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 5: ABOUT */}
                    {activeTab === 'about' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Thông tin Ứng dụng & Giấy phép</h2>
                                <p className="settings-card-desc">Phiên bản nền tảng, công nghệ điều phối và tình trạng hoạt động của Universal Cockpit.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Phiên bản Universal Engine (Core Version)</div>
                                        <div className="setting-hint">Kiến trúc đa luồng Rust Actix Web kết hợp giao diện Tauri v2</div>
                                    </div>
                                    <div className="setting-control">
                                        <span className="badge" style={{ background: 'rgba(59, 130, 246, 0.15)', color: 'var(--primary)', border: '1px solid rgba(59, 130, 246, 0.3)', padding: '0.3rem 0.75rem', borderRadius: '6px', fontWeight: 600 }}>
                                            v2.5.5-unified
                                        </span>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Môi trường thực thi (Environment)</div>
                                        <div className="setting-hint">Hệ thống tệp cục bộ và phiên giao diện</div>
                                    </div>
                                    <div className="setting-control">
                                        <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>
                                            Linux x86_64 (GTK3 / WebKitGTK)
                                        </span>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Dung lượng bộ nhớ & Kết nối Daemon</div>
                                        <div className="setting-hint">Trạng thái phản hồi của IPC Gateway nội bộ</div>
                                    </div>
                                    <div className="setting-control">
                                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.82rem', color: 'var(--success)' }}>
                                            <span style={{ width: 8, height: 8, borderRadius: '50%', backgroundColor: 'var(--success)' }} />
                                            Đang hoạt động ổn định
                                        </span>
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
