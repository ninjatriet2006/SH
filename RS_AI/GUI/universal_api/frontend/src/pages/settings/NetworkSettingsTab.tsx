import React from 'react';
import type { GuiSettings } from '../../../../bridge/types';
import { Toggle } from './SettingsShared';

interface NetworkSettingsTabProps {
    settings: GuiSettings;
    saving: boolean;
    saveField: <K extends keyof GuiSettings>(key: K, value: GuiSettings[K]) => Promise<void>;
}

export const NetworkSettingsTab: React.FC<NetworkSettingsTabProps> = ({
    settings,
    saving,
    saveField,
}) => {
    return (
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
    );
};
