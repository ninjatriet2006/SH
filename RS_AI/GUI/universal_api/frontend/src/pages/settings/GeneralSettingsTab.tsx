import React from 'react';
import { RefreshCw, FolderOpen } from 'lucide-react';
import type { GuiSettings, TokenKeeperReport } from '../../../../bridge/types';
import { Toggle } from './SettingsShared';

interface GeneralSettingsTabProps {
    settings: GuiSettings;
    saving: boolean;
    saveField: <K extends keyof GuiSettings>(key: K, value: GuiSettings[K]) => Promise<void>;
    applyTheme: (theme: string) => void;
    handleColorPackChange: (colorPack: string) => void;
    handleUiScaleChange: (scaleStr: string) => void;
    tokenKeeperReport: TokenKeeperReport | null;
    tokenKeeperRunning: boolean;
    handleTriggerTokenKeeper: () => void;
    handleShowFloatingCard: () => void;
    handleOpenDataFolder: () => void;
}

export const GeneralSettingsTab: React.FC<GeneralSettingsTabProps> = ({
    settings,
    saving,
    saveField,
    applyTheme,
    handleColorPackChange,
    handleUiScaleChange,
    tokenKeeperReport,
    tokenKeeperRunning,
    handleTriggerTokenKeeper,
    handleShowFloatingCard,
    handleOpenDataFolder,
}) => {
    return (
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
    );
};
