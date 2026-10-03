import { useState } from 'react';
import {
    Palette,
    Shield,
    Globe,
    Terminal,
    Zap,
    Cloud,
    Check,
    AlertCircle,
    Download,
    RefreshCw,
} from 'lucide-react';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useFontStore } from '../store/useFontStore';
import { useTranslation } from '../utils/i18n';
import type { GuiSettings } from '../../../bridge/types';
import { invokeIpc } from '../../../bridge/ipc';

type SettingsTab = 'appearance' | 'daemon' | 'network' | 'integrations' | 'autoswitch' | 'backup';

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const { themes, applyTheme } = useThemeStore();
    const { fonts } = useFontStore();

    const [activeTab, setActiveTab] = useState<SettingsTab>('appearance');
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

    return (
        <div className="settings-page">
            <div className="settings-header">
                <div>
                    <h1 className="settings-header-title">{t('settings.title') || 'Cài đặt Hệ thống'}</h1>
                    <span style={{ fontSize: '0.82rem', color: 'var(--text-secondary)' }}>
                        Quản lý giao diện, token daemon, mạng proxy, IDE integrations và bản sao lưu Cockpit
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
                {/* Navigation Sidebar */}
                <nav className="settings-nav">
                    <button
                        className={`settings-nav-item ${activeTab === 'appearance' ? 'active' : ''}`}
                        onClick={() => setActiveTab('appearance')}
                    >
                        <Palette size={16} />
                        <span>Giao diện & Trải nghiệm</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'daemon' ? 'active' : ''}`}
                        onClick={() => setActiveTab('daemon')}
                    >
                        <Shield size={16} />
                        <span>Token Keeper Daemon</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'network' ? 'active' : ''}`}
                        onClick={() => setActiveTab('network')}
                    >
                        <Globe size={16} />
                        <span>Mạng & Proxy Toàn cục</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'integrations' ? 'active' : ''}`}
                        onClick={() => setActiveTab('integrations')}
                    >
                        <Terminal size={16} />
                        <span>Đường dẫn IDE & Automation</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'autoswitch' ? 'active' : ''}`}
                        onClick={() => setActiveTab('autoswitch')}
                    >
                        <Zap size={16} />
                        <span>Cảnh báo & Đảo tài khoản</span>
                    </button>
                    <button
                        className={`settings-nav-item ${activeTab === 'backup' ? 'active' : ''}`}
                        onClick={() => setActiveTab('backup')}
                    >
                        <Cloud size={16} />
                        <span>Sao lưu & Đồng bộ WebDAV</span>
                    </button>
                </nav>

                {/* Content Card */}
                <div className="settings-card">
                    {/* TAB 1: Appearance & UI */}
                    {activeTab === 'appearance' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Giao diện & Trải nghiệm người dùng</h2>
                                <p className="settings-card-desc">Tùy biến ngôn ngữ, màu sắc chủ đạo, font chữ và hành vi cửa sổ ứng dụng.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Ngôn ngữ hiển thị (Language)</div>
                                        <div className="setting-hint">Lựa chọn ngôn ngữ cho toàn bộ giao diện quản trị.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.language || 'en'}
                                            onChange={(e) => void saveField('language', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 180 }}
                                        >
                                            <option value="en">English (US)</option>
                                            <option value="vi">Tiếng Việt</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Chủ đề giao diện (Theme)</div>
                                        <div className="setting-hint">Chế độ hiển thị sáng, tối hoặc theo hệ điều hành.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.theme || 'default'}
                                            onChange={(e) => {
                                                applyTheme(e.target.value);
                                                void saveField('theme', e.target.value);
                                            }}
                                            disabled={saving}
                                            style={{ width: 180 }}
                                        >
                                            {themes.map((t) => (
                                                <option key={t.id} value={t.id}>{t.name}</option>
                                            ))}
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Font chữ hệ thống (Font Family)</div>
                                        <div className="setting-hint">Chọn kiểu chữ hiển thị cho toàn bộ văn bản và bảng mã.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.font || 'default'}
                                            onChange={(e) => void saveField('font', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 180 }}
                                        >
                                            {fonts.map((f) => (
                                                <option key={f.id} value={f.id}>{f.name}</option>
                                            ))}
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Hành vi khi đóng cửa sổ (Close Behavior)</div>
                                        <div className="setting-hint">Thu nhỏ xuống khay hệ thống (System Tray) để giữ token sống hoặc thoát hẳn.</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.close_behavior || 'minimize'}
                                            onChange={(e) => void saveField('close_behavior', e.target.value)}
                                            disabled={saving}
                                            style={{ width: 180 }}
                                        >
                                            <option value="minimize">Thu xuống khay hệ thống</option>
                                            <option value="quit">Thoát ứng dụng hoàn toàn</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Giảm hiệu ứng động (Reduced Motion)</div>
                                        <div className="setting-hint">Tắt các chuyển động lướt và hiệu ứng xoay để tối ưu hóa hiệu năng máy yếu.</div>
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
                            </div>
                        </>
                    )}

                    {/* TAB 2: Token Keeper Daemon */}
                    {activeTab === 'daemon' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Token Keeper & Auto-Refresh Daemon</h2>
                                <p className="settings-card-desc">Cơ chế giữ phiên đăng nhập độc quyền từ Cockpit, tự động xoay vòng refresh token định kỳ.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Token Keeper Background Daemon</div>
                                        <div className="setting-hint">Chạy tiến trình ngầm định kỳ gọi API refresh để ngăn chặn Tencent thu hồi token (TTL 30 ngày).</div>
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

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Chu kỳ tự động làm mới (Auto Refresh Interval)</div>
                                        <div className="setting-hint">Khoảng thời gian giữa mỗi lần quét và xoay vòng refresh token của các tài khoản (mặc định 10 phút).</div>
                                    </div>
                                    <div className="setting-control">
                                        <select
                                            className="select"
                                            value={settings.auto_refresh_minutes || 10}
                                            onChange={(e) => void saveField('auto_refresh_minutes', Number(e.target.value))}
                                            disabled={saving}
                                            style={{ width: 180 }}
                                        >
                                            <option value={5}>5 phút (Thường xuyên)</option>
                                            <option value={10}>10 phút (Khuyên dùng - Chuẩn Cockpit)</option>
                                            <option value={15}>15 phút</option>
                                            <option value={30}>30 phút</option>
                                            <option value={60}>60 phút (Tiết kiệm mạng)</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động nạp tài khoản từ IDE khi khởi động</div>
                                        <div className="setting-hint">Quét tệp <code>state.vscdb</code> trong VS Code / Antigravity IDE khi mở ứng dụng để đồng bộ phiên mới nhất.</div>
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

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Chia sẻ phiên làm việc giữa các IDE (Share Sessions)</div>
                                        <div className="setting-hint">Đồng bộ trạng thái đăng nhập đồng thời sang cả VS Code, Trae, Antigravity IDE khi chuyển tài khoản.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.share_sessions_on_switch}
                                                onChange={(e) => void saveField('share_sessions_on_switch', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 3: Network & Proxy */}
                    {activeTab === 'network' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Mạng & Proxy Toàn cục</h2>
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
                                    <div className="setting-control" style={{ width: 280 }}>
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
                                    <div className="setting-control" style={{ width: 280 }}>
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
                            </div>
                        </>
                    )}

                    {/* TAB 4: IDE Paths & Automation */}
                    {activeTab === 'integrations' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Đường dẫn Công cụ IDE & Tự động hóa</h2>
                                <p className="settings-card-desc">Cấu hình vị trí tệp thực thi để Cockpit tự động khởi chạy và tiêm tài khoản khi bấm nút Play.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động khởi chạy IDE khi chuyển tài khoản (Launch on Switch)</div>
                                        <div className="setting-hint">Mở ngay ứng dụng IDE tương ứng sau khi bạn bấm nút Play tiêm session.</div>
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

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">VS Code Executable Path</div>
                                        <div className="setting-hint">Đường dẫn tệp thực thi của Visual Studio Code.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 280 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.vscode_app_path || '/usr/bin/code'}
                                            onChange={(e) => void saveField('vscode_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Antigravity IDE Executable Path</div>
                                        <div className="setting-hint">Đường dẫn nhị phân của Antigravity IDE desktop.</div>
                                    </div>
                                    <div className="setting-control" style={{ width: 280 }}>
                                        <input
                                            type="text"
                                            className="input"
                                            value={settings.antigravity_app_path || '/home/bimatkeo/Applications/antigravity-ide/antigravity-ide'}
                                            onChange={(e) => void saveField('antigravity_app_path', e.target.value)}
                                            disabled={saving}
                                            style={{ width: '100%', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}
                                        />
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 5: Auto Switch & Quota Alerts */}
                    {activeTab === 'autoswitch' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Cảnh báo Hạn mức & Tự động Đảo tài khoản</h2>
                                <p className="settings-card-desc">Thiết lập ngưỡng cảnh báo khi sắp cạn quota và tự động chuyển sang tài khoản dự phòng.</p>
                            </div>

                            <div className="settings-group">
                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Cảnh báo hạn mức cạn kiệt (Quota Alert)</div>
                                        <div className="setting-hint">Hiển thị thông báo khi số lượng request còn lại xuống thấp.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.quota_alert_enabled}
                                                onChange={(e) => void saveField('quota_alert_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Ngưỡng cảnh báo hạn mức (Alert Threshold)</div>
                                        <div className="setting-hint">Cảnh báo khi tài nguyên còn lại ít hơn hoặc bằng mức này: {settings.quota_alert_threshold || 20}%</div>
                                    </div>
                                    <div className="setting-control">
                                        <input
                                            type="range"
                                            min={5}
                                            max={50}
                                            step={5}
                                            value={settings.quota_alert_threshold || 20}
                                            onChange={(e) => void saveField('quota_alert_threshold', Number(e.target.value))}
                                            disabled={!settings.quota_alert_enabled || saving}
                                            style={{ width: 160 }}
                                        />
                                    </div>
                                </div>

                                <div className="setting-row">
                                    <div className="setting-info">
                                        <div className="setting-label">Tự động đảo tài khoản (Auto Switch on Depletion)</div>
                                        <div className="setting-hint">Khi tài khoản hiện tại hết hạn mức, tự động kích hoạt tài khoản tiếp theo trong kho.</div>
                                    </div>
                                    <div className="setting-control">
                                        <label className="toggle-switch">
                                            <input
                                                type="checkbox"
                                                checked={!!settings.auto_switch_enabled}
                                                onChange={(e) => void saveField('auto_switch_enabled', e.target.checked)}
                                                disabled={saving}
                                            />
                                            <span className="toggle-slider" />
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </>
                    )}

                    {/* TAB 6: Cloud Sync & WebDAV */}
                    {activeTab === 'backup' && (
                        <>
                            <div className="settings-card-header">
                                <h2 className="settings-card-title">Sao lưu & Đồng bộ Đám mây (WebDAV)</h2>
                                <p className="settings-card-desc">Bảo vệ kho tài khoản của bạn thông qua sao lưu mã hóa cục bộ và đồng bộ máy chủ WebDAV.</p>
                            </div>

                            <div className="settings-group">
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
                                            style={{ width: 180 }}
                                        >
                                            <option value={7}>7 ngày</option>
                                            <option value={15}>15 ngày (Mặc định Cockpit)</option>
                                            <option value={30}>30 ngày</option>
                                        </select>
                                    </div>
                                </div>

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

                                {/* Quick Actions Row */}
                                <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1rem', paddingTop: '1rem', borderTop: '1px solid var(--border-light)' }}>
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
                </div>
            </div>
        </div>
    );
}
