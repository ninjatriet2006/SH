import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { getGatewayStatus, refreshGatewayModels, startGateway, stopGateway } from '../../../bridge/gateway_bridge';
import { getExternalConfig, getExternalStatus, getProviders, getProviderStatus } from '../../../bridge/external_bridge';
import { createAccessKey, listAccessKeys, revokeAccessKey } from '../../../bridge/access_keys_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccessKey, ExternalStatus, GatewayInfo } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';
import { Activity, Copy, Check, RefreshCw, Key, Power, ExternalLink } from 'lucide-react';

export function ApiRelayPage() {
    const { t } = useTranslation();
    const [gateway, setGateway] = useState<GatewayInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [extEnabled, setExtEnabled] = useState(false);
    const [extStatus, setExtStatus] = useState<ExternalStatus | null>(null);
    const [copied, setCopied] = useState(false);
    const [modelsMsg, setModelsMsg] = useState<string | null>(null);
    const [keys, setKeys] = useState<AccessKey[]>([]);
    const [newLabel, setNewLabel] = useState('');
    const [freshKey, setFreshKey] = useState<string | null>(null);

    const refreshKeys = async () => {
        try { setKeys(await listAccessKeys()); }
        catch (e) { setError(ipcErrorMessage(e)); }
    };

    const createKey = async () => {
        if (!newLabel.trim()) { setError(t('admin.keys_label_required') || 'Nhãn key không được trống'); return; }
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
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setLoading(false); }
        void refreshExternal();
    };

    const refreshExternal = async () => {
        try {
            const list = await getProviders();
            const enabled = list.filter((p) => p.config.enabled);
            if (enabled.length > 0) {
                const first = enabled[0];
                setExtEnabled(true);
                const s = await getProviderStatus(first.name);
                setExtStatus({
                    reachable: s.reachable,
                    http_status: s.http_status,
                    latency_ms: s.latency_ms,
                    error: s.error,
                });
            } else {
                const cfg = await getExternalConfig();
                setExtEnabled(cfg.enabled);
                setExtStatus(cfg.enabled ? await getExternalStatus() : null);
            }
        } catch {
            setExtStatus(null);
        }
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

    const status = gateway?.status ?? 'stopped';
    const extBadge = !extEnabled
        ? <span className="badge badge-warning">{t('dashboard.bridge_disabled')}</span>
        : extStatus?.reachable
            ? <span className="badge badge-success">{t('dashboard.bridge_enabled')}</span>
            : <span className="badge badge-danger">{t('dashboard.bridge_down')}</span>;

    return (
        <div style={{ maxWidth: 1000, margin: '0 auto' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1.5rem' }}>
                <div>
                    <h1 style={{ fontSize: '1.6rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                        <Activity color="var(--primary)" size={26} /> OpenAI Relay Gateway
                    </h1>
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '0.2rem' }}>
                        Cổng dịch ngược endpoint OpenAI兼容 chuẩn (tách biệt độc lập khỏi luồng đăng nhập IDE)
                    </p>
                </div>
                <button className="btn" onClick={() => { void refresh(); void refreshKeys(); }} disabled={busy}>
                    <RefreshCw size={14} className={busy ? 'spin' : ''} /> Làm mới
                </button>
            </div>

            {error && <div className="error-message" style={{ marginBottom: '1rem' }}>{error}</div>}

            <div className="card">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                    <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>Trạng thái Gateway</h3>
                    <span className={`badge ${status === 'running' ? 'badge-success' : status === 'error' ? 'badge-danger' : 'badge-warning'}`}>
                        {loading ? 'Đang tải...' : status.toUpperCase()}
                    </span>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1.5rem', marginBottom: '1.5rem' }}>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>Địa chỉ phục vụ</div>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '0.3rem' }}>
                            <code style={{ background: 'rgba(0,0,0,0.3)', padding: '0.2rem 0.5rem', borderRadius: 4 }}>{endpointUrl}</code>
                            {endpointUrl !== '--' && (
                                <button className="btn" style={{ padding: '0.2rem 0.5rem' }} onClick={copyEndpoint}>
                                    {copied ? <Check size={12} color="var(--success)" /> : <Copy size={12} />}
                                </button>
                            )}
                        </div>
                    </div>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>Tài khoản phục vụ</div>
                        <div style={{ fontWeight: 600, marginTop: '0.3rem' }}>{gateway?.healthy_accounts ?? 0} tài khoản</div>
                    </div>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>External Upstream</div>
                        <div style={{ marginTop: '0.3rem' }}>{extBadge}</div>
                    </div>
                </div>

                <div style={{ display: 'flex', gap: '0.75rem', borderTop: '1px solid var(--border)', paddingTop: '1rem' }}>
                    <button
                        className={`btn ${status === 'running' ? 'btn-danger' : 'btn-primary'}`}
                        onClick={toggleGateway}
                        disabled={busy || loading}
                    >
                        <Power size={14} /> {status === 'running' ? 'Dừng Gateway' : 'Bật Gateway'}
                    </button>
                    <button className="btn" onClick={refreshModels} disabled={busy || status !== 'running'}>
                        <RefreshCw size={14} /> Cập nhật Models Upstream
                    </button>
                    <Link to="/providers" className="btn" style={{ textDecoration: 'none', marginLeft: 'auto' }}>
                        <ExternalLink size={14} /> Cấu hình Upstream Providers
                    </Link>
                </div>
                {modelsMsg && <div className="status-message">{modelsMsg}</div>}
            </div>

            {/* Access Keys Management */}
            <div className="card" style={{ marginTop: '1.5rem' }}>
                <h3 style={{ fontSize: '1.1rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Key size={18} color="var(--primary)" /> Quản lý Access Keys Gateway
                </h3>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', marginTop: '0.25rem' }}>
                    Các khóa API dùng để cấp quyền cho bên thứ 3 (Curl, Chatbox, NextChat, Open-WebUI) gọi vào endpoint relay.
                </p>

                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem', marginBottom: '1.5rem' }}>
                    <input
                        type="text"
                        className="input"
                        placeholder="Nhãn khóa mới (vd: Open-WebUI Desktop)"
                        value={newLabel}
                        onChange={(e) => setNewLabel(e.target.value)}
                        style={{ flex: 1 }}
                    />
                    <button className="btn btn-primary" onClick={createKey} disabled={busy}>
                        Tạo Key Mới
                    </button>
                </div>

                {freshKey && (
                    <div style={{ background: 'rgba(34, 197, 94, 0.1)', border: '1px solid var(--success)', padding: '1rem', borderRadius: 8, marginBottom: '1rem' }}>
                        <div style={{ color: 'var(--success)', fontWeight: 600, fontSize: '0.85rem' }}>Khóa vừa tạo thành công (chỉ hiển thị 1 lần):</div>
                        <code style={{ display: 'block', marginTop: '0.5rem', wordBreak: 'break-all', color: '#fff' }}>{freshKey}</code>
                    </div>
                )}

                <table>
                    <thead>
                        <tr>
                            <th>ID</th>
                            <th>Nhãn</th>
                            <th>Tiền tố</th>
                            <th>Ngày tạo</th>
                            <th style={{ textAlign: 'right' }}>Thao tác</th>
                        </tr>
                    </thead>
                    <tbody>
                        {keys.length === 0 ? (
                            <tr>
                                <td colSpan={5} style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '2rem' }}>
                                    Chưa có khóa nào được tạo.
                                </td>
                            </tr>
                        ) : (
                            keys.map((k) => (
                                <tr key={k.id}>
                                    <td>#{k.id}</td>
                                    <td><strong>{k.label}</strong></td>
                                    <td><code>{k.prefix}...</code></td>
                                    <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>{new Date(k.created_at * 1000).toLocaleString()}</td>
                                    <td style={{ textAlign: 'right' }}>
                                        <button className="btn btn-danger" style={{ padding: '0.25rem 0.5rem', fontSize: '0.75rem' }} onClick={() => revokeKey(k.id)}>
                                            Thu hồi
                                        </button>
                                    </td>
                                </tr>
                            ))
                        )}
                    </tbody>
                </table>
            </div>
        </div>
    );
}
