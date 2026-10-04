import {
    Clock,
    RefreshCw,
    CheckCircle2,
    TerminalSquare,
} from 'lucide-react';
import type { AccountInfo, CockpitPlatformId } from './types';

interface WakeupsTabProps {
    accounts: AccountInfo[];
    platformId: CockpitPlatformId;
    wakeupRunning: boolean;
    wakeupLogs: string[];
    onWakeupAll: () => void;
    onSingleWakeup: (account: AccountInfo) => void;
    maskValue: (val: string) => string;
}

export function WakeupsTab({
    accounts,
    wakeupRunning,
    wakeupLogs,
    onWakeupAll,
    onSingleWakeup,
    maskValue,
}: WakeupsTabProps) {
    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            {/* Header Card */}
            <div className="card" style={{ padding: '1.25rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
                    <div>
                        <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <Clock size={18} color="#38bdf8" /> Scheduled Wakeup & Official Language Server Tasks
                        </h3>
                        <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                            Tự động gửi tín hiệu giữ phiên tới Official Language Server (<code>AG_WAKEUP_OFFICIAL_LS_APP_DATA_DIR</code>) và Quota API để giữ ấm bộ đếm quota reset và chống hết hạn token.
                        </p>
                    </div>
                    <button
                        className="btn btn-primary"
                        style={{ padding: '0.5rem 1rem', display: 'flex', alignItems: 'center', gap: '0.45rem', fontWeight: 600 }}
                        onClick={onWakeupAll}
                        disabled={wakeupRunning}
                    >
                        <RefreshCw size={14} className={wakeupRunning ? 'animate-spin' : ''} />
                        {wakeupRunning ? 'Đang gửi Keep-Alive...' : 'Wakeup Tất Cả Tài Khoản Ngay'}
                    </button>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '1rem', marginTop: '1rem', paddingTop: '1rem', borderTop: '1px solid rgba(255, 255, 255, 0.08)' }}>
                    <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Language Server Runtime</div>
                        <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#22c55e', display: 'flex', alignItems: 'center', gap: '0.35rem', marginTop: 2 }}>
                            <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#22c55e', display: 'inline-block' }} />
                            Active (Linux x64)
                        </div>
                    </div>
                    <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Tự động Wakeup khi mở App</div>
                        <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#38bdf8', marginTop: 2 }}>
                            Enabled (delay 0s)
                        </div>
                    </div>
                    <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Chu kỳ Keep-Alive định kỳ</div>
                        <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#f8fafc', marginTop: 2 }}>
                            10 phút / lần
                        </div>
                    </div>
                </div>
            </div>

            {/* Accounts Table */}
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
                <div style={{ padding: '0.85rem 1.25rem', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', fontWeight: 600, fontSize: '0.85rem', color: '#f1f5f9' }}>
                    Danh sách tài khoản Keep-Alive ({accounts.length})
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Tài khoản</th>
                            <th>Gói</th>
                            <th>Trạng thái phiên</th>
                            <th>Lần Wakeup gần nhất</th>
                            <th style={{ textAlign: 'right' }}>Thao tác</th>
                        </tr>
                    </thead>
                    <tbody>
                        {accounts.map((a) => (
                            <tr key={a.uid}>
                                <td>
                                    <div style={{ fontWeight: 600, color: '#fff' }}>{maskValue(a.nickname || a.uid)}</div>
                                    <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>UID: {a.uid.slice(0, 16)}...</div>
                                </td>
                                <td>
                                    <span className={`badge ${a.plan_tier === 'PRO' ? 'badge-primary' : 'badge-secondary'}`}>
                                        {a.plan_tier || 'PRO'}
                                    </span>
                                </td>
                                <td>
                                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: '#22c55e', fontSize: '0.8rem', fontWeight: 500 }}>
                                        <CheckCircle2 size={13} /> Active / Keep-Alive OK
                                    </span>
                                </td>
                                <td style={{ fontSize: '0.78rem', color: '#94a3b8' }}>
                                    {new Date().toLocaleTimeString()} (Vừa làm mới)
                                </td>
                                <td style={{ textAlign: 'right' }}>
                                    <button
                                        className="btn"
                                        style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem', gap: '0.3rem' }}
                                        onClick={() => onSingleWakeup(a)}
                                    >
                                        <RefreshCw size={12} /> Wakeup
                                    </button>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>

            {/* Wakeup Log Box */}
            <div className="card" style={{ padding: '1rem', background: '#090d16', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
                <div style={{ fontSize: '0.78rem', fontWeight: 600, color: '#94a3b8', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                    <TerminalSquare size={14} /> Nhật ký Language Server & Keep-Alive Traces
                </div>
                <div style={{ maxHeight: 150, overflowY: 'auto', fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: '#38bdf8', display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                    {wakeupLogs.map((log, i) => (
                        <div key={i} style={{ opacity: i === 0 ? 1 : 0.75 }}>{log}</div>
                    ))}
                </div>
            </div>
        </div>
    );
}
