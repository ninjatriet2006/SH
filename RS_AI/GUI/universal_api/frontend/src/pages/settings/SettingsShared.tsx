import { RotateCcw, Search } from 'lucide-react';
import type { GuiSettings } from '../../../../bridge/types';

export type SettingsTab = 'general' | 'platforms' | 'network' | 'data' | 'about';

export function formatBytes(bytes: number): string {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export const Toggle = ({
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

export const PathRow = ({
    label,
    desc,
    fieldKey,
    currentPath,
    placeholder = 'Default path',
    detectTarget,
    defaultPath = '',
    onBrowse,
    onResetDefault,
    onAutoDetect,
    saving,
}: {
    label: string;
    desc?: string;
    fieldKey: keyof GuiSettings;
    currentPath?: string;
    placeholder?: string;
    detectTarget?: string;
    defaultPath?: string;
    onBrowse: (fieldKey: keyof GuiSettings) => void;
    onResetDefault: (fieldKey: keyof GuiSettings, defaultVal?: string) => void;
    onAutoDetect: (target: string, fieldKey: keyof GuiSettings) => void;
    saving: boolean;
}) => (
    <div className="setting-row">
        <div className="setting-info">
            <div className="setting-label">{label}</div>
            {desc && <div className="setting-hint">{desc}</div>}
        </div>
        <div
            className="setting-control"
            style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.4rem',
                flexWrap: 'wrap',
                justifyContent: 'flex-end',
                minWidth: '320px',
                maxWidth: '65%',
            }}
        >
            <input
                type="text"
                readOnly
                placeholder={placeholder}
                value={currentPath || ''}
                className="input"
                style={{
                    flex: 1,
                    minWidth: '180px',
                    fontSize: '0.78rem',
                    fontFamily: 'var(--font-mono)',
                }}
            />
            <button
                type="button"
                className="btn"
                style={{ fontSize: '0.78rem', padding: '0.42rem 0.75rem', height: '32px' }}
                onClick={() => onBrowse(fieldKey)}
                disabled={saving}
            >
                Select
            </button>
            <button
                type="button"
                className="btn"
                style={{
                    fontSize: '0.78rem',
                    padding: '0.42rem 0.75rem',
                    height: '32px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.3rem',
                }}
                title="Đặt lại về mặc định"
                onClick={() => onResetDefault(fieldKey, defaultPath)}
                disabled={saving}
            >
                <RotateCcw size={12} />
                <span>Reset to default</span>
            </button>
            {detectTarget && (
                <button
                    type="button"
                    className="btn btn-primary"
                    style={{
                        fontSize: '0.78rem',
                        padding: '0.42rem 0.75rem',
                        height: '32px',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.3rem',
                    }}
                    title="Tự động quét tệp .desktop để tìm ứng dụng"
                    onClick={() => onAutoDetect(detectTarget, fieldKey)}
                    disabled={saving}
                >
                    <Search size={12} />
                    <span>Auto detect</span>
                </button>
            )}
        </div>
    </div>
);
