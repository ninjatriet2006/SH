import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getGatewayStatus, refreshGatewayModels, startGateway, stopGateway } from '../../../bridge/gateway_bridge';
import { getExternalConfig, getExternalStatus } from '../../../bridge/external_bridge';
import { createAccessKey, listAccessKeys, revokeAccessKey } from '../../../bridge/access_keys_bridge';
import { runTask, type SchedulerTask } from '../../../bridge/scheduler_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccessKey, ExternalStatus, GatewayInfo, TaskResult } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

export function DashboardPage() {
    const { t } = useTranslation();
    const [gateway, setGateway] = useState<GatewayInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [taskResult, setTaskResult] = useState<TaskResult | null>(null);
    const [extEnabled, setExtEnabled] = useState(false);
    const [extBase, setExtBase] = useState('');
    const [extStatus, setExtStatus] = useState<ExternalStatus | null>(null);
    const [copied, setCopied] = useState(false);
    const [modelsMsg, setModelsMsg] = useState<string | null>(null);
    // Access keys (chuyển từ Admin — таб keys duy nhất không trùng).
    const [keys, setKeys] = useState<AccessKey[]>([]);
    const [newLabel, setNewLabel] = useState('');
    const [freshKey, setFreshKey] = useState<string | null>(null);

    const refreshKeys = async () => {
        try { setKeys(await listAccessKeys()); }
        catch (e) { setError(ipcErrorMessage(e)); }
    };

    const createKey = async () => {
        if (!newLabel.trim()) { setError(t('admin.keys_label_required')); return; }
        try {
            setBusy(true); setError(null);
            const created = await createAccessKey(newLabel.trim());
            setFreshKey(created.plaintext);
            setNewLabel('');
            setKeys(await listAccessKeys());
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const revokeKey = async (id: number) => {
        try {
            setBusy(true); setError(null);
            await revokeAccessKey(id);
            setKeys(await listAccessKeys());
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    /// `listen` (":7863", "0.0.0.0:7863", ...) → URL local bấm là copy.
    const endpointUrl = (() => {
        const listen = gateway?.listen?.trim() ?? '';
        if (!listen) return '--';
        const m = listen.match(/:(\d+)$/);
        return `http://127.0.0.1:${m ? m[1] : listen}`;
    })();

    const copyEndpoint = async () => {
        try {
            await navigator.clipboard.writeText(endpointUrl);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
        } catch {
            setCopied(false);
        }
    };

    const refresh = async () => {
        try {
            setError(null);
            setGateway(await getGatewayStatus());
            try {
                const cfg = await getExternalConfig();
                setExtEnabled(cfg.enabled);
                setExtBase(cfg.base_url);
                setExtStatus(cfg.enabled ? await getExternalStatus() : null);
            } catch {
                setExtStatus(null);
            }
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setLoading(false); }
    };

    useEffect(() => { void refresh(); void refreshKeys(); }, []);

    const toggleGateway = async () => {
        try {
            setBusy(true); setError(null);
            if (gateway?.status === 'running') await stopGateway(); else await startGateway();
            await refresh();
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const refreshModels = async () => {
        try {
            setBusy(true); setError(null); setModelsMsg(null);
            const models = await refreshGatewayModels();
            setModelsMsg(`${models.length} models: ${models.map((m) => m.id).join(', ')}`);
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const executeTask = async (task: SchedulerTask) => {
        try { setBusy(true); setError(null); setTaskResult(await runTask(task)); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const status = gateway?.status ?? 'stopped';
    const extBadge = !extEnabled
        ? <span className="badge badge-warning">{t('dashboard.bridge_disabled')}</span>
        : extStatus?.reachable
            ? <span className="badge badge-success">{t('dashboard.bridge_enabled')}</span>
            : <span className="badge badge-danger">{t('dashboard.bridge_down')}</span>;
    return (
        <div>
            <h1>{t('dashboard.title')}</h1>
            <div className="card">
                <h3>
                    {t('dashboard.gateway_status')}{' '}
                    <span className="badge badge-success">CodeBuddy</span>
                </h3>
                <div style={{ display: 'flex', gap: '2rem', marginTop: '1rem' }}>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{t('dashboard.status')}</div>
                        <div><span className={`badge ${status === 'running' ? 'badge-success' : status === 'error' ? 'badge-danger' : 'badge-warning'}`}>{loading ? t('common.loading') : status}</span></div>
                    </div>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{t('dashboard.accounts')}</div>
                        <div>{gateway?.healthy_accounts ?? 0} / {gateway?.total_accounts ?? 0}</div>
                    </div>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{t('dashboard.listen')}</div>
                        <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                            <code style={{ fontSize: '0.85rem' }}>{loading ? t('common.loading') : endpointUrl}</code>
                            <button className="btn" onClick={() => void copyEndpoint()} disabled={loading || !gateway?.listen} title={endpointUrl}>
                                {copied ? '✓' : t('common.copy')}
                            </button>
                        </div>
                    </div>
                </div>
                <div style={{ marginTop: '1rem' }}>
                    <button className="btn btn-primary" onClick={() => void toggleGateway()} disabled={busy || loading}>{status === 'running' ? t('dashboard.stop') : t('dashboard.start')}</button>
                    <button className="btn" onClick={() => void refresh()} disabled={busy}>{t('common.refresh')}</button>
                    <button className="btn" onClick={() => void refreshModels()} disabled={busy || loading || status !== 'running'} title={t('dashboard.models_hint')}>{t('dashboard.models')}</button>
                </div>
                {modelsMsg && <p className="success-message" style={{ wordBreak: 'break-word' }}>{modelsMsg}</p>}
                {gateway?.error && <p className="error-message">{gateway.error}</p>}
                {error && <p className="error-message">{error}</p>}
            </div>
            <div className="card">
                <h3>
                    {t('dashboard.bridge_title')}{' '}
                    <span className="badge badge-info">Anti-API</span>
                </h3>
                <div style={{ display: 'flex', gap: '2rem', marginTop: '1rem', alignItems: 'center', flexWrap: 'wrap' }}>
                    <div>{extBadge}</div>
                    {extEnabled && (
                        <div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>Base URL</div>
                            <div style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>{extBase || '--'}</div>
                        </div>
                    )}
                    {extEnabled && extStatus?.reachable && (
                        <div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>Health</div>
                            <div>
                                {extStatus.http_status ? `HTTP ${extStatus.http_status}` : 'OK'}
                                {extStatus.latency_ms != null ? ` · ${extStatus.latency_ms}ms` : ''}
                            </div>
                        </div>
                    )}
                    {extEnabled && extStatus && !extStatus.reachable && extStatus.error && (
                        <div style={{ color: 'var(--danger)', fontSize: '0.85rem' }}>{extStatus.error}</div>
                    )}
                    <div>
                        <Link className="btn" style={{ textDecoration: 'none', color: 'var(--text-primary)' }} to="/providers">{t('dashboard.bridge_open')}</Link>
                    </div>
                </div>
            </div>
            <div className="card">
                <h3>{t('admin.tab_keys')}</h3>
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                    <input
                        className="input"
                        style={{ flex: 1 }}
                        placeholder={t('admin.keys_label_placeholder')}
                        value={newLabel}
                        onChange={(e) => setNewLabel(e.target.value)}
                        disabled={busy}
                    />
                    <button className="btn btn-primary" onClick={() => void createKey()} disabled={busy}>{t('admin.keys_create')}</button>
                </div>
                {freshKey && (
                    <p className="success-message" style={{ wordBreak: 'break-all' }}>
                        {t('admin.keys_created')}: <code>{freshKey}</code>
                    </p>
                )}
                {keys.length > 0 && (
                    <table style={{ marginTop: '1rem' }}>
                        <thead><tr>
                            <th>{t('admin.keys_label')}</th><th>{t('admin.keys_prefix')}</th>
                            <th>{t('admin.keys_last_used')}</th><th>{t('admin.keys_status')}</th><th></th>
                        </tr></thead>
                        <tbody>
                            {keys.map((k) => (
                                <tr key={k.id}>
                                    <td>{k.label}</td><td><code>{k.prefix}</code></td>
                                    <td>{k.last_used_at ? new Date(k.last_used_at * 1000).toLocaleString() : '--'}</td>
                                    <td>{k.revoked
                                        ? <span className="badge badge-danger">{t('admin.revoked')}</span>
                                        : <span className="badge badge-success">{t('admin.active')}</span>}
                                    </td>
                                    <td>
                                        {!k.revoked && (
                                            <button className="btn btn-danger" disabled={busy} onClick={() => void revokeKey(k.id)}>
                                                {t('admin.revoke')}
                                            </button>
                                        )}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                )}
            </div>
            <div className="card">
                <h3>{t('dashboard.quick_actions')}</h3>
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                    <button className="btn btn-primary" onClick={() => void executeTask('checkin')} disabled={busy}>{t('dashboard.checkin_now')}</button>
                    <button className="btn btn-primary" onClick={() => void executeTask('travel')} disabled={busy}>{t('dashboard.travel_now')}</button>
                    <button className="btn btn-primary" onClick={() => void executeTask('activity')} disabled={busy}>{t('dashboard.activity_now')}</button>
                    <button className="btn btn-primary" onClick={() => void executeTask('keepalive')} disabled={busy}>{t('dashboard.keepalive_now')}</button>
                </div>
                {taskResult && <p className={taskResult.success ? 'success-message' : 'error-message'}>{taskResult.message}</p>}
            </div>
        </div>
    );
}
