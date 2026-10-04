import {
    Play,
    Square,
    Folder,
    Copy,
    Edit3,
    Trash2,
} from 'lucide-react';
import type { InstanceProfile, AccountInfo } from './types';

interface InstanceCardProps {
    inst: InstanceProfile;
    accounts: AccountInfo[];
    onLaunch: (inst: InstanceProfile) => void;
    onStop: (inst: InstanceProfile) => void;
    onOpenFolder: (inst: InstanceProfile) => void;
    onClone: (inst: InstanceProfile) => void;
    onEdit: (inst: InstanceProfile) => void;
    onDelete: (inst: InstanceProfile) => void;
    maskText: (text: string) => string;
}

export function InstanceCard({
    inst,
    accounts,
    onLaunch,
    onStop,
    onOpenFolder,
    onClone,
    onEdit,
    onDelete,
    maskText,
}: InstanceCardProps) {
    const isRunning = Boolean(inst.is_running || inst.isRunning);
    const isDef = Boolean(inst.is_default || inst.isDefault || inst.id === 'default');
    const pid = inst.last_pid || inst.lastPid;
    const dir = inst.user_data_dir || inst.userDataDir;

    // Find bound account
    const boundId = inst.bound_account_id || inst.bindAccountId;
    const boundAcc = boundId ? accounts.find((a) => a.uid === boundId || a.nickname === boundId) : undefined;

    return (
        <div
            style={{
                display: 'grid',
                gridTemplateColumns: 'minmax(220px, 1.8fr) minmax(200px, 2fr) minmax(120px, 1fr) 180px',
                padding: '1rem 1.25rem',
                borderBottom: '1px solid rgba(51, 65, 85, 0.6)',
                alignItems: 'center',
                backgroundColor: isRunning ? 'rgba(16, 185, 129, 0.03)' : 'transparent',
                transition: 'background-color 0.2s',
            }}
        >
            {/* 1. Instance Name + Badge + Dir */}
            <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                    <span style={{ fontWeight: 600, color: '#f8fafc', fontSize: '0.9rem' }}>
                        {inst.name}
                    </span>
                    {/* Status Badge */}
                    {isRunning ? (
                        <span
                            style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '0.3rem',
                                backgroundColor: 'rgba(16, 185, 129, 0.15)',
                                color: '#34d399',
                                border: '1px solid rgba(16, 185, 129, 0.3)',
                                padding: '0.15rem 0.45rem',
                                borderRadius: '9999px',
                                fontSize: '0.7rem',
                                fontWeight: 700,
                            }}
                        >
                            <span
                                style={{
                                    width: '6px',
                                    height: '6px',
                                    borderRadius: '50%',
                                    backgroundColor: '#10b981',
                                    boxShadow: '0 0 6px #10b981',
                                }}
                            />
                            RUNNING
                        </span>
                    ) : (
                        <span
                            style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '0.3rem',
                                backgroundColor: 'rgba(100, 116, 139, 0.15)',
                                color: '#94a3b8',
                                border: '1px solid rgba(100, 116, 139, 0.3)',
                                padding: '0.15rem 0.45rem',
                                borderRadius: '9999px',
                                fontSize: '0.7rem',
                                fontWeight: 600,
                            }}
                        >
                            <span
                                style={{
                                    width: '6px',
                                    height: '6px',
                                    borderRadius: '50%',
                                    backgroundColor: '#64748b',
                                }}
                            />
                            STOPPED
                        </span>
                    )}
                </div>
                <div style={{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.25rem' }}>
                    {isDef ? 'Default profile' : maskText(dir || 'Custom profile')}
                </div>
            </div>

            {/* 2. Account */}
            <div>
                {boundAcc ? (
                    <div>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                            <span style={{ fontSize: '0.825rem', color: '#e2e8f0', fontWeight: 500 }}>
                                {maskText(boundAcc.nickname || boundAcc.uid)}
                            </span>
                            <span
                                style={{
                                    backgroundColor: 'rgba(56, 189, 248, 0.15)',
                                    color: '#38bdf8',
                                    border: '1px solid rgba(56, 189, 248, 0.3)',
                                    borderRadius: '0.25rem',
                                    padding: '0.1rem 0.35rem',
                                    fontSize: '0.65rem',
                                    fontWeight: 700,
                                }}
                            >
                                {boundAcc.plan_tier || 'PRO'}
                            </span>
                        </div>
                        <div style={{ fontSize: '0.72rem', color: '#94a3b8', marginTop: '0.2rem' }}>
                            • Quota: Available
                        </div>
                    </div>
                ) : (
                    <span style={{ fontSize: '0.8rem', color: '#64748b' }}>
                        -- Chưa gán tài khoản --
                    </span>
                )}
            </div>

            {/* 3. Process / PID */}
            <div>
                {isRunning && pid ? (
                    <span
                        style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '0.3rem',
                            backgroundColor: 'rgba(6, 182, 212, 0.12)',
                            color: '#22d3ee',
                            border: '1px solid rgba(6, 182, 212, 0.3)',
                            padding: '0.2rem 0.5rem',
                            borderRadius: '0.375rem',
                            fontSize: '0.75rem',
                            fontWeight: 600,
                            fontFamily: 'monospace',
                        }}
                    >
                        PID: {pid}
                    </span>
                ) : (
                    <span style={{ color: '#64748b', fontSize: '0.85rem' }}>-</span>
                )}
            </div>

            {/* 4. Actions */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '0.4rem' }}>
                {/* Open Folder */}
                <button
                    title="Mở thư mục"
                    onClick={() => onOpenFolder(inst)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        borderRadius: '0.375rem',
                        color: '#cbd5e1',
                        padding: '0.4rem',
                        cursor: 'pointer',
                    }}
                >
                    <Folder size={14} />
                </button>

                {/* Play / Stop */}
                {isRunning ? (
                    <button
                        title="Dừng instance"
                        onClick={() => onStop(inst)}
                        style={{
                            backgroundColor: 'rgba(239, 68, 68, 0.15)',
                            border: '1px solid rgba(239, 68, 68, 0.4)',
                            borderRadius: '0.375rem',
                            color: '#f87171',
                            padding: '0.4rem',
                            cursor: 'pointer',
                        }}
                    >
                        <Square size={14} fill="#f87171" />
                    </button>
                ) : (
                    <button
                        title="Khởi chạy instance"
                        onClick={() => onLaunch(inst)}
                        style={{
                            backgroundColor: 'rgba(16, 185, 129, 0.15)',
                            border: '1px solid rgba(16, 185, 129, 0.4)',
                            borderRadius: '0.375rem',
                            color: '#34d399',
                            padding: '0.4rem',
                            cursor: 'pointer',
                        }}
                    >
                        <Play size={14} fill="#34d399" />
                    </button>
                )}

                {/* Clone */}
                <button
                    title="Nhân bản"
                    onClick={() => onClone(inst)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        borderRadius: '0.375rem',
                        color: '#cbd5e1',
                        padding: '0.4rem',
                        cursor: 'pointer',
                    }}
                >
                    <Copy size={14} />
                </button>

                {/* Edit */}
                <button
                    title="Chỉnh sửa cấu hình"
                    onClick={() => onEdit(inst)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        borderRadius: '0.375rem',
                        color: '#cbd5e1',
                        padding: '0.4rem',
                        cursor: 'pointer',
                    }}
                >
                    <Edit3 size={14} />
                </button>

                {/* Delete */}
                <button
                    title={isDef ? 'Không thể xóa Default Instance' : 'Xóa instance'}
                    disabled={isDef}
                    onClick={() => onDelete(inst)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        borderRadius: '0.375rem',
                        color: isDef ? '#475569' : '#f87171',
                        padding: '0.4rem',
                        cursor: isDef ? 'not-allowed' : 'pointer',
                        opacity: isDef ? 0.35 : 1,
                    }}
                >
                    <Trash2 size={14} />
                </button>
            </div>
        </div>
    );
}
