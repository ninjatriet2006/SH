import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { addAccount, disableAccount, enableAccount, listAccounts, removeAccount } from '../../../bridge/accounts_bridge';
import { createAccessKey, listAccessKeys, revokeAccessKey } from '../../../bridge/access_keys_bridge';
import { getDebugTraces, getUsageSummary, listUsageLogs } from '../../../bridge/usage_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccessKey, AccountInfo, TraceRecord, UsageLog, UsageSummary } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

type AdminTab = 'credentials' | 'keys' | 'usage' | 'status' | 'debug';

export function AdminPage() {
    const { t } = useTranslation();
    const [tab, setTab] = useState<AdminTab>('credentials');
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [keys, setKeys] = useState<AccessKey[]>([]);
    const [summary, setSummary] = useState<UsageSummary[]>([]);
    const [logs, setLogs] = useState<UsageLog[]>([]);
    const [traces, setTraces] = useState<TraceRecord[]>([]);
    const [newLabel, setNewLabel] = useState('');
    const [freshKey, setFreshKey] = useState<string | null>(null);

    const run = async (fn: () => Promise<void>) => {
        try { setBusy(true); setError(null); await fn(); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const refreshAll = async () => {
        await run(async () => {
            const [a, k, s, l, tr] = await Promise.all([
                listAccounts(), listAccessKeys(), getUsageSummary(), listUsageLogs(100), getDebugTraces(),
            ]);
            setAccounts(a); setKeys(k); setSummary(s); setLogs(l); setTraces(tr);
        });
    };

    useEffect(() => { void refreshAll(); }, []);

    const chooseAuthFile = async () => {
        const path = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (typeof path !== 'string') return;
        await run(async () => { await addAccount(path); setAccounts(await listAccounts()); });
    };

    const createKey = async () => {
        if (!newLabel.trim()) { setError(t('admin.keys_label_required')); return; }
        await run(async () => {
            const created = await createAccessKey(newLabel.trim());
            setFreshKey(created.plaintext);
            setNewLabel('');
            setKeys(await listAccessKeys());
        });
    };

    const tabs: { id: AdminTab; label: string }[] = [
        { id: 'credentials', label: t('admin.tab_credentials') },
        { id: 'keys', label: t('admin.tab_keys') },
        { id: 'usage', label: t('admin.tab_usage') },
        { id: 'status', label: t('admin.tab_status') },
        { id: 'debug', label: t('admin.tab_debug') },
    ];

    return (
        <div>
            <h1>{t('admin.title')}</h1>
            <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem', flexWrap: 'wrap' }}>
                {tabs.map(tb => (
                    <button
                        key={tb.id}
                        className={tab === tb.id ? 'btn btn-primary' : 'btn'}
                        onClick={() => setTab(tb.id)}
                    >
                        {tb.label}
                    </button>
                ))}
                <span style={{ flex: 1 }} />
                <button className="btn" onClick={() => void refreshAll()} disabled={busy}>{t('common.refresh')}</button>
            </div>
            {error && <p className="error-message">{error}</p>}

            {tab === 'credentials' && (
                <div className="card">
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '1rem' }}>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('accounts.count')}: {accounts.length}</span>
                        <button className="btn btn-primary" onClick={() => void chooseAuthFile()} disabled={busy}>{t('accounts.add')}</button>
                    </div>
                    <table>
                        <thead><tr>
                            <th>{t('accounts.col_uid')}</th><th>{t('accounts.col_nickname')}</th>
                            <th>{t('accounts.col_credits')}</th><th>{t('accounts.col_status')}</th>
                            <th>{t('accounts.col_actions')}</th>
                        </tr></thead>
                        <tbody>
                            {accounts.map(a => (
                                <tr key={a.uid}>
                                    <td>{a.uid}</td><td>{a.nickname}</td><td>{a.credits}</td>
                                    <td>{a.disabled
                                        ? <span className="badge badge-danger">{t('admin.disabled')}</span>
                                        : a.healthy
                                            ? <span className="badge badge-success">Live</span>
                                            : <span className="badge badge-warning">Die</span>}
                                    </td>
                                    <td>
                                        <button className="btn" disabled={busy} onClick={() => void run(async () => { if (a.disabled) { await enableAccount(a.uid); } else { await disableAccount(a.uid); } setAccounts(await listAccounts()); })}>
                                            {a.disabled ? t('admin.enable') : t('admin.disable')}
                                        </button>{' '}
                                        <button className="btn btn-danger" disabled={busy} onClick={() => void run(async () => { await removeAccount(a.uid); setAccounts(await listAccounts()); })}>{t('common.delete')}</button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {tab === 'keys' && (
                <div className="card">
                    <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
                        <input
                            placeholder={t('admin.keys_label_placeholder')}
                            value={newLabel}
                            onChange={e => setNewLabel(e.target.value)}
                            style={{ flex: 1, padding: '0.5rem', borderRadius: '6px', border: '1px solid var(--border)', background: 'transparent', color: 'var(--text-primary)' }}
                        />
                        <button className="btn btn-primary" onClick={() => void createKey()} disabled={busy}>{t('admin.keys_create')}</button>
                    </div>
                    {freshKey && (
                        <p className="success-message" style={{ wordBreak: 'break-all' }}>
                            {t('admin.keys_created')}: <code>{freshKey}</code>
                        </p>
                    )}
                    <table>
                        <thead><tr>
                            <th>ID</th><th>{t('admin.keys_label')}</th><th>{t('admin.keys_prefix')}</th>
                            <th>{t('admin.keys_last_used')}</th><th>{t('admin.keys_status')}</th><th>{t('accounts.col_actions')}</th>
                        </tr></thead>
                        <tbody>
                            {keys.map(k => (
                                <tr key={k.id}>
                                    <td>{k.id}</td><td>{k.label}</td><td><code>{k.prefix}</code></td>
                                    <td>{k.last_used_at ? new Date(k.last_used_at * 1000).toLocaleString() : '--'}</td>
                                    <td>{k.revoked
                                        ? <span className="badge badge-danger">{t('admin.revoked')}</span>
                                        : <span className="badge badge-success">{t('admin.active')}</span>}
                                    </td>
                                    <td>
                                        {!k.revoked && (
                                            <button className="btn btn-danger" disabled={busy} onClick={() => void run(async () => { await revokeAccessKey(k.id); setKeys(await listAccessKeys()); })}>
                                                {t('admin.revoke')}
                                            </button>
                                        )}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {tab === 'usage' && (
                <div>
                    <div className="card">
                        <h3>{t('admin.usage_summary')}</h3>
                        <table>
                            <thead><tr>
                                <th>{t('admin.keys_prefix')}</th><th>{t('admin.usage_requests')}</th>
                                <th>{t('admin.usage_tokens')}</th><th>{t('admin.usage_errors')}</th>
                            </tr></thead>
                            <tbody>
                                {summary.map(s => (
                                    <tr key={s.access_key_prefix || '(none)'}>
                                        <td><code>{s.access_key_prefix || '(none)'}</code></td>
                                        <td>{s.requests}</td><td>{s.tokens}</td><td>{s.errors}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                    <div className="card">
                        <h3>{t('admin.usage_recent')}</h3>
                        <table>
                            <thead><tr>
                                <th>ID</th><th>{t('admin.keys_prefix')}</th><th>Route</th>
                                <th>Model</th><th>Status</th><th>Tokens</th><th>ms</th>
                            </tr></thead>
                            <tbody>
                                {logs.map(l => (
                                    <tr key={l.id}>
                                        <td>{l.id}</td><td><code>{l.access_key_prefix}</code></td>
                                        <td>{l.route}</td><td>{l.model}</td>
                                        <td>{l.status >= 400
                                            ? <span className="badge badge-danger">{l.status}</span>
                                            : <span className="badge badge-success">{l.status}</span>}
                                        </td>
                                        <td>{l.tokens}</td><td>{l.elapsed_ms}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}

            {tab === 'status' && (
                <div className="card">
                    <table>
                        <thead><tr>
                            <th>{t('accounts.col_uid')}</th><th>{t('accounts.col_nickname')}</th>
                            <th>{t('admin.status_live')}</th><th>{t('accounts.col_credits')}</th>
                            <th>{t('accounts.col_inflight')}</th><th>{t('accounts.col_status')}</th>
                        </tr></thead>
                        <tbody>
                            {accounts.map(a => (
                                <tr key={a.uid}>
                                    <td>{a.uid}</td><td>{a.nickname}</td>
                                    <td>{a.healthy
                                        ? <span className="badge badge-success">Live</span>
                                        : <span className="badge badge-danger">Die</span>}
                                    </td>
                                    <td>{a.credits}</td><td>{a.in_flight}</td>
                                    <td>{a.cooling ? `${a.cool_kind ?? 'cool'} (${a.cool_remaining_sec ?? 0}s)` : a.disabled_reason ?? '--'}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {tab === 'debug' && (
                <div className="card">
                    <h3>{t('admin.debug_title')}</h3>
                    <table>
                        <thead><tr>
                            <th>Route</th><th>Model</th><th>Status</th><th>Tokens</th><th>ms</th><th>Time</th>
                        </tr></thead>
                        <tbody>
                            {traces.map((tr, i) => (
                                <tr key={i}>
                                    <td>{tr.route}</td><td>{tr.model}</td>
                                    <td>{tr.status >= 400
                                        ? <span className="badge badge-danger">{tr.status}</span>
                                        : <span className="badge badge-success">{tr.status}</span>}
                                    </td>
                                    <td>{tr.tokens}</td><td>{tr.elapsed_ms}</td>
                                    <td>{new Date(tr.timestamp * 1000).toLocaleString()}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                    <h3 style={{ marginTop: '1.5rem' }}>{t('admin.debug_detail')}</h3>
                    {traces.filter(tr => tr.detail).slice(0, 10).map((tr, i) => (
                        <details key={i} style={{ marginBottom: '0.5rem' }}>
                            <summary style={{ cursor: 'pointer' }}>
                                {tr.route} · {tr.model} · {tr.status} · {tr.detail?.req_bytes ?? 0} req bytes
                            </summary>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.5rem', marginTop: '0.5rem' }}>
                                <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontSize: '0.75rem', background: 'rgba(0,0,0,0.3)', padding: '0.5rem', borderRadius: '6px', maxHeight: '200px', overflow: 'auto' }}>
                                    {t('admin.debug_request')}: {tr.detail?.req_preview ?? '--'}
                                </pre>
                                <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontSize: '0.75rem', background: 'rgba(0,0,0,0.3)', padding: '0.5rem', borderRadius: '6px', maxHeight: '200px', overflow: 'auto' }}>
                                    {t('admin.debug_response')}: {tr.detail?.resp_preview ?? '--'}
                                    {tr.detail?.error ? `\n${t('admin.debug_error')}: ${tr.detail.error}` : ''}
                                </pre>
                            </div>
                        </details>
                    ))}
                </div>
            )}
        </div>
    );
}
