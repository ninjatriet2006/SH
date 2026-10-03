import React, { useState } from 'react';
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
    Layers,
    RotateCcw,
    ChevronDown,
    ChevronRight,
    Search,
    Shield,
    HardDrive,
    Bell,
    Shuffle,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useSettingsStore } from '../store/useSettingsStore';
import { useThemeStore } from '../store/useThemeStore';
import { useTranslation } from '../utils/i18n';
import type { GuiSettings } from '../../../bridge/types';
import { invokeIpc } from '../../../bridge/ipc';

type SettingsTab = 'general' | 'platforms' | 'network' | 'data' | 'about';

export function SettingsPage() {
    const { t } = useTranslation();
    const { settings, updateSettings } = useSettingsStore();
    const { applyTheme } = useThemeStore();

    const [activeTab, setActiveTab] = useState<SettingsTab>('general');
    const [saving, setSaving] = useState(false);
    const [feedback, setFeedback] = useState<{ text: string; ok: boolean } | null>(null);
    const [platformSearch, setPlatformSearch] = useState('');
    const [expandedAccountConfigs, setExpandedAccountConfigs] = useState<Record<string, boolean>>({});

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
            const res = await invokeIpc<any>('import_from_cockpit', {});
            showMsg(res?.message || 'Đã nhập tài khoản từ Cockpit Tools thành công!', true);
        } catch (e: any) {
            showMsg(ipcErrorMessage(e) || 'Lỗi khi nhập tài khoản từ Cockpit', false);
        } finally {
            setSaving(false);
        }
    };

    const toggleAccountConfig = (platform: string) => {
        setExpandedAccountConfigs(prev => ({ ...prev, [platform]: !prev[platform] }));
    };

    // Reusable UI components
    const ToggleSwitch = ({
        checked,
        onChange,
        disabled,
    }: {
        checked: boolean;
        onChange: (val: boolean) => void;
        disabled?: boolean;
    }) => (
        <button
            type="button"
            disabled={disabled}
            onClick={() => onChange(!checked)}
            className={`w-11 h-6 flex items-center rounded-full p-1 transition-colors duration-200 ease-in-out focus:outline-none ${
                checked ? 'bg-blue-600' : 'bg-neutral-700/80 hover:bg-neutral-700'
            } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
        >
            <div
                className={`bg-white w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 ease-in-out ${
                    checked ? 'translate-x-5' : 'translate-x-0'
                }`}
            />
        </button>
    );

    const SelectDropdown = ({
        value,
        options,
        onChange,
    }: {
        value: string | number;
        options: { label: string; value: string | number }[];
        onChange: (val: string) => void;
    }) => (
        <select
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="bg-neutral-950/80 border border-neutral-700/80 rounded-lg px-3 py-1.5 text-xs text-neutral-200 font-medium focus:outline-none focus:border-blue-500 cursor-pointer min-w-[90px]"
        >
            {options.map((opt) => (
                <option key={opt.value} value={opt.value} className="bg-neutral-900 text-neutral-200">
                    {opt.label}
                </option>
            ))}
        </select>
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
        <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
            <div className="min-w-0 flex-1 pr-2">
                <div className="text-sm font-medium text-neutral-200">{label}</div>
                {desc && <div className="text-xs text-neutral-400 mt-0.5">{desc}</div>}
            </div>
            <div className="flex items-center gap-2 flex-wrap md:flex-nowrap justify-end">
                <input
                    type="text"
                    readOnly
                    placeholder={placeholder}
                    value={currentPath || ''}
                    className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-56 lg:w-72 font-mono truncate focus:outline-none focus:border-blue-500"
                />
                <button
                    type="button"
                    onClick={() => handleBrowse(fieldKey)}
                    className="px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-xs text-neutral-200 font-medium border border-neutral-700/80 transition-colors shrink-0"
                >
                    Select
                </button>
                <button
                    type="button"
                    title="Đặt lại về đường dẫn mặc định"
                    onClick={() => handleResetDefault(fieldKey, defaultPath)}
                    className="px-2.5 py-1.5 rounded-lg bg-neutral-800/60 hover:bg-neutral-700 text-xs text-neutral-300 font-medium border border-neutral-700/60 transition-colors flex items-center gap-1 shrink-0"
                >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span className="hidden sm:inline">Reset to default</span>
                </button>
                {detectTarget && (
                    <button
                        type="button"
                        title="Tự động quét tệp .desktop để tìm ứng dụng"
                        onClick={() => handleAutoDetect(detectTarget, fieldKey)}
                        className="px-2.5 py-1.5 rounded-lg bg-blue-600/20 hover:bg-blue-600/30 text-blue-400 text-xs font-medium border border-blue-500/30 transition-colors flex items-center gap-1 shrink-0"
                    >
                        <Search className="w-3.5 h-3.5" />
                        <span className="hidden sm:inline">Quét .desktop</span>
                    </button>
                )}
            </div>
        </div>
    );

    const PlatformCard = ({
        id,
        title,
        children,
    }: {
        id: string;
        title: string;
        children: React.ReactNode;
    }) => (
        <section id={id} className="space-y-2.5">
            <div className="flex items-center gap-2">
                <span className="w-1 h-5 bg-blue-500 rounded-full" />
                <h3 className="text-base font-semibold text-neutral-100">{title}</h3>
            </div>
            <div className="bg-neutral-900/60 border border-neutral-800/80 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                {children}
            </div>
        </section>
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

    return (
        <div className="h-full flex flex-col bg-neutral-950 text-neutral-200">
            {/* Header */}
            <div className="p-6 border-b border-neutral-800 bg-neutral-900/40 shrink-0">
                <div className="flex items-center justify-between">
                    <div>
                        <h1 className="text-xl font-bold text-neutral-100 flex items-center gap-2">
                            <Sliders className="w-5 h-5 text-blue-400" />
                            {t('settings') || 'Cài đặt hệ thống'}
                        </h1>
                        <p className="text-xs text-neutral-400 mt-1">
                            Tùy chỉnh giao diện, mạng, đồng bộ dữ liệu và cấu hình chi tiết cho từng nền tảng AI
                        </p>
                    </div>
                    {saving && (
                        <div className="flex items-center gap-2 px-3 py-1.5 bg-blue-500/10 border border-blue-500/20 text-blue-400 rounded-lg text-xs animate-pulse">
                            <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                            <span>Đang lưu...</span>
                        </div>
                    )}
                </div>

                {/* Feedback Toast */}
                {feedback && (
                    <div
                        className={`mt-4 p-3 rounded-lg border text-xs flex items-center gap-2 transition-all ${
                            feedback.ok
                                ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
                                : 'bg-red-500/10 border-red-500/30 text-red-300'
                        }`}
                    >
                        {feedback.ok ? (
                            <Check className="w-4 h-4 shrink-0 text-emerald-400" />
                        ) : (
                            <AlertCircle className="w-4 h-4 shrink-0 text-red-400" />
                        )}
                        <span>{feedback.text}</span>
                    </div>
                )}

                {/* 5 Main Tabs */}
                <div className="flex gap-2 mt-6 overflow-x-auto pb-1 scrollbar-none">
                    <button
                        onClick={() => setActiveTab('general')}
                        className={`px-4 py-2 rounded-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer whitespace-nowrap ${
                            activeTab === 'general'
                                ? 'bg-blue-600 text-white shadow-sm shadow-blue-500/20'
                                : 'bg-neutral-900/80 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80'
                        }`}
                    >
                        <Sliders className="w-3.5 h-3.5" />
                        <span>Chung (General)</span>
                    </button>
                    <button
                        onClick={() => setActiveTab('platforms')}
                        className={`px-4 py-2 rounded-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer whitespace-nowrap ${
                            activeTab === 'platforms'
                                ? 'bg-blue-600 text-white shadow-sm shadow-blue-500/20'
                                : 'bg-neutral-900/80 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80'
                        }`}
                    >
                        <Layers className="w-3.5 h-3.5" />
                        <span>Nền tảng (Platforms)</span>
                    </button>
                    <button
                        onClick={() => setActiveTab('network')}
                        className={`px-4 py-2 rounded-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer whitespace-nowrap ${
                            activeTab === 'network'
                                ? 'bg-blue-600 text-white shadow-sm shadow-blue-500/20'
                                : 'bg-neutral-900/80 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80'
                        }`}
                    >
                        <Globe className="w-3.5 h-3.5" />
                        <span>Mạng (Network)</span>
                    </button>
                    <button
                        onClick={() => setActiveTab('data')}
                        className={`px-4 py-2 rounded-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer whitespace-nowrap ${
                            activeTab === 'data'
                                ? 'bg-blue-600 text-white shadow-sm shadow-blue-500/20'
                                : 'bg-neutral-900/80 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80'
                        }`}
                    >
                        <Database className="w-3.5 h-3.5" />
                        <span>Dữ liệu & Sao lưu (Data)</span>
                    </button>
                    <button
                        onClick={() => setActiveTab('about')}
                        className={`px-4 py-2 rounded-lg text-xs font-medium flex items-center gap-2 transition-all cursor-pointer whitespace-nowrap ${
                            activeTab === 'about'
                                ? 'bg-blue-600 text-white shadow-sm shadow-blue-500/20'
                                : 'bg-neutral-900/80 text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/80'
                        }`}
                    >
                        <Info className="w-3.5 h-3.5" />
                        <span>Thông tin (About)</span>
                    </button>
                </div>
            </div>

            {/* Tab Contents */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6 max-w-5xl mx-auto w-full">
                {/* TAB 1: GENERAL */}
                {activeTab === 'general' && (
                    <div className="space-y-6">
                        {/* Section 1: Giao diện & Hiển thị */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Sparkles className="w-4 h-4 text-purple-400" />
                                Giao diện & Trải nghiệm
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                {/* Ngôn ngữ */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Ngôn ngữ (Language)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Chọn ngôn ngữ hiển thị giao diện</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.language || 'vi'}
                                        options={[
                                            { label: 'Tiếng Việt', value: 'vi' },
                                            { label: 'English', value: 'en' },
                                            { label: '简体中文', value: 'zh' },
                                        ]}
                                        onChange={(v) => saveField('language', v)}
                                    />
                                </div>

                                {/* Chủ đề */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Chủ đề (Theme)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Chuyển đổi giữa giao diện Sáng, Tối hoặc Hệ thống</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.theme || 'dark'}
                                        options={[
                                            { label: 'Tối (Dark)', value: 'dark' },
                                            { label: 'Sáng (Light)', value: 'light' },
                                            { label: 'Hệ thống (System)', value: 'system' },
                                        ]}
                                        onChange={(v) => {
                                            void saveField('theme', v);
                                            applyTheme(v);
                                        }}
                                    />
                                </div>

                                {/* Gói màu sắc */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Gói màu sắc giao diện (Color Pack)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tùy biến bảng màu sắc chủ đạo của ứng dụng</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.color_pack || 'default'}
                                        options={[
                                            { label: 'Mặc định (Default)', value: 'default' },
                                            { label: 'Emerald (Ngọc lục bảo)', value: 'emerald' },
                                            { label: 'Sapphire (Xanh lam)', value: 'sapphire' },
                                            { label: 'Violet (Tím)', value: 'violet' },
                                            { label: 'Amber (Hổ phách)', value: 'amber' },
                                            { label: 'Rose (Hoa hồng)', value: 'rose' },
                                        ]}
                                        onChange={handleColorPackChange}
                                    />
                                </div>

                                {/* Tỉ lệ UI */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Tỉ lệ giao diện (UI Scale)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Điều chỉnh kích thước tổng thể các thành phần giao diện</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.ui_scale || 1.0}
                                        options={[
                                            { label: '75%', value: 0.75 },
                                            { label: '85%', value: 0.85 },
                                            { label: '90%', value: 0.9 },
                                            { label: '100% (Chuẩn)', value: 1.0 },
                                            { label: '110%', value: 1.1 },
                                            { label: '125%', value: 1.25 },
                                            { label: '150%', value: 1.5 },
                                        ]}
                                        onChange={handleUiScaleChange}
                                    />
                                </div>

                                {/* Giảm hiệu ứng động */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Giảm hiệu ứng động (Reduce Motion)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tắt hoạt ảnh chuyển trang và chuyển động để tối ưu hiệu năng</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.reduced_motion_enabled}
                                        onChange={(val) => saveField('reduced_motion_enabled', val)}
                                    />
                                </div>

                                {/* Bố cục thanh bên */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Bố cục thanh bên (Sidebar Layout)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Lựa chọn hiển thị thanh điều hướng bên trái</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.side_nav_layout_mode || 'classic'}
                                        options={[
                                            { label: 'Cổ điển (Classic)', value: 'classic' },
                                            { label: 'Nguyên bản (Original)', value: 'original' },
                                            { label: 'Tối giản (Compact)', value: 'compact' },
                                        ]}
                                        onChange={(v) => saveField('side_nav_layout_mode', v)}
                                    />
                                </div>

                                {/* Trang mở đầu */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Trang mở đầu (Startup Page)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Trang hiển thị mặc định khi ứng dụng khởi chạy</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.startup_page || 'last'}
                                        options={[
                                            { label: 'Ghi nhớ trang trước (Remember last)', value: 'last' },
                                            { label: 'Tổng quan (Dashboard)', value: 'dashboard' },
                                            { label: 'Tài khoản (Accounts)', value: 'accounts' },
                                            { label: 'Thử nghiệm (Playground)', value: 'playground' },
                                            { label: 'Phiên làm việc (Sessions)', value: 'sessions' },
                                            { label: 'Cài đặt (Settings)', value: 'settings' },
                                        ]}
                                        onChange={(v) => saveField('startup_page', v)}
                                    />
                                </div>

                                {/* Hiển thị thông báo khuyến mãi / Top Promo */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hiển thị thông báo đỉnh trang (Show Top Promo)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Bật/tắt thanh banner quảng bá tính năng mới ở trên cùng</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.show_top_promo}
                                        onChange={(val) => saveField('show_top_promo', val)}
                                    />
                                </div>
                            </div>
                        </section>

                        {/* Section 2: Cửa sổ & Hành vi hệ thống */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <HardDrive className="w-4 h-4 text-emerald-400" />
                                Cửa sổ & Hệ thống
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                {/* Hành vi khi đóng */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hành vi khi đóng cửa sổ (Close Behavior)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Lựa chọn đóng cửa sổ vào khay hệ thống hay thoát hoàn toàn</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.close_behavior || 'minimize'}
                                        options={[
                                            { label: 'Thu nhỏ vào khay (Minimize to tray)', value: 'minimize' },
                                            { label: 'Thoát hẳn ứng dụng (Exit)', value: 'exit' },
                                        ]}
                                        onChange={(v) => saveField('close_behavior', v)}
                                    />
                                </div>

                                {/* Khởi động cùng hệ thống */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Khởi động cùng máy tính (Launch at Login)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động khởi động ứng dụng ngầm sau khi đăng nhập hệ thống</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.app_auto_launch_enabled}
                                        onChange={(val) => saveField('app_auto_launch_enabled', val)}
                                    />
                                </div>

                                {/* Khởi động thu nhỏ */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Khởi động ở chế độ thu nhỏ (Start Minimized)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Ẩn cửa sổ chính vào khay hệ thống khi ứng dụng bắt đầu</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.startup_minimized}
                                        onChange={(val) => saveField('startup_minimized', val)}
                                    />
                                </div>

                                {/* Ghi nhớ kích thước cửa sổ */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Ghi nhớ kích thước & vị trí cửa sổ (Remember Window State)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Khôi phục đúng kích thước và tọa độ cửa sổ từ phiên trước</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.remember_main_window_state}
                                        onChange={(val) => saveField('remember_main_window_state', val)}
                                    />
                                </div>

                                {/* Terminal mặc định */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Terminal mặc định (Default Terminal)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Trình giả lập dòng lệnh dùng khi mở terminal từ ứng dụng</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.default_terminal || 'system'}
                                        options={[
                                            { label: 'Mặc định hệ thống (System Default)', value: 'system' },
                                            { label: 'Bash', value: 'bash' },
                                            { label: 'Zsh', value: 'zsh' },
                                            { label: 'Fish', value: 'fish' },
                                            { label: 'Kitty', value: 'kitty' },
                                            { label: 'Alacritty', value: 'alacritty' },
                                            { label: 'WezTerm', value: 'wezterm' },
                                            { label: 'Konsole', value: 'konsole' },
                                            { label: 'Gnome Terminal', value: 'gnome-terminal' },
                                        ]}
                                        onChange={(v) => saveField('default_terminal', v)}
                                    />
                                </div>
                            </div>
                        </section>

                        {/* Section 3: Thẻ tài khoản nổi (Floating Card) */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Layers className="w-4 h-4 text-cyan-400" />
                                Thẻ tài khoản nổi (Floating Card)
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hiện thẻ nổi khi khởi động</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động bật widget thẻ tài khoản nổi mini trên màn hình</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.floating_card_show_on_startup}
                                        onChange={(val) => saveField('floating_card_show_on_startup', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Thẻ nổi luôn trên cùng (Always on top)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Giữ widget thẻ tài khoản nổi nằm trên các cửa sổ IDE khác</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.floating_card_always_on_top}
                                        onChange={(val) => saveField('floating_card_always_on_top', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Mở thẻ tài khoản nổi ngay bây giờ</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Kích hoạt cửa sổ thẻ nổi để theo dõi trạng thái tài khoản</div>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={handleShowFloatingCard}
                                        className="px-3.5 py-1.5 rounded-lg bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 text-xs font-medium border border-cyan-500/30 transition-colors"
                                    >
                                        Mở thẻ nổi
                                    </button>
                                </div>
                            </div>
                        </section>

                        {/* Section 4: Tự động chuyển đổi tài khoản (Auto Switch & Failover) */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Shuffle className="w-4 h-4 text-orange-400" />
                                Tự động chuyển đổi tài khoản (Auto Switch & Failover)
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Bật tự động chuyển đổi khi hết hạn mức (Auto Switch)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động tráo sang tài khoản khả dụng khác khi tài khoản hiện tại cạn hạn mức</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.auto_switch_enabled}
                                        onChange={(val) => saveField('auto_switch_enabled', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Ngưỡng kích hoạt chuyển đổi (% còn lại)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Kích hoạt chuyển tài khoản khi hạn mức tụt xuống dưới ngưỡng này</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.auto_switch_threshold || 5}
                                        options={[
                                            { label: '3% còn lại', value: 3 },
                                            { label: '5% còn lại (Mặc định)', value: 5 },
                                            { label: '10% còn lại', value: 10 },
                                            { label: '15% còn lại', value: 15 },
                                            { label: '20% còn lại', value: 20 },
                                        ]}
                                        onChange={(v) => saveField('auto_switch_threshold', parseInt(v, 10))}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Phạm vi tài khoản tham gia (Account Scope)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Chỉ quét và luân chuyển giữa các tài khoản nằm trong phạm vi chỉ định</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.auto_switch_scope_mode || 'any_group'}
                                        options={[
                                            { label: 'Tất cả tài khoản (All accounts)', value: 'any_group' },
                                            { label: 'Theo nhóm tài khoản (Group)', value: 'group_only' },
                                            { label: 'Chỉ tài khoản được chọn (Selected)', value: 'selected' },
                                        ]}
                                        onChange={(v) => saveField('auto_switch_scope_mode', v)}
                                    />
                                </div>
                            </div>
                        </section>

                        {/* Section 5: Hạn mức trên khay hệ thống (Menu Bar Quota) */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Bell className="w-4 h-4 text-yellow-400" />
                                Hạn mức trên khay hệ thống (Menu Bar Quota)
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hiển thị hạn mức trực tiếp trên khay (Show live quota in menu bar)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Hiển thị % hạn mức còn lại của tài khoản hiện tại ngay cạnh icon khay</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.menu_bar_quota_enabled}
                                        onChange={(val) => saveField('menu_bar_quota_enabled', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Nền tảng theo dõi trên khay hệ thống</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Chọn nền tảng AI bạn muốn hiển thị % hạn mức nhanh</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.menu_bar_quota_platform || 'codex'}
                                        options={[
                                            { label: 'Codex', value: 'codex' },
                                            { label: 'Antigravity IDE', value: 'antigravity' },
                                            { label: 'Claude', value: 'claude' },
                                            { label: 'GitHub Copilot', value: 'ghcp' },
                                            { label: 'Devin / Windsurf', value: 'windsurf' },
                                            { label: 'Cursor', value: 'cursor' },
                                            { label: 'Zed', value: 'zed' },
                                        ]}
                                        onChange={(v) => saveField('menu_bar_quota_platform', v)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hiển thị tiền tố 4 ký tự email (Show first 4 email chars)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Hiện tên viết tắt tài khoản cùng số % quota trên thanh khay</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.menu_bar_show_account_prefix}
                                        onChange={(val) => saveField('menu_bar_show_account_prefix', val)}
                                    />
                                </div>
                            </div>
                        </section>

                        {/* Section 6: Tài khoản cục bộ & Duy trì phiên */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Shield className="w-4 h-4 text-blue-400" />
                                Phiên làm việc & Thư mục ứng dụng
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Duy trì phiên đăng nhập (Auth Keep-Alive)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động làm mới token phiên theo từng lô nhỏ khi sắp hết hạn</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.token_keeper_enabled ?? true}
                                        onChange={(val) => saveField('token_keeper_enabled', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Tự động quét tài khoản cục bộ (Auto-import local accounts)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động phát hiện và import tài khoản khi bạn đăng nhập trên client chính thức</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.auto_import_from_local_enabled}
                                        onChange={(val) => saveField('auto_import_from_local_enabled', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Thư mục dữ liệu cấu hình ứng dụng</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Mở thư mục lưu trữ tài khoản, cache và tệp cấu hình trên hệ thống</div>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={handleOpenDataFolder}
                                        className="px-3.5 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium border border-neutral-700 transition-colors flex items-center gap-1.5"
                                    >
                                        <FolderOpen className="w-3.5 h-3.5 text-blue-400" />
                                        Mở thư mục
                                    </button>
                                </div>
                            </div>
                        </section>
                    </div>
                )}

                {/* TAB 2: PLATFORMS (Cài đặt từng nền tảng chuẩn Cockpit 1:1) */}
                {activeTab === 'platforms' && (
                    <div className="space-y-6">
                        {/* Search & Quick Jumper */}
                        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-neutral-900/40 p-3 rounded-xl border border-neutral-800/80">
                            <div className="relative flex-1">
                                <Search className="w-4 h-4 text-neutral-400 absolute left-3 top-1/2 -translate-y-1/2" />
                                <input
                                    type="text"
                                    value={platformSearch}
                                    onChange={(e) => setPlatformSearch(e.target.value)}
                                    placeholder="Tìm kiếm nền tảng (Claude, Zed, GitHub Copilot, Devin, Antigravity, Codex...)"
                                    className="w-full bg-neutral-950/80 border border-neutral-800 text-xs pl-9 pr-3 py-2 rounded-lg text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-blue-500"
                                />
                            </div>
                            <div className="flex items-center gap-1.5 overflow-x-auto scrollbar-none text-xs text-neutral-400">
                                <span className="text-[11px] uppercase tracking-wider text-neutral-500 mr-1">Nhảy tới:</span>
                                {['claude', 'zed', 'ghcp', 'windsurf', 'antigravity', 'codex'].map((id) => (
                                    <a
                                        key={id}
                                        href={`#platform-${id}`}
                                        className="px-2 py-1 bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded text-[11px] transition-colors whitespace-nowrap"
                                    >
                                        {id.toUpperCase()}
                                    </a>
                                ))}
                            </div>
                        </div>

                        {/* 1. CLAUDE SETTINGS (Screenshot 1) */}
                        {(!platformSearch || 'claude'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-claude" title="Claude Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Claude quota auto refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Automatically refresh cached quota for Claude accounts in the background.
                                        </div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.claude_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('claude_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Refresh current account only. Default is 1 minute.
                                        </div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.claude_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('claude_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Claude quota as remaining %</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Default shows used percentage; enable to show remaining. Auto-switch and alerts still use used ratio.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.claude_quota_display_remaining}
                                        onChange={(val) => saveField('claude_quota_display_remaining', val)}
                                    />
                                </div>

                                <PathRow
                                    label="Claude Desktop launch target"
                                    desc="The default profile can use a Microsoft Store target; multi-instance profiles need the real Claude.exe."
                                    fieldKey="claude_app_path"
                                    currentPath={settings.claude_app_path}
                                    placeholder="Claude.exe path or shell:AppsFolder\..."
                                    detectTarget="claude"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            When any current-account model quota drops below the threshold, send a native notification and show a quick-switch action in the app.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.claude_quota_alert_enabled}
                                        onChange={(val) => saveField('claude_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 2. ZED SETTINGS (Screenshot 2) */}
                        {(!platformSearch || 'zed'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-zed" title="Zed Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Zed Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.zed_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('zed_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Refresh current account only. Default is 1 minute.</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.zed_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('zed_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <PathRow
                                    label="Zed Launch Path"
                                    desc="Leave empty to use default path"
                                    fieldKey="zed_app_path"
                                    currentPath={settings.zed_app_path}
                                    detectTarget="zed"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            When any current-account model quota drops below the threshold, send a native notification and show a quick-switch action in the app.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.zed_quota_alert_enabled}
                                        onChange={(val) => saveField('zed_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 3. GITHUB COPILOT SETTINGS (Screenshot 3) */}
                        {(!platformSearch || 'github copilot ghcp'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-ghcp" title="GitHub Copilot Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">GitHub Copilot Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.ghcp_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('ghcp_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Refresh current account only. Default is 1 minute.</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.ghcp_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('ghcp_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                {/* 账号级刷新配置 (Account-level refresh configuration) */}
                                <div className="p-4 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex items-center justify-between gap-4">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">账号级刷新配置</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">
                                                为不同账号设置不同的自动刷新间隔，覆盖平台级默认值。
                                            </div>
                                        </div>
                                        <button
                                            type="button"
                                            onClick={() => toggleAccountConfig('ghcp')}
                                            className="px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium border border-neutral-700 flex items-center gap-1.5 transition-colors"
                                        >
                                            {expandedAccountConfigs['ghcp'] ? (
                                                <>
                                                    <ChevronDown className="w-3.5 h-3.5" />
                                                    <span>收起配置</span>
                                                </>
                                            ) : (
                                                <>
                                                    <ChevronRight className="w-3.5 h-3.5" />
                                                    <span>展开配置</span>
                                                </>
                                            )}
                                        </button>
                                    </div>
                                    {expandedAccountConfigs['ghcp'] && (
                                        <div className="mt-3 p-3 bg-neutral-950/80 rounded-lg border border-neutral-800 text-xs text-neutral-400 space-y-2">
                                            <div className="text-neutral-300 font-medium">Danh sách tần suất làm mới theo tài khoản Copilot:</div>
                                            <div className="text-[11px] text-neutral-500 italic">
                                                Hệ thống áp dụng tần suất làm mới mặc định ({settings.ghcp_auto_refresh_minutes || 10} phút) cho tất cả tài khoản. Có thể tinh chỉnh riêng từng tài khoản trong trang Quản lý tài khoản.
                                            </div>
                                        </div>
                                    )}
                                </div>

                                <PathRow
                                    label="VS Code App Path"
                                    desc="Leave empty to use default path"
                                    fieldKey="vscode_app_path"
                                    currentPath={settings.vscode_app_path}
                                    defaultPath="/usr/bin/code"
                                    detectTarget="vscode"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Launch GitHub Copilot App on switch</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Khởi chạy VS Code khi thực hiện chuyển đổi tài khoản GitHub Copilot</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.ghcp_launch_on_switch ?? true}
                                        onChange={(val) => saveField('ghcp_launch_on_switch', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            When any current-account model quota drops below the threshold, send a native notification and show a quick-switch action in the app.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.ghcp_quota_alert_enabled}
                                        onChange={(val) => saveField('ghcp_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 4. DEVIN / WINDSURF SETTINGS (Screenshot 4) */}
                        {(!platformSearch || 'devin windsurf'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-windsurf" title="Devin Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Devin Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.windsurf_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('windsurf_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Refresh current account only. Default is 1 minute.</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.windsurf_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('windsurf_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <PathRow
                                    label="Devin App Path"
                                    desc="Leave empty to use default path"
                                    fieldKey="windsurf_app_path"
                                    currentPath={settings.windsurf_app_path}
                                    detectTarget="windsurf"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            When any current-account model quota drops below the threshold, send a native notification and show a quick-switch action in the app.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.windsurf_quota_alert_enabled}
                                        onChange={(val) => saveField('windsurf_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 5. ANTIGRAVITY IDE SETTINGS */}
                        {(!platformSearch || 'antigravity'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-antigravity" title="Antigravity IDE Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Antigravity Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tần suất quét và cập nhật hạn mức Antigravity chạy nền</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.antigravity_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('antigravity_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tần suất làm mới riêng tài khoản đang hoạt động</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.antigravity_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('antigravity_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <PathRow
                                    label="Antigravity IDE App Path"
                                    desc="Đường dẫn đến bản Antigravity IDE chuyên dụng"
                                    fieldKey="antigravity_app_path"
                                    currentPath={settings.antigravity_app_path}
                                    defaultPath="/home/bimatkeo/Applications/antigravity-ide/antigravity-ide"
                                    detectTarget="antigravity_ide"
                                />

                                <PathRow
                                    label="Antigravity Desktop App Path"
                                    desc="Đường dẫn đến bản Antigravity Client thông thường"
                                    fieldKey="antigravity_desktop_app_path"
                                    currentPath={settings.antigravity_desktop_app_path}
                                    detectTarget="antigravity_desktop"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Launch Antigravity on switch</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Khởi động lại Antigravity khi hoàn tất chuyển đổi tài khoản</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.antigravity_launch_on_switch ?? true}
                                        onChange={(val) => saveField('antigravity_launch_on_switch', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Dual switch without restart</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Chuyển đổi liền mạch dữ liệu tài khoản và gọi extension trong 1 luồng duy nhất mà không cần tắt/bật lại IDE
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.antigravity_dual_switch_no_restart_enabled}
                                        onChange={(val) => saveField('antigravity_dual_switch_no_restart_enabled', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Run Antigravity IDE wakeup after startup</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động chạy các tác vụ đánh thức phiên Antigravity khi mở ứng dụng</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.antigravity_startup_wakeup_enabled}
                                        onChange={(val) => saveField('antigravity_startup_wakeup_enabled', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Gửi thông báo hệ thống khi quota Antigravity giảm thấp</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.antigravity_quota_alert_enabled}
                                        onChange={(val) => saveField('antigravity_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 6. CODEX SETTINGS */}
                        {(!platformSearch || 'codex'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-codex" title="Codex Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Codex Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.codex_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('codex_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Current Account Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Refresh current account only. Default is 1 minute.</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.codex_current_account_refresh_minutes ?? 1}
                                        options={currentAccountRefreshOptions}
                                        onChange={(v) => saveField('codex_current_account_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>

                                <PathRow
                                    label="Codex App Path"
                                    desc="Leave empty to use default path"
                                    fieldKey="codex_app_path"
                                    currentPath={settings.codex_app_path}
                                    detectTarget="codex"
                                />

                                <PathRow
                                    label="Specified App Path"
                                    desc="Đường dẫn ứng dụng phụ cần tự động khởi động lại cùng khi chuyển Codex"
                                    fieldKey="codex_specified_app_path"
                                    currentPath={settings.codex_specified_app_path}
                                    placeholder="Ví dụ: /Applications/Host.app"
                                />

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Đồng bộ môi trường WSL (Sync WSL)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Đồng bộ token và cấu hình Codex sang môi trường Windows Subsystem for Linux</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.codex_sync_wsl}
                                        onChange={(val) => saveField('codex_sync_wsl', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Tiêm giao diện UI ứng dụng Codex (App UI Injection)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động chèn nút chuyển tài khoản trực tiếp vào giao diện Codex</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.codex_app_ui_injection_enabled ?? true}
                                        onChange={(val) => saveField('codex_app_ui_injection_enabled', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Hiển thị lối vào API cục bộ (Local Access Entry Visible)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Bật mục điều hướng đến dịch vụ Local API Gateway trên trang Codex</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.codex_local_access_entry_visible ?? true}
                                        onChange={(val) => saveField('codex_local_access_entry_visible', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Ẩn hạn mức Relay (Hide Relay Quota)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Không hiển thị hạn mức thẻ tiếp sức trong thẻ tài khoản Codex</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.codex_hide_relay_quota}
                                        onChange={(val) => saveField('codex_hide_relay_quota', val)}
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Gửi thông báo hệ thống khi quota Codex xuống dưới ngưỡng</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.codex_quota_alert_enabled}
                                        onChange={(val) => saveField('codex_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 7. CODEBUDDY & CODEBUDDY CN SETTINGS */}
                        {(!platformSearch || 'codebuddy'.includes(platformSearch.toLowerCase())) && (
                            <>
                                <PlatformCard id="platform-codebuddy" title="CodeBuddy Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">CodeBuddy Auto Refresh Quota</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background auto-refresh interval</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.codebuddy_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('codebuddy_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="CodeBuddy Launch Path"
                                        desc="Leave empty for default path"
                                        fieldKey="codebuddy_app_path"
                                        currentPath={settings.codebuddy_app_path}
                                        detectTarget="codebuddy"
                                    />
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Share Local Sessions on Switch</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">
                                                Merge local sessions and restore state between CodeBuddy accounts on this device.
                                            </div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.codebuddy_share_sessions_on_switch}
                                            onChange={(val) => saveField('codebuddy_share_sessions_on_switch', val)}
                                        />
                                    </div>
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức CodeBuddy</div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.codebuddy_quota_alert_enabled}
                                            onChange={(val) => saveField('codebuddy_quota_alert_enabled', val)}
                                        />
                                    </div>
                                </PlatformCard>

                                <PlatformCard id="platform-codebuddy-cn" title="CodeBuddy CN Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">CodeBuddy CN Auto Refresh</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background auto-refresh frequency</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.codebuddy_cn_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('codebuddy_cn_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="CodeBuddy CN App Path"
                                        desc="Leave blank to use the default path"
                                        fieldKey="codebuddy_cn_app_path"
                                        currentPath={settings.codebuddy_cn_app_path}
                                        detectTarget="codebuddy_cn"
                                    />
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Share Local Sessions on Switch</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">
                                                Merge local sessions between CodeBuddy CN accounts.
                                            </div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.codebuddy_cn_share_sessions_on_switch}
                                            onChange={(val) => saveField('codebuddy_cn_share_sessions_on_switch', val)}
                                        />
                                    </div>
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức CodeBuddy CN</div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.codebuddy_cn_quota_alert_enabled}
                                            onChange={(val) => saveField('codebuddy_cn_quota_alert_enabled', val)}
                                        />
                                    </div>
                                </PlatformCard>
                            </>
                        )}

                        {/* 8. WORKBUDDY SETTINGS */}
                        {(!platformSearch || 'workbuddy'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-workbuddy" title="WorkBuddy Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">WorkBuddy Auto Refresh</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background auto-refresh frequency</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.workbuddy_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('workbuddy_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>
                                <PathRow
                                    label="WorkBuddy Launch Path"
                                    desc="Leave empty for default path"
                                    fieldKey="workbuddy_app_path"
                                    currentPath={settings.workbuddy_app_path}
                                    detectTarget="workbuddy"
                                />
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Share Local Sessions on Switch</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Merge local sessions and restore state between WorkBuddy accounts on this device.
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.workbuddy_share_sessions_on_switch ?? true}
                                        onChange={(val) => saveField('workbuddy_share_sessions_on_switch', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức WorkBuddy</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.workbuddy_quota_alert_enabled}
                                        onChange={(val) => saveField('workbuddy_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 9. TRAE SETTINGS */}
                        {(!platformSearch || 'trae'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-trae" title="Trae Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Trae Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Background auto-refresh interval</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.trae_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('trae_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>
                                <PathRow
                                    label="Trae Launch Path"
                                    desc="Leave empty for default path"
                                    fieldKey="trae_app_path"
                                    currentPath={settings.trae_app_path}
                                    detectTarget="trae"
                                />
                                <PathRow
                                    label="TRAE SOLO Launch Path"
                                    desc="Đường dẫn đến bản TRAE SOLO"
                                    fieldKey="trae_solo_app_path"
                                    currentPath={settings.trae_solo_app_path}
                                    detectTarget="trae_solo"
                                />
                                <PathRow
                                    label="Trae CN Launch Path"
                                    desc="Đường dẫn đến bản Trae CN nội địa"
                                    fieldKey="trae_cn_app_path"
                                    currentPath={settings.trae_cn_app_path}
                                    detectTarget="trae_cn"
                                />
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Share Local Sessions on Switch</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Chia sẻ phiên làm việc giữa các tài khoản Trae</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.trae_share_sessions_on_switch}
                                        onChange={(val) => saveField('trae_share_sessions_on_switch', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức Trae</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.trae_quota_alert_enabled}
                                        onChange={(val) => saveField('trae_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}

                        {/* 10. CURSOR & KIRO */}
                        {(!platformSearch || 'cursor kiro'.includes(platformSearch.toLowerCase())) && (
                            <>
                                <PlatformCard id="platform-cursor" title="Cursor Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Cursor Auto Refresh Quota</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.cursor_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('cursor_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="Cursor App Path"
                                        desc="Leave empty to use default path"
                                        fieldKey="cursor_app_path"
                                        currentPath={settings.cursor_app_path}
                                        detectTarget="cursor"
                                    />
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức Cursor</div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.cursor_quota_alert_enabled}
                                            onChange={(val) => saveField('cursor_quota_alert_enabled', val)}
                                        />
                                    </div>
                                </PlatformCard>

                                <PlatformCard id="platform-kiro" title="Kiro Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Kiro Auto Refresh Quota</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background update frequency</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.kiro_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('kiro_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="Kiro App Path"
                                        desc="Leave empty to use default path"
                                        fieldKey="kiro_app_path"
                                        currentPath={settings.kiro_app_path}
                                        detectTarget="kiro"
                                    />
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức Kiro</div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.kiro_quota_alert_enabled}
                                            onChange={(val) => saveField('kiro_quota_alert_enabled', val)}
                                        />
                                    </div>
                                </PlatformCard>
                            </>
                        )}

                        {/* 11. QODER & ZCODE */}
                        {(!platformSearch || 'qoder zcode'.includes(platformSearch.toLowerCase())) && (
                            <>
                                <PlatformCard id="platform-qoder" title="Qoder Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Qoder Auto Refresh</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background auto-refresh frequency</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.qoder_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('qoder_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="Qoder App Path"
                                        desc="Leave blank to use the default path"
                                        fieldKey="qoder_app_path"
                                        currentPath={settings.qoder_app_path}
                                        detectTarget="qoder"
                                    />
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức Qoder</div>
                                        </div>
                                        <ToggleSwitch
                                            checked={!!settings.qoder_quota_alert_enabled}
                                            onChange={(val) => saveField('qoder_quota_alert_enabled', val)}
                                        />
                                    </div>
                                </PlatformCard>

                                <PlatformCard id="platform-zcode" title="ZCode Settings">
                                    <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                        <div>
                                            <div className="text-sm font-medium text-neutral-200">ZCode Auto Refresh</div>
                                            <div className="text-xs text-neutral-400 mt-0.5">Background quota refresh interval</div>
                                        </div>
                                        <SelectDropdown
                                            value={settings.zcode_auto_refresh_minutes ?? 10}
                                            options={quotaRefreshOptions}
                                            onChange={(v) => saveField('zcode_auto_refresh_minutes', parseInt(v, 10))}
                                        />
                                    </div>
                                    <PathRow
                                        label="ZCode Launch Path"
                                        desc="Leave blank to use the default path"
                                        fieldKey="zcode_app_path"
                                        currentPath={settings.zcode_app_path}
                                        detectTarget="zcode"
                                    />
                                </PlatformCard>
                            </>
                        )}

                        {/* 12. GROK CLI SETTINGS */}
                        {(!platformSearch || 'grok'.includes(platformSearch.toLowerCase())) && (
                            <PlatformCard id="platform-grok" title="Grok CLI Settings">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Grok Auto Refresh Quota</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tần suất làm mới hạn mức Grok CLI</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.grok_auto_refresh_minutes ?? 10}
                                        options={quotaRefreshOptions}
                                        onChange={(v) => saveField('grok_auto_refresh_minutes', parseInt(v, 10))}
                                    />
                                </div>
                                <PathRow
                                    label="Grok CLI Path"
                                    desc="Đường dẫn đến tệp thực thi grok cli"
                                    fieldKey="grok_cli_path"
                                    currentPath={settings.grok_cli_path}
                                    detectTarget="grok"
                                />
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Sync official login on switch</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Ghi thông tin đăng nhập vào ~/.grok/auth.json chính thức khi chuyển tài khoản
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.grok_sync_official_auth_on_switch}
                                        onChange={(val) => saveField('grok_sync_official_auth_on_switch', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Enable Quota Alert</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Cảnh báo hạn mức Grok CLI</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.grok_quota_alert_enabled}
                                        onChange={(val) => saveField('grok_quota_alert_enabled', val)}
                                    />
                                </div>
                            </PlatformCard>
                        )}
                    </div>
                )}

                {/* TAB 3: NETWORK */}
                {activeTab === 'network' && (
                    <div className="space-y-6">
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Globe className="w-4 h-4 text-blue-400" />
                                Cấu hình Proxy & Mạng kết nối
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                {/* Proxy Toggle */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Sử dụng Proxy toàn cục (Global Proxy)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Định tuyến mọi yêu cầu API qua máy chủ Proxy chỉ định</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.global_proxy_enabled}
                                        onChange={(val) => saveField('global_proxy_enabled', val)}
                                    />
                                </div>

                                {/* Proxy URL */}
                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Địa chỉ Proxy (Proxy URL)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Hỗ trợ giao thức HTTP, HTTPS hoặc SOCKS5</div>
                                    </div>
                                    <input
                                        type="text"
                                        placeholder="http://127.0.0.1:7890 hoặc socks5://127.0.0.1:1080"
                                        defaultValue={settings.global_proxy_url || ''}
                                        onBlur={(e) => saveField('global_proxy_url', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500 font-mono"
                                    />
                                </div>

                                {/* No Proxy list */}
                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Bỏ qua Proxy (Bypass List)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Các địa chỉ IP/domain không đi qua proxy (ngăn cách bằng dấu phẩy)</div>
                                    </div>
                                    <input
                                        type="text"
                                        defaultValue={settings.global_proxy_no_proxy || '127.0.0.1,localhost,::1'}
                                        onBlur={(e) => saveField('global_proxy_no_proxy', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500 font-mono"
                                    />
                                </div>

                                {/* Cho phép truy cập mạng ngoài */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Cho phép truy cập mạng ngoài (Allow external network)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Khi tắt, chặn WebDAV, kiểm tra cập nhật và làm mới OpenRouter (chỉ tính năng nội bộ hoạt động)
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.allow_external_network ?? true}
                                        onChange={(val) => saveField('allow_external_network', val)}
                                    />
                                </div>

                                {/* Danh sách tên miền WebDAV được phép */}
                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Danh sách tên miền WebDAV được phép (Domain allowlist)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Ngăn cách bởi dấu phẩy; để trống = không giới hạn</div>
                                    </div>
                                    <input
                                        type="text"
                                        placeholder="dav.jianguoyun.com, mydav.example.com"
                                        defaultValue={settings.webdav_allowed_domains || ''}
                                        onBlur={(e) => saveField('webdav_allowed_domains', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500 font-mono"
                                    />
                                </div>

                                {/* Cổng WebSocket cục bộ */}
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Cổng WebSocket cục bộ (Local Port)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Cổng dịch vụ nền phục vụ giao tiếp giữa extension và ứng dụng</div>
                                    </div>
                                    <input
                                        type="number"
                                        defaultValue={settings.ws_port || 19528}
                                        onBlur={(e) => saveField('ws_port', parseInt(e.target.value, 10))}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-28 text-center font-mono focus:outline-none focus:border-blue-500"
                                    />
                                </div>
                            </div>
                        </section>
                    </div>
                )}

                {/* TAB 4: DATA & BACKUP */}
                {activeTab === 'data' && (
                    <div className="space-y-6">
                        {/* Section 1: Tự động sao lưu cục bộ (Local Auto Backup) */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <HardDrive className="w-4 h-4 text-emerald-400" />
                                Tự động sao lưu cục bộ (Local Auto Backup)
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Bật tự động sao lưu định kỳ (Auto Backup)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Tự động nén tệp sao lưu .json và .zip chứa dữ liệu tài khoản vào thư mục lưu trữ
                                        </div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.auto_backup_enabled ?? true}
                                        onChange={(val) => saveField('auto_backup_enabled', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Bao gồm danh sách tài khoản (Include Accounts)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Sao lưu token và thông tin các tài khoản AI</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.auto_backup_include_accounts ?? true}
                                        onChange={(val) => saveField('auto_backup_include_accounts', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Bao gồm tệp cài đặt (Include Config)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Sao lưu toàn bộ tùy chọn thiết lập hệ thống</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={settings.auto_backup_include_config ?? true}
                                        onChange={(val) => saveField('auto_backup_include_config', val)}
                                    />
                                </div>
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Thời gian lưu trữ bản sao lưu (Retention Days)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động dọn dẹp các tệp sao lưu cũ hơn thời hạn chỉ định</div>
                                    </div>
                                    <SelectDropdown
                                        value={settings.auto_backup_retention_days || 15}
                                        options={[
                                            { label: '7 ngày', value: 7 },
                                            { label: '15 ngày (Mặc định)', value: 15 },
                                            { label: '30 ngày', value: 30 },
                                            { label: '60 ngày', value: 60 },
                                        ]}
                                        onChange={(v) => saveField('auto_backup_retention_days', parseInt(v, 10))}
                                    />
                                </div>
                            </div>
                        </section>

                        {/* Section 2: Đồng bộ đám mây WebDAV */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Database className="w-4 h-4 text-emerald-400" />
                                Đồng bộ đám mây (WebDAV Cloud Sync)
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Kích hoạt đồng bộ WebDAV</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Tự động sao lưu và đồng bộ tài khoản giữa nhiều thiết bị</div>
                                    </div>
                                    <ToggleSwitch
                                        checked={!!settings.webdav_sync_enabled}
                                        onChange={(val) => saveField('webdav_sync_enabled', val)}
                                    />
                                </div>

                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Địa chỉ WebDAV URL</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Ví dụ: Jianguoyun, Nextcloud, OwnCloud...</div>
                                    </div>
                                    <input
                                        type="text"
                                        placeholder="https://dav.jianguoyun.com/dav/"
                                        defaultValue={settings.webdav_sync_url || 'https://dav.jianguoyun.com/dav/'}
                                        onBlur={(e) => saveField('webdav_sync_url', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500 font-mono"
                                    />
                                </div>

                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Tài khoản WebDAV</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Email hoặc Username dịch vụ WebDAV</div>
                                    </div>
                                    <input
                                        type="text"
                                        placeholder="username hoặc email"
                                        defaultValue={settings.webdav_sync_username || ''}
                                        onBlur={(e) => saveField('webdav_sync_username', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500"
                                    />
                                </div>

                                <div className="p-4 flex flex-col md:flex-row md:items-center justify-between gap-3 hover:bg-neutral-800/30 transition-colors">
                                    <div className="flex-1">
                                        <div className="text-sm font-medium text-neutral-200">Mật khẩu ứng dụng WebDAV</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Mật khẩu ứng dụng chuyên dụng tạo từ trang WebDAV</div>
                                    </div>
                                    <input
                                        type="password"
                                        placeholder="••••••••••••"
                                        defaultValue={settings.webdav_sync_password || ''}
                                        onBlur={(e) => saveField('webdav_sync_password', e.target.value)}
                                        className="bg-neutral-950/80 border border-neutral-800 text-xs px-3 py-1.5 rounded-lg text-neutral-300 w-full md:w-80 focus:outline-none focus:border-blue-500 font-mono"
                                    />
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Đồng bộ ngay bây giờ</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Đẩy dữ liệu sao lưu mới nhất lên máy chủ WebDAV</div>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={() => showMsg('Đã khởi chạy tiến trình đồng bộ WebDAV!', true)}
                                        className="px-3.5 py-1.5 rounded-lg bg-emerald-600/20 hover:bg-emerald-600/30 text-emerald-300 text-xs font-medium border border-emerald-500/30 transition-colors flex items-center gap-1.5"
                                    >
                                        <RefreshCw className="w-3.5 h-3.5" />
                                        Đồng bộ ngay
                                    </button>
                                </div>
                            </div>
                        </section>

                        {/* Section 3: Nhập dữ liệu & Dọn dẹp */}
                        <section className="space-y-3">
                            <h2 className="text-sm font-semibold text-neutral-300 uppercase tracking-wider flex items-center gap-2">
                                <Download className="w-4 h-4 text-cyan-400" />
                                Nhập dữ liệu & Quản lý Cache
                            </h2>
                            <div className="bg-neutral-900/60 border border-neutral-800 rounded-xl overflow-hidden divide-y divide-neutral-800/60 shadow-sm">
                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Nhập dữ liệu từ Cockpit Tools gốc</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">
                                            Tự động phát hiện và chuyển đổi toàn bộ tài khoản từ Cockpit Tools (~/.cockpit_tools)
                                        </div>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={handleImportFromCockpit}
                                        className="px-3.5 py-1.5 rounded-lg bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 text-xs font-medium border border-cyan-500/30 transition-colors flex items-center gap-1.5"
                                    >
                                        <Download className="w-3.5 h-3.5" />
                                        Nhập từ Cockpit
                                    </button>
                                </div>

                                <div className="p-4 flex items-center justify-between gap-4 hover:bg-neutral-800/30 transition-colors">
                                    <div>
                                        <div className="text-sm font-medium text-neutral-200">Dọn dẹp bộ nhớ đệm (Clear Cache)</div>
                                        <div className="text-xs text-neutral-400 mt-0.5">Xóa cache quota, lịch sử thông báo và tài nguyên tạm thời</div>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={() => showMsg('Đã dọn dẹp bộ nhớ đệm thành công!', true)}
                                        className="px-3.5 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs font-medium border border-neutral-700 transition-colors flex items-center gap-1.5"
                                    >
                                        <Trash2 className="w-3.5 h-3.5 text-neutral-400" />
                                        Xóa Cache
                                    </button>
                                </div>
                            </div>
                        </section>
                    </div>
                )}

                {/* TAB 5: ABOUT */}
                {activeTab === 'about' && (
                    <div className="space-y-6">
                        <section className="bg-neutral-900/60 border border-neutral-800 rounded-xl p-6 space-y-4">
                            <div className="flex items-center gap-3">
                                <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-blue-600 to-indigo-500 flex items-center justify-center shadow-lg shadow-blue-500/20">
                                    <Layers className="w-5 h-5 text-white" />
                                </div>
                                <div>
                                    <h2 className="text-base font-bold text-neutral-100">Universal API & AI Cockpit Hub</h2>
                                    <div className="text-xs text-neutral-400">Phiên bản: 0.1.0-alpha (Tương thích giao thức Cockpit Tools)</div>
                                </div>
                            </div>
                            <p className="text-xs text-neutral-400 leading-relaxed">
                                Nền tảng quản lý tài khoản, đồng bộ phiên làm việc, tự động hóa và điều phối hạn mức AI thông minh cho các IDE lập trình hàng đầu: Antigravity IDE, Visual Studio Code, Cursor, Claude Desktop, Zed, Devin/Windsurf, Trae, CodeBuddy và Kiro.
                            </p>
                            <div className="pt-4 border-t border-neutral-800/80 flex flex-wrap items-center justify-between text-xs text-neutral-500 gap-3">
                                <div>Bản quyền &copy; 2026. Phát triển mã nguồn mở an toàn & bảo mật cục bộ.</div>
                                <div className="flex items-center gap-3 text-neutral-400">
                                    <span className="hover:text-blue-400 cursor-pointer">Tài liệu</span>
                                    <span>•</span>
                                    <span className="hover:text-blue-400 cursor-pointer">Bảo mật</span>
                                    <span>•</span>
                                    <span className="hover:text-blue-400 cursor-pointer">Kho mã nguồn</span>
                                </div>
                            </div>
                        </section>
                    </div>
                )}
            </div>
        </div>
    );
}
