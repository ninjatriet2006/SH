import React from 'react';

export const AboutSettingsTab: React.FC = () => {
    return (
        <>
            <div className="settings-card-header">
                <h2 className="settings-card-title">Thông tin Ứng dụng & Giấy phép</h2>
                <p className="settings-card-desc">Phiên bản nền tảng, công nghệ điều phối và tình trạng hoạt động của Universal Cockpit.</p>
            </div>

            <div className="settings-group">
                <div className="setting-row">
                    <div className="setting-info">
                        <div className="setting-label">Phiên bản Universal Engine (Core Version)</div>
                        <div className="setting-hint">Kiến trúc đa luồng Rust kết hợp giao diện Tauri v2</div>
                    </div>
                    <div className="setting-control">
                        <span
                            className="badge"
                            style={{
                                background: 'rgba(59, 130, 246, 0.15)',
                                color: 'var(--primary)',
                                border: '1px solid rgba(59, 130, 246, 0.3)',
                                padding: '0.3rem 0.75rem',
                                borderRadius: '6px',
                                fontWeight: 600,
                            }}
                        >
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
    );
};
