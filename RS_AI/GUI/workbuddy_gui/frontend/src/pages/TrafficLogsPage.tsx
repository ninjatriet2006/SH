import { useEffect, useState } from 'react';
import { Copy, Trash2, RefreshCw, Radio } from 'lucide-react';
import { getTrafficLogs, clearTrafficLogs } from '../../../bridge/audit_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { TrafficAuditLog } from '../../../bridge/types';

export function TrafficLogsPage() {
    const [logs, setLogs] = useState<TrafficAuditLog[]>([]);
    const [selectedLog, setSelectedLog] = useState<TrafficAuditLog | null>(null);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [searchTerm, setSearchTerm] = useState('');

    const refresh = async () => {
        try {
            setError(null);
            const data = await getTrafficLogs();
            setLogs(data);
            if (data.length > 0 && !selectedLog) {
                setSelectedLog(data[0]);
            } else if (selectedLog) {
                const stillExists = data.find((l) => l.id === selectedLog.id);
                if (stillExists) setSelectedLog(stillExists);
                else if (data.length > 0) setSelectedLog(data[0]);
                else setSelectedLog(null);
            }
        } catch (e) {
            setError(ipcErrorMessage(e));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void refresh();
        const interval = setInterval(() => {
            void refresh();
        }, 3000);
        return () => clearInterval(interval);
    }, []);

    const handleClear = async () => {
        try {
            setBusy(true);
            await clearTrafficLogs();
            setLogs([]);
            setSelectedLog(null);
        } catch (e) {
            setError(ipcErrorMessage(e));
        } finally {
            setBusy(false);
        }
    };

    const copyText = (text: string) => {
        navigator.clipboard.writeText(text);
    };

    const filteredLogs = logs.filter((l) => {
        if (!searchTerm.trim()) return true;
        const q = searchTerm.toLowerCase();
        return (
            l.route.toLowerCase().includes(q) ||
            l.model.toLowerCase().includes(q) ||
            l.account_uid.toLowerCase().includes(q) ||
            l.status_code.toString().includes(q)
        );
    });

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: 'calc(100vh - 4rem)', gap: '1rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                    <h1 style={{ margin: 0 }}>Traffic Audit Log</h1>
                    <span style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                        Real-time inspection of inbound raw wire headers vs forwarded upstream headers
                    </span>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <button className="btn" onClick={() => void refresh()} disabled={busy}>
                        <RefreshCw size={14} style={{ marginRight: '4px' }} /> Refresh
                    </button>
                    <button className="btn btn-danger" onClick={() => void handleClear()} disabled={busy || logs.length === 0}>
                        <Trash2 size={14} style={{ marginRight: '4px' }} /> Clear Buffer
                    </button>
                </div>
            </div>

            {error && <p className="error-message" style={{ margin: 0 }}>{error}</p>}

            <div style={{ display: 'flex', flex: 1, gap: '1rem', minHeight: 0 }}>
                {/* Left side: Request list */}
                <div className="card" style={{ width: '380px', display: 'flex', flexDirection: 'column', padding: '0.75rem', minHeight: 0 }}>
                    <div style={{ marginBottom: '0.5rem' }}>
                        <input
                            type="text"
                            placeholder="Filter requests..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                            style={{
                                width: '100%',
                                padding: '0.4rem 0.6rem',
                                borderRadius: '4px',
                                border: '1px solid var(--border-color)',
                                background: 'var(--bg-primary)',
                                color: 'var(--text-primary)',
                                fontSize: '0.85rem',
                            }}
                        />
                    </div>
                    <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
                        {loading && <div style={{ color: 'var(--text-secondary)', textAlign: 'center', padding: '1rem' }}>Loading...</div>}
                        {!loading && filteredLogs.length === 0 && (
                            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-secondary)', gap: '0.5rem' }}>
                                <Radio size={28} />
                                <span style={{ fontSize: '0.85rem' }}>No traffic recorded yet</span>
                            </div>
                        )}
                        {filteredLogs.map((log) => {
                            const isSelected = selectedLog?.id === log.id;
                            const isErr = log.status_code >= 400;
                            return (
                                <div
                                    key={log.id}
                                    onClick={() => setSelectedLog(log)}
                                    style={{
                                        padding: '0.5rem 0.6rem',
                                        borderRadius: '4px',
                                        cursor: 'pointer',
                                        background: isSelected ? 'var(--bg-card-hover, rgba(59, 130, 246, 0.15))' : 'var(--bg-primary)',
                                        border: isSelected ? '1px solid var(--primary)' : '1px solid var(--border-color)',
                                        fontSize: '0.8rem',
                                        display: 'flex',
                                        flexDirection: 'column',
                                        gap: '0.2rem',
                                    }}
                                >
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                        <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                                            <span
                                                style={{
                                                    fontSize: '0.7rem',
                                                    fontWeight: 'bold',
                                                    padding: '0.1rem 0.3rem',
                                                    borderRadius: '3px',
                                                    background: isErr ? '#fee2e2' : '#dcfce7',
                                                    color: isErr ? '#991b1b' : '#166534',
                                                }}
                                            >
                                                {log.status_code}
                                            </span>
                                            <span style={{ fontWeight: 600 }}>{log.model || log.route}</span>
                                        </div>
                                        <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>
                                            {log.duration_ms}ms
                                        </span>
                                    </div>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)', fontSize: '0.75rem' }}>
                                        <span>Account: {log.account_uid || 'none'}</span>
                                        <span>{log.proxy_used ? 'VPN/Proxy' : 'Direct'}</span>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>

                {/* Right side: Split Panels (Header Audit) */}
                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '1rem', minHeight: 0 }}>
                    {selectedLog ? (
                        <>
                            {/* Panel 1: RAW REQUEST HEADERS */}
                            <div className="card" style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '0.75rem', minHeight: 0 }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.4rem' }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                        <strong style={{ fontSize: '0.85rem', color: 'var(--text-primary)' }}>
                                            RAW REQUEST HEADERS (FROM CLIENT)
                                        </strong>
                                        <span className="badge badge-info" style={{ fontSize: '0.75rem' }}>
                                            {selectedLog.raw_request_headers.length} headers
                                        </span>
                                    </div>
                                    <button
                                        className="btn"
                                        style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem' }}
                                        onClick={() => {
                                            const text = selectedLog.raw_request_headers.map(([k, v]) => `${k}: ${v}`).join('\n');
                                            copyText(text);
                                        }}
                                    >
                                        <Copy size={12} style={{ marginRight: '4px' }} /> Copy
                                    </button>
                                </div>
                                <div style={{ flex: 1, overflowY: 'auto', fontFamily: 'monospace', fontSize: '0.8rem', background: 'var(--bg-primary)', padding: '0.5rem', borderRadius: '4px' }}>
                                    {selectedLog.raw_request_headers.length === 0 ? (
                                        <span style={{ color: 'var(--text-secondary)' }}>No client headers</span>
                                    ) : (
                                        selectedLog.raw_request_headers.map(([k, v], i) => (
                                            <div key={i} style={{ marginBottom: '0.2rem', wordBreak: 'break-all' }}>
                                                <span style={{ color: '#0ea5e9', fontWeight: 600 }}>{k}:</span>{' '}
                                                <span style={{ color: 'var(--text-primary)' }}>{v}</span>
                                            </div>
                                        ))
                                    )}
                                </div>
                            </div>

                            {/* Panel 2: RAW FORWARDED HEADERS */}
                            <div className="card" style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '0.75rem', minHeight: 0 }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.4rem' }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                        <strong style={{ fontSize: '0.85rem', color: '#6366f1' }}>
                                            RAW FORWARDED HEADERS (SENT TO UPSTREAM)
                                        </strong>
                                        <span className="badge badge-success" style={{ fontSize: '0.75rem' }}>
                                            {selectedLog.raw_forwarded_headers.length} headers
                                        </span>
                                        {selectedLog.proxy_used && (
                                            <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                                                via {selectedLog.proxy_used}
                                            </span>
                                        )}
                                    </div>
                                    <button
                                        className="btn"
                                        style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem' }}
                                        onClick={() => {
                                            const text = selectedLog.raw_forwarded_headers.map(([k, v]) => `${k}: ${v}`).join('\n');
                                            copyText(text);
                                        }}
                                    >
                                        <Copy size={12} style={{ marginRight: '4px' }} /> Copy
                                    </button>
                                </div>
                                <div style={{ flex: 1, overflowY: 'auto', fontFamily: 'monospace', fontSize: '0.8rem', background: 'var(--bg-primary)', padding: '0.5rem', borderRadius: '4px' }}>
                                    {selectedLog.raw_forwarded_headers.length === 0 ? (
                                        <span style={{ color: 'var(--text-secondary)' }}>No upstream forwarded headers</span>
                                    ) : (
                                        selectedLog.raw_forwarded_headers.map(([k, v], i) => (
                                            <div key={i} style={{ marginBottom: '0.2rem', wordBreak: 'break-all' }}>
                                                <span style={{ color: '#6366f1', fontWeight: 600 }}>{k}:</span>{' '}
                                                <span style={{ color: 'var(--text-primary)' }}>{v}</span>
                                            </div>
                                        ))
                                    )}
                                </div>
                            </div>
                        </>
                    ) : (
                        <div className="card" style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)' }}>
                            Select a request from the left list to inspect wire headers
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
