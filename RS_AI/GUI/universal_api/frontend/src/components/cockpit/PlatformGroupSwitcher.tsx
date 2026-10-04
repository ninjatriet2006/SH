import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { ChevronDown, Pencil } from 'lucide-react';
import { ALL_PLATFORMS, type PlatformVariant } from './types';

interface PlatformGroupSwitcherProps {
    platformMenuOpen: boolean;
    setPlatformMenuOpen: (open: boolean) => void;
    platformIcon: string;
    platformLabel: string;
    currentActiveTriggerLabel: string;
    groupVariants: PlatformVariant[];
}

export function PlatformGroupSwitcher({
    platformMenuOpen,
    setPlatformMenuOpen,
    platformIcon,
    platformLabel,
    currentActiveTriggerLabel,
    groupVariants,
}: PlatformGroupSwitcherProps) {
    const navigate = useNavigate();
    const switcherRef = useRef<HTMLDivElement | null>(null);

    // Close switcher dropdown on outside click or Esc
    useEffect(() => {
        if (!platformMenuOpen) return;
        const handleMouseDown = (e: MouseEvent) => {
            if (switcherRef.current && !switcherRef.current.contains(e.target as Node)) {
                setPlatformMenuOpen(false);
            }
        };
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setPlatformMenuOpen(false);
        };
        document.addEventListener('mousedown', handleMouseDown);
        document.addEventListener('keydown', handleKeyDown);
        return () => {
            document.removeEventListener('mousedown', handleMouseDown);
            document.removeEventListener('keydown', handleKeyDown);
        };
    }, [platformMenuOpen, setPlatformMenuOpen]);

    return (
        <div className="platform-group-switcher" ref={switcherRef}>
            <button
                type="button"
                className={`platform-group-switcher-trigger ${platformMenuOpen ? 'is-open' : ''}`}
                onClick={() => setPlatformMenuOpen(!platformMenuOpen)}
                aria-label="Chuyển đổi phân loại / nền tảng cùng nhóm"
            >
                <span className="platform-group-switcher-trigger-icon">
                    <img src={platformIcon} alt="" style={{ width: 16, height: 16, objectFit: 'contain' }} />
                </span>
                <span className="platform-group-switcher-trigger-label">
                    {currentActiveTriggerLabel}
                </span>
                <ChevronDown size={14} className="platform-group-switcher-trigger-caret" />
            </button>

            {platformMenuOpen && (
                <div className="platform-group-switcher-dropdown">
                    {/* Group Variants Section */}
                    <div style={{ fontSize: '0.7rem', fontWeight: 700, color: 'var(--text-muted)', padding: '4px 10px 2px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                        Phân loại ứng dụng ({platformLabel})
                    </div>
                    {groupVariants.map((v) => (
                        <button
                            key={v.id}
                            type="button"
                            className={`platform-group-switcher-option ${v.isActive ? 'is-active' : ''}`}
                            onClick={() => {
                                v.onSelect();
                                setPlatformMenuOpen(false);
                            }}
                        >
                            <span className="platform-group-switcher-option-icon">
                                <img src={v.icon} alt="" style={{ width: 16, height: 16, objectFit: 'contain' }} />
                            </span>
                            <span className="platform-group-switcher-option-text">
                                <span className="platform-group-switcher-option-label">{v.label}</span>
                                <span className="platform-group-switcher-option-subtext">{v.subtext}</span>
                            </span>
                            {v.isActive && <span className="platform-group-switcher-check">✓</span>}
                        </button>
                    ))}

                    <div className="platform-group-switcher-divider" />

                    {/* All Platforms Navigation Section */}
                    <div style={{ fontSize: '0.7rem', fontWeight: 700, color: 'var(--text-muted)', padding: '4px 10px 2px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                        Tất cả nền tảng Cockpit
                    </div>
                    <div style={{ maxHeight: 180, overflowY: 'auto' }}>
                        {ALL_PLATFORMS.map((p) => (
                            <button
                                key={p.id}
                                type="button"
                                className="platform-group-switcher-option"
                                onClick={() => {
                                    setPlatformMenuOpen(false);
                                    navigate(p.path);
                                }}
                            >
                                <span className="platform-group-switcher-option-icon">
                                    <img src={p.icon} alt="" style={{ width: 15, height: 15, objectFit: 'contain' }} />
                                </span>
                                <span className="platform-group-switcher-option-label" style={{ fontSize: '0.8rem' }}>
                                    {p.label}
                                </span>
                                <span />
                            </button>
                        ))}
                    </div>

                    <div className="platform-group-switcher-divider" />

                    {/* Group Management action */}
                    <button
                        type="button"
                        className="platform-group-switcher-action"
                        onClick={() => {
                            setPlatformMenuOpen(false);
                            navigate('/settings');
                        }}
                    >
                        <span className="platform-group-switcher-action-icon">
                            <Pencil size={14} />
                        </span>
                        <span className="platform-group-switcher-action-label">
                            Quản lý nhóm nền tảng (Group Settings)
                        </span>
                    </button>
                </div>
            )}
        </div>
    );
}
