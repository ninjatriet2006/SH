import { useEffect, useState } from 'react';
import { Copy, Trash2, RefreshCw, Radio } from 'lucide-react';
import { getTrafficLogs, clearTrafficLogs } from '../../../bridge/audit_bridge';
import { getUsageSummary } from '../../../bridge/usage_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { TrafficAuditLog, UsageSummary } from '../../../bridge/types';

function headersText(hs: [string, string][]): string {
    return (hs ?? []).map(([k, v]) => `${k}: ${v}`).join('\n');
}

function WirePanel(props: {
    title: string;
    accent: string;
    badge: string;
    badgeClass: string;
    extra?: React.ReactNode;
    onCopy: () => void;
    children: React.ReactNode;
}) {
    return (
        <div className="card" style={{ display: 'flex', flexDirection: 'column', padding: '0.75rem', marginBottom: 0, flexShrink: 0 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.4rem' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <strong style={{ fontSize: '0.85rem', color: props.accent }}>{props.title}</strong>
                    <span className={`badge ${props.badgeClass}`} style={{ fontSize: '0.75rem' }}>{props.badge}</span>
                    {props.extra}
                </div>
                <button className="btn" style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem' }} onClick={props.onCopy}>
                    <Copy size={12} style={{ marginRight: '4px' }} /> Copy
                </button>
            </div>
            <div style={{ maxHeight: '16rem', overflowY: 'auto', fontFamily: 'monospace', fontSize: '0.8rem', background: 'var(--bg-primary)', padding: '0.5rem', borderRadius: '4px', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>
                {props.children}
            </div>
        </div>
    );
}

function HeaderRows(props: { hs: [string, string][]; emptyText: string; keyColor: string }) {
    const hs = props.hs ?? [];
    if (hs.length === 0) return <span style={{ color: 'var(--text-secondary)' }}>{props.emptyText}</span>;
    return (
        <>
            {hs.map(([k, v], i) => (
                <div key={i} style={{ marginBottom: '0.2rem' }}>
                    <span style={{ color: props.keyColor, fontWeight: 600 }}>{k}:</span>{' '}
                    <span style={{ color: 'var(--text-primary)' }}>{v}</span>
                </div>
            ))}
        </>
    );
}

export function TrafficLogsPage() {
    const [logs, setLogs] = useState<TrafficAuditLog[]>([]);
    const [summary, setSummary] = useState<UsageSummary[]>([]);
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
            try { setSummary(await getUsageSummary()); } catch { /* usage optional */ }
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

    const copyText = async (text: string) => {
        try {
            await navigator.clipboard.writeText(text);
        } catch {
            // Fallback khi clipboard API bị chặn (Tauri webview): chọn tay.
            const ta = document.createElement('textarea');
            ta.value = text;
            document.body.appendChild(ta);
            ta.select();
            try { document.execCommand('copy'); } catch { /* ignore */ }
            document.body.removeChild(ta);
        }
    };

    const filteredLogs = logs.filter((l) => {
        if (!searchTerm.trim()) return true;
        const q = searchTerm.toLowerCase();
        return (
            l.route.toLowerCase().includes(q) ||
            l.model.toLowerCase().includes(q) ||
            l.account_uid.toLowerCase().includes(q) ||
            l.status_code.toString().includes(q) ||
            (l.raw_request_body || '').toLowerCase().includes(q) ||
            (l.raw_response_preview || '').toLowerCase().includes(q)
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

            {summary.length > 0 && (
                <details style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                    <summary style={{ cursor: 'pointer' }}>
                        Usage per key — {summary.reduce((a, s) => a + s.requests, 0)} requests,{' '}
                        {summary.reduce((a, s) => a + s.tokens, 0)} tokens,{' '}
                        {summary.reduce((a, s) => a + s.errors, 0)} errors
                    </summary>
                    <table style={{ marginTop: '0.5rem' }}>
                        <thead><tr><th>Key</th><th>Requests</th><th>Tokens</th><th>Errors</th></tr></thead>
                        <tbody>
                            {summary.map((s) => (
                                <tr key={s.access_key_prefix || '(none)'}>
                                    <td><code>{s.access_key_prefix || '(none)'}</code></td>
                                    <td>{s.requests}</td><td>{s.tokens}</td><td>{s.errors}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </details>
            )}

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
                                            <span
                                                className={`badge ${log.account_uid === 'external' ? 'badge-info' : 'badge-success'}`}
                                                style={{ fontSize: '0.65rem' }}
                                                title={log.account_uid === 'external' ? 'Routed via anti-api bridge' : 'CodeBuddy pool account'}
                                            >
                                                {log.account_uid === 'external' ? 'Anti-API' : 'CodeBuddy'}
                                            </span>
                                        </div>
                                        <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>
                                            {log.duration_ms}ms
                                        </span>
                                    </div>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)', fontSize: '0.75rem' }}>
                                        <span>Account: {log.account_uid || 'none'}</span>
                                        <span>{log.proxy_used ? 'Proxy' : 'Direct'}</span>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>

                {/* Right side: Raw wire panels (học vpn_ai_proxy_gui TrafficTab) */}
                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '1rem', minHeight: 0, overflowY: 'auto', paddingRight: '0.25rem' }}>
                    {selectedLog ? (
                        <>
                            <WirePanel
                                title="RAW REQUEST HEADERS (FROM CLIENT)"
                                accent="var(--text-primary)"
                                badge={`${(selectedLog.raw_request_headers ?? []).length} headers`}
                                badgeClass="badge-info"
                                onCopy={() => void copyText(headersText(selectedLog.raw_request_headers))}
                            >
                                <HeaderRows hs={selectedLog.raw_request_headers} emptyText="No client headers" keyColor="#0ea5e9" />
                            </WirePanel>

                            <WirePanel
                                title="RAW REQUEST BODY (FROM CLIENT)"
                                accent="var(--text-primary)"
                                badge={`${(selectedLog.raw_request_body || '').length} chars`}
                                badgeClass="badge-info"
                                onCopy={() => void copyText(selectedLog.raw_request_body || '')}
                            >
                                {selectedLog.raw_request_body
                                    ? <span style={{ color: 'var(--text-primary)' }}>{selectedLog.raw_request_body}</span>
                                    : <span style={{ color: 'var(--text-secondary)' }}>&lt;empty body&gt;</span>}
                            </WirePanel>

                            {(selectedLog.raw_forwarded_body || '') !== '' && (
                                <WirePanel
                                    title="FORWARDED BODY (SENT UPSTREAM, POST-INJECTION)"
                                    accent="#6366f1"
                                    badge={`${selectedLog.raw_forwarded_body.length} chars`}
                                    badgeClass="badge-success"
                                    onCopy={() => void copyText(selectedLog.raw_forwarded_body)}
                                >
                                    <span style={{ color: 'var(--text-primary)' }}>{selectedLog.raw_forwarded_body}</span>
                                </WirePanel>
                            )}

                            <WirePanel
                                title="RAW FORWARDED HEADERS (SENT TO UPSTREAM)"
                                accent="#6366f1"
                                badge={`${(selectedLog.raw_forwarded_headers ?? []).length} headers`}
                                badgeClass="badge-success"
                                extra={selectedLog.proxy_used && (
                                    <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>via {selectedLog.proxy_used}</span>
                                )}
                                onCopy={() => void copyText(headersText(selectedLog.raw_forwarded_headers))}
                            >
                                <HeaderRows hs={selectedLog.raw_forwarded_headers} emptyText="No upstream forwarded headers" keyColor="#6366f1" />
                            </WirePanel>

                            <WirePanel
                                title="RAW RESPONSE HEADERS (FROM UPSTREAM)"
                                accent="#f59e0b"
                                badge={`${(selectedLog.raw_response_headers ?? []).length} headers`}
                                badgeClass="badge-warning"
                                onCopy={() => void copyText(headersText(selectedLog.raw_response_headers))}
                            >
                                <HeaderRows hs={selectedLog.raw_response_headers} emptyText="No response headers captured" keyColor="#f59e0b" />
                            </WirePanel>

                            <WirePanel
                                title="RAW RESPONSE BODY (FROM UPSTREAM)"
                                accent="#f59e0b"
                                badge={`${(selectedLog.raw_response_preview || '').length} chars${selectedLog.is_streaming ? ' · stream' : ''}`}
                                badgeClass="badge-warning"
                                onCopy={() => void copyText(selectedLog.raw_response_preview || '')}
                            >
                                {selectedLog.raw_response_preview
                                    ? <span style={{ color: 'var(--text-primary)' }}>{selectedLog.raw_response_preview}</span>
                                    : <span style={{ color: 'var(--text-secondary)' }}>&lt;empty body&gt;</span>}
                            </WirePanel>
                        </>
                    ) : (
                        <div className="card" style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)' }}>
                            Select a request from the left list to inspect raw wire traffic
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
