import { useState, useEffect } from 'react';
import {
    Sliders,
    Layers,
    Globe,
    Database,
    Info,
    Check,
    AlertCircle,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { ipcErrorMessage, invokeIpc } from '../../../bridge/ipc';
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

import { type SettingsTab } from './settings/SettingsShared';
import { GeneralSettingsTab } from './settings/GeneralSettingsTab';
import { PlatformsSettingsTab } from './settings/PlatformsSettingsTab';
import { NetworkSettingsTab } from './settings/NetworkSettingsTab';
import { DataManagementTab } from './settings/DataManagementTab';
import { AboutSettingsTab } from './settings/AboutSettingsTab';

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
    const [checkinConfig, setCheckinConfig] = useState<AutoCheckinConfig>({
        enabled: false,
        lastCheckedDate: null,
    });
    const [checkinLogs, setCheckinLogs] = useState<AutoCheckinLogRecord[]>([]);
    const [checkinRunning, setCheckinRunning] = useState(false);

    const loadCheckinStatus = async () => {
        try {
            const res = await invokeIpc<AutoCheckinStatusResponse>('get_auto_checkin_status', {});
            setCheckinConfig(res.config);
            setCheckinLogs(res.logs || []);
        } catch {
            /* ignore */
        }
    };

    const handleToggleCheckin = async (enabled: boolean) => {
        try {
            await invokeIpc<boolean, { enabled: boolean }>('toggle_auto_checkin', { enabled });
            setCheckinConfig((prev) => ({ ...prev, enabled }));
            showMsg(enabled ? 'Đã kích hoạt tự động điểm danh' : 'Đã tắt tự động điểm danh', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi bật/tắt tự động điểm danh', false);
        }
    };

    const handleRunCheckinNow = async () => {
        try {
            setCheckinRunning(true);
            const logs = await invokeIpc<AutoCheckinLogRecord[]>('run_auto_checkin_now', {});
            setCheckinLogs(logs);
            showMsg(`Điểm danh hoàn tất cho ${logs.length} tài khoản!`, true);
            void loadCheckinStatus();
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi chạy điểm danh', false);
        } finally {
            setCheckinRunning(false);
        }
    };

    // Storage Cleaner State & Handlers
    const [scanReport, setScanReport] = useState<StorageScanReport | null>(null);
    const [cleanReport, setCleanReport] = useState<StorageCleanReport | null>(null);
    const [storageScanning, setStorageScanning] = useState(false);
    const [storageCleaning, setStorageCleaning] = useState(false);
    const [cleanOrphans, setCleanOrphans] = useState(true);
    const [cleanCaches, setCleanCaches] = useState(true);

    const handleScanStorage = async () => {
        try {
            setStorageScanning(true);
            const res = await invokeIpc<StorageScanReport>('scan_instance_storage', {});
            setScanReport(res);
            setCleanReport(null);
            showMsg('Đã hoàn tất quét lưu trữ instances!', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi quét lưu trữ', false);
        } finally {
            setStorageScanning(false);
        }
    };

    const handleCleanStorage = async () => {
        if (!scanReport) return;
        if (!confirm('Bạn có chắc chắn muốn dọn dẹp các thư mục rác và bộ nhớ đệm đã chọn?')) {
            return;
        }
        try {
            setStorageCleaning(true);
            const res = await invokeIpc<StorageCleanReport, { delete_orphans: boolean; clean_caches: boolean }>(
                'execute_instance_storage_clean',
                { delete_orphans: cleanOrphans, clean_caches: cleanCaches }
            );
            setCleanReport(res);
            setScanReport(null);
            showMsg('Đã hoàn tất dọn dẹp bộ nhớ đệm!', true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi dọn dẹp lưu trữ', false);
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
            const res = await invokeIpc<TokenKeeperReport>('trigger_token_keeper', {});
            setTokenKeeperReport(res);
            showMsg(`Token Keeper hoàn tất: Đã kiểm tra ${res.checkedCount} tài khoản, làm mới ${res.refreshedCount} token!`, true);
        } catch (e) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi kích hoạt Token Keeper', false);
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

    return (
        <div className="page-container">
            {/* Page Header */}
            <div className="page-header">
                <div>
                    <h1 className="page-title">{t('settings.title')}</h1>
                    <p className="page-desc">Tùy biến môi trường, tài khoản và hành vi của Universal Engine</p>
                </div>
            </div>

            {/* Notification Banner */}
            {feedback && (
                <div
                    style={{
                        padding: '0.65rem 1rem',
                        borderRadius: '8px',
                        marginBottom: '1rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.5rem',
                        fontSize: '0.85rem',
                        background: feedback.ok ? 'rgba(34, 197, 94, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                        color: feedback.ok ? 'var(--success)' : 'var(--danger)',
                        border: `1px solid ${feedback.ok ? 'var(--success)' : 'var(--danger)'}`,
                        animation: 'fadeIn 0.2s ease',
                    }}
                >
                    {feedback.ok ? <Check size={16} /> : <AlertCircle size={16} />}
                    <span>{feedback.text}</span>
                </div>
            )}

            {/* Settings Layout with Vertical Tabs */}
            <div className="settings-layout">
                {/* Side Navigation Bar */}
                <div className="settings-nav">
                    <button
                        type="button"
                        className={`settings-nav-item ${activeTab === 'general' ? 'active' : ''}`}
                        onClick={() => setActiveTab('general')}
                    >
                        <Sliders size={16} />
                        <span>Chung & Giao diện</span>
                    </button>
                    <button
                        type="button"
                        className={`settings-nav-item ${activeTab === 'platforms' ? 'active' : ''}`}
                        onClick={() => setActiveTab('platforms')}
                    >
                        <Layers size={16} />
                        <span>Nền tảng AI & IDE</span>
                    </button>
                    <button
                        type="button"
                        className={`settings-nav-item ${activeTab === 'network' ? 'active' : ''}`}
                        onClick={() => setActiveTab('network')}
                    >
                        <Globe size={16} />
                        <span>Mạng & Proxy</span>
                    </button>
                    <button
                        type="button"
                        className={`settings-nav-item ${activeTab === 'data' ? 'active' : ''}`}
                        onClick={() => setActiveTab('data')}
                    >
                        <Database size={16} />
                        <span>Dữ liệu & Dọn dẹp</span>
                    </button>
                    <button
                        type="button"
                        className={`settings-nav-item ${activeTab === 'about' ? 'active' : ''}`}
                        onClick={() => setActiveTab('about')}
                    >
                        <Info size={16} />
                        <span>Thông tin ứng dụng</span>
                    </button>
                </div>

                {/* Content Card */}
                <div className="settings-card">
                    {activeTab === 'general' && (
                        <GeneralSettingsTab
                            settings={settings}
                            saving={saving}
                            saveField={saveField}
                            applyTheme={applyTheme}
                            handleColorPackChange={handleColorPackChange}
                            handleUiScaleChange={handleUiScaleChange}
                            tokenKeeperReport={tokenKeeperReport}
                            tokenKeeperRunning={tokenKeeperRunning}
                            handleTriggerTokenKeeper={handleTriggerTokenKeeper}
                            handleShowFloatingCard={handleShowFloatingCard}
                            handleOpenDataFolder={handleOpenDataFolder}
                        />
                    )}

                    {activeTab === 'platforms' && (
                        <PlatformsSettingsTab
                            settings={settings}
                            saving={saving}
                            saveField={saveField}
                            handleBrowse={handleBrowse}
                            handleResetDefault={handleResetDefault}
                            handleAutoDetect={handleAutoDetect}
                        />
                    )}

                    {activeTab === 'network' && (
                        <NetworkSettingsTab
                            settings={settings}
                            saving={saving}
                            saveField={saveField}
                        />
                    )}

                    {activeTab === 'data' && (
                        <DataManagementTab
                            settings={settings}
                            saving={saving}
                            webdavConfig={webdavConfig}
                            setWebdavConfig={setWebdavConfig}
                            webdavTesting={webdavTesting}
                            webdavBackingUp={webdavBackingUp}
                            webdavFiles={webdavFiles}
                            webdavLoadingFiles={webdavLoadingFiles}
                            webdavRestoring={webdavRestoring}
                            handleSaveWebdav={handleSaveWebdav}
                            handleTestWebdav={handleTestWebdav}
                            handleBackupWebdavNow={handleBackupWebdavNow}
                            handleListWebdavBackups={handleListWebdavBackups}
                            handleRestoreWebdav={handleRestoreWebdav}
                            checkinConfig={checkinConfig}
                            checkinLogs={checkinLogs}
                            checkinRunning={checkinRunning}
                            handleToggleCheckin={handleToggleCheckin}
                            handleRunCheckinNow={handleRunCheckinNow}
                            scanReport={scanReport}
                            cleanReport={cleanReport}
                            storageScanning={storageScanning}
                            storageCleaning={storageCleaning}
                            cleanOrphans={cleanOrphans}
                            setCleanOrphans={setCleanOrphans}
                            cleanCaches={cleanCaches}
                            setCleanCaches={setCleanCaches}
                            handleScanStorage={handleScanStorage}
                            handleCleanStorage={handleCleanStorage}
                            handleImportFromCockpit={handleImportFromCockpit}
                            handleExportBackup={handleExportBackup}
                        />
                    )}

                    {activeTab === 'about' && <AboutSettingsTab />}
                </div>
            </div>
        </div>
    );
}
