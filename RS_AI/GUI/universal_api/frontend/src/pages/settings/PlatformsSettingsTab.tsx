import React, { useState } from 'react';
import { Search } from 'lucide-react';
import type { GuiSettings } from '../../../../bridge/types';
import { Toggle, PathRow } from './SettingsShared';

interface PlatformsSettingsTabProps {
    settings: GuiSettings;
    saving: boolean;
    saveField: <K extends keyof GuiSettings>(key: K, value: GuiSettings[K]) => Promise<void>;
    handleBrowse: (fieldKey: keyof GuiSettings) => Promise<void>;
    handleResetDefault: (fieldKey: keyof GuiSettings, defaultVal?: string) => Promise<void>;
    handleAutoDetect: (target: string, fieldKey: keyof GuiSettings) => Promise<void>;
}

export const PlatformsSettingsTab: React.FC<PlatformsSettingsTabProps> = ({
    settings,
    saving,
    saveField,
    handleBrowse,
    handleResetDefault,
    handleAutoDetect,
}) => {
    const [platformSearch, setPlatformSearch] = useState('');

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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
                            />

                            <PathRow
                                label="Antigravity Desktop Launch Target"
                                desc="Đường dẫn tệp thực thi Antigravity Desktop (App Legacy)"
                                fieldKey="antigravity_desktop_app_path"
                                currentPath={settings.antigravity_desktop_app_path}
                                detectTarget="antigravity_desktop"
                                defaultPath="/home/bimatkeo/Applications/antigravity/antigravity"
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
                            />

                            <PathRow
                                label="CodeBuddy CN App Path"
                                desc="Đường dẫn tệp thực thi của CodeBuddy Nội Địa (Tencent Cloud)"
                                fieldKey="codebuddy_cn_app_path"
                                currentPath={settings.codebuddy_cn_app_path}
                                detectTarget="codebuddy_cn"
                                defaultPath=""
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
                                onBrowse={handleBrowse}
                                onResetDefault={handleResetDefault}
                                onAutoDetect={handleAutoDetect}
                                saving={saving}
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
    );
};
