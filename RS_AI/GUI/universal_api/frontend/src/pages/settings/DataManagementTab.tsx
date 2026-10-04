import React from 'react';
import {
    Cloud,
    Calendar,
    HardDrive,
    Sparkles,
    Check,
    UploadCloud,
    FolderOpen,
    Play,
    CheckCircle2,
    XCircle,
    Search,
    Trash2,
    Download,
} from 'lucide-react';
import type {
    GuiSettings,
    WebdavSettings,
    WebdavRemoteFile,
    AutoCheckinConfig,
    AutoCheckinLogRecord,
    StorageScanReport,
    StorageCleanReport,
} from '../../../../bridge/types';
import { Toggle, formatBytes } from './SettingsShared';

interface DataManagementTabProps {
    settings: GuiSettings;
    saving: boolean;
    webdavConfig: WebdavSettings;
    setWebdavConfig: React.Dispatch<React.SetStateAction<WebdavSettings>>;
    webdavTesting: boolean;
    webdavBackingUp: boolean;
    webdavFiles: WebdavRemoteFile[];
    webdavLoadingFiles: boolean;
    webdavRestoring: string | null;
    handleSaveWebdav: () => Promise<void>;
    handleTestWebdav: () => Promise<void>;
    handleBackupWebdavNow: () => Promise<void>;
    handleListWebdavBackups: () => Promise<void>;
    handleRestoreWebdav: (fileName: string) => Promise<void>;
    checkinConfig: AutoCheckinConfig;
    checkinLogs: AutoCheckinLogRecord[];
    checkinRunning: boolean;
    handleToggleCheckin: (enabled: boolean) => Promise<void>;
    handleRunCheckinNow: () => Promise<void>;
    scanReport: StorageScanReport | null;
    cleanReport: StorageCleanReport | null;
    storageScanning: boolean;
    storageCleaning: boolean;
    cleanOrphans: boolean;
    setCleanOrphans: React.Dispatch<React.SetStateAction<boolean>>;
    cleanCaches: boolean;
    setCleanCaches: React.Dispatch<React.SetStateAction<boolean>>;
    handleScanStorage: () => Promise<void>;
    handleCleanStorage: () => Promise<void>;
    handleImportFromCockpit: () => Promise<void>;
    handleExportBackup: () => void;
}

export const DataManagementTab: React.FC<DataManagementTabProps> = ({
    saving,
    webdavConfig,
    setWebdavConfig,
    webdavTesting,
    webdavBackingUp,
    webdavFiles,
    webdavLoadingFiles,
    webdavRestoring,
    handleSaveWebdav,
    handleTestWebdav,
    handleBackupWebdavNow,
    handleListWebdavBackups,
    handleRestoreWebdav,
    checkinConfig,
    checkinLogs,
    checkinRunning,
    handleToggleCheckin,
    handleRunCheckinNow,
    scanReport,
    cleanReport,
    storageScanning,
    storageCleaning,
    cleanOrphans,
    setCleanOrphans,
    cleanCaches,
    setCleanCaches,
    handleScanStorage,
    handleCleanStorage,
    handleImportFromCockpit,
    handleExportBackup,
}) => {
    return (
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
    );
};
