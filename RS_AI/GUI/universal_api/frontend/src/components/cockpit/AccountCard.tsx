import {
    AlertCircle,
    Tag,
    FileText,
    Play,
    RefreshCw,
    Upload,
    Trash2,
} from 'lucide-react';
import type { AccountInfo, CockpitPlatformId } from './types';

interface AccountCardProps {
    account: AccountInfo;
    platformId: CockpitPlatformId;
    isSelected: boolean;
    refreshingUid: string | null;
    onToggleSelect: (uid: string) => void;
    onSwitchAccount: (account: AccountInfo) => void;
    onRefreshSingle: (account: AccountInfo) => void;
    onDelete: (uid: string) => void;
    maskValue: (val: string) => string;
}

export function AccountCard({
    account,
    platformId,
    isSelected,
    refreshingUid,
    onToggleSelect,
    onSwitchAccount,
    onRefreshSingle,
    onDelete,
    maskValue,
}: AccountCardProps) {
    const isCurrent = Boolean(account.is_current);

    return (
        <div className={`account-card ${isCurrent ? 'current' : ''}`}>
            {/* Top Row */}
            <div className="card-top" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <input
                    type="checkbox"
                    checked={isSelected}
                    onChange={() => onToggleSelect(account.uid)}
                />
                <div className="card-email-label" style={{ flex: 1, fontWeight: 600, fontSize: '0.88rem' }}>
                    {maskValue(account.nickname || account.uid)}
                </div>
                {platformId === 'github_copilot' ? (
                    <div style={{ display: 'flex', gap: '0.35rem' }}>
                        <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                            Current
                        </span>
                        <span className="badge" style={{ background: '#0284c7', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                            PRO
                        </span>
                    </div>
                ) : platformId === 'antigravity' ? (
                    (() => {
                        const rawTier = (account.plan_tier || account.quota_details?.plan_tier || 'FREE').toUpperCase();
                        const isProTier = rawTier.includes('PRO') || rawTier.includes('ULTRA');
                        return (
                            <div style={{ display: 'flex', gap: '0.35rem' }}>
                                {isCurrent && (
                                    <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                        Current
                                    </span>
                                )}
                                <span
                                    className="badge"
                                    style={{
                                        background: isProTier ? '#0284c7' : 'rgba(255, 255, 255, 0.1)',
                                        color: '#fff',
                                        fontSize: '0.65rem',
                                        fontWeight: 600,
                                        padding: '0.15rem 0.45rem',
                                        borderRadius: 4
                                    }}
                                >
                                    {rawTier}
                                </span>
                            </div>
                        );
                    })()
                ) : (
                    <div style={{ display: 'flex', gap: '0.35rem' }}>
                        {isCurrent && (
                            <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                Current
                            </span>
                        )}
                        <span className="badge" style={{ background: 'rgba(255, 255, 255, 0.1)', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                            {account.plan_tier || 'FREE'}
                        </span>
                    </div>
                )}
            </div>

            {/* Antigravity Specific Quota Grid vs Generic Status */}
            {platformId === 'antigravity' ? (
                (() => {
                    const q = account.quota_details as any;
                    const isPro = (account.plan_tier || q?.plan_tier || '').toUpperCase().includes('PRO');
                    const models = q?.models || [];
                    const claudeModel = models.find((m: any) => m.name?.toLowerCase().includes('claude'));
                    const geminiModel = models.find((m: any) => m.name?.toLowerCase().includes('gemini'));

                    const parseBucket = (model: any, bucketType: string) => {
                        const b = model?.buckets?.find((x: any) => x.bucket_type === bucketType);
                        if (!b) return null;
                        const pct = Math.round(b.remaining_percentage || 0);
                        return {
                            percent: pct,
                            time_left: b.time_left || 'Ready',
                        };
                    };

                    const claude5h = claudeModel ? parseBucket(claudeModel, 'rolling_5h') : null;
                    const claudeWeekly = claudeModel ? parseBucket(claudeModel, 'weekly') : null;
                    const gemini5h = geminiModel ? parseBucket(geminiModel, 'rolling_5h') : null;
                    const geminiWeekly = geminiModel ? parseBucket(geminiModel, 'weekly') : null;

                    return (
                        <div style={{ padding: '0.6rem 0.75rem', background: 'rgba(15, 23, 42, 0.4)', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.04)', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
                                {/* Claude Column */}
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                                    <div style={{ fontSize: '0.8rem', fontWeight: 700, color: '#e2e8f0' }}>Claude</div>

                                    {claude5h && (
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>5h</span>
                                                <span style={{ color: claude5h.percent >= 50 ? '#22c55e' : '#f97316', fontWeight: 600 }}>{claude5h.percent}%</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: `${claude5h.percent}%`, background: claude5h.percent >= 50 ? '#22c55e' : '#f97316' }} />
                                            </div>
                                            <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                {claude5h.time_left}
                                            </div>
                                        </div>
                                    )}

                                    {claudeWeekly && (
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>Weekly</span>
                                                <span style={{ color: claudeWeekly.percent >= 50 ? '#22c55e' : '#f97316', fontWeight: 600 }}>{claudeWeekly.percent}%</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: `${claudeWeekly.percent}%`, background: claudeWeekly.percent >= 50 ? '#22c55e' : '#f97316' }} />
                                            </div>
                                            <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                {claudeWeekly.time_left}
                                            </div>
                                        </div>
                                    )}

                                    {!claude5h && !claudeWeekly && (
                                        <div style={{ fontSize: '0.72rem', color: '#64748b', fontStyle: 'italic', marginTop: 2 }}>
                                            Chưa có hạn ngạch
                                        </div>
                                    )}
                                </div>

                                {/* Gemini Column */}
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                                    <div style={{ fontSize: '0.8rem', fontWeight: 700, color: '#e2e8f0' }}>Gemini</div>

                                    {isPro && gemini5h && (
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>5h</span>
                                                <span style={{ color: gemini5h.percent >= 50 ? '#22c55e' : '#ef4444', fontWeight: 600 }}>{gemini5h.percent}%</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: `${gemini5h.percent}%`, background: gemini5h.percent >= 50 ? '#22c55e' : '#ef4444' }} />
                                            </div>
                                            <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                {gemini5h.time_left}
                                            </div>
                                        </div>
                                    )}

                                    {geminiWeekly && (
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>Weekly</span>
                                                <span style={{ color: geminiWeekly.percent >= 50 ? '#22c55e' : '#ef4444', fontWeight: 600 }}>{geminiWeekly.percent}%</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: `${geminiWeekly.percent}%`, background: geminiWeekly.percent >= 50 ? '#22c55e' : '#ef4444' }} />
                                            </div>
                                            <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                {geminiWeekly.time_left}
                                            </div>
                                        </div>
                                    )}

                                    {!gemini5h && !geminiWeekly && (
                                        <div style={{ fontSize: '0.72rem', color: '#64748b', fontStyle: 'italic', marginTop: 2 }}>
                                            Chưa có hạn ngạch
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)', marginTop: '0.4rem' }}>
                                Available AI Credits: {account.credits ?? 0}
                            </div>
                        </div>
                    );
                })()
            ) : (
                <>
                    {/* Usage Status */}
                    <div className="card-status-row">
                        <span>Usage Status</span>
                        <span className="card-status-val">Normal</span>
                    </div>

                    {/* Quota Query Box */}
                    <div className="card-quota-box">
                        <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)', fontWeight: 600 }}>
                            Quota Query
                        </div>
                        <div className="card-quota-title-row">
                            <span className="card-quota-title">Free Plan Subscription</span>
                            <span className="card-quota-val">0 / 100</span>
                        </div>

                        {/* Progress Bar */}
                        <div className="quota-track">
                            <div className="quota-fill" style={{ width: '10%' }} />
                        </div>

                        <div className="card-quota-meta">
                            Next refresh time: 11/01/2026, 00:00:00
                        </div>

                        <div className="card-quota-title-row" style={{ marginTop: 2 }}>
                            <span className="card-quota-title" style={{ fontSize: '0.75rem' }}>Credit Package</span>
                            <span className="card-quota-val" style={{ fontSize: '0.75rem' }}>0 / 0</span>
                        </div>
                    </div>
                </>
            )}

            {/* Card Footer with Actions */}
            <div className="card-footer">
                <span className="card-date">
                    10/01/2026 15:02
                </span>

                <div className="card-actions">
                    <button
                        className="card-action-btn"
                        title="Thông tin chi tiết"
                    >
                        <AlertCircle size={12} />
                    </button>
                    <button
                        className="card-action-btn"
                        title="Sửa nhãn tag"
                    >
                        <Tag size={12} />
                    </button>
                    <button
                        className="card-action-btn"
                        title="Ghi chú"
                    >
                        <FileText size={12} />
                    </button>
                    <button
                        className="card-action-btn play"
                        onClick={() => onSwitchAccount(account)}
                        title="Khởi chạy / Chuyển tài khoản vào IDE"
                    >
                        <Play size={12} />
                    </button>
                    <button
                        className="card-action-btn"
                        onClick={() => onRefreshSingle(account)}
                        title="Làm mới Quota"
                    >
                        <RefreshCw size={12} className={refreshingUid === account.uid ? 'spin' : ''} />
                    </button>
                    <button
                        className="card-action-btn"
                        title="Xuất JSON / Upload"
                    >
                        <Upload size={12} />
                    </button>
                    <button
                        className="card-action-btn trash"
                        onClick={() => onDelete(account.uid)}
                        title="Xóa tài khoản"
                    >
                        <Trash2 size={12} />
                    </button>
                </div>
            </div>
        </div>
    );
}
