import { useEffect, useState } from 'react';
import {
    getProviders,
    saveProviders,
    getProviderStatus,
    refreshProviderModels,
    checkPortAvailable,
    suggestFreePort,
} from '../../../bridge/external_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { ExternalConfig, ExternalModel, ProviderEntry, ProviderStatus } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

// Thiết kế theo mẫu CodeBuddy: mỗi provider = Account + Configuration.
// KHÔNG scheduler (tasks bảo trì chỉ có ở CodeBuddy), KHÔNG quick actions.

const DEFAULT_CONFIG: ExternalConfig = {
    enabled: false,
    base_url: 'http://127.0.0.1:8964',
    api_key: '',
    timeout_seconds: 10,
    chat_timeout_seconds: 600,
    control_port: 8964,
    public_port: 8966,
    auto_fallback: true,
};

function validPort(p: number): boolean {
    return Number.isInteger(p) && p >= 1 && p <= 65535;
}

const num = (v: string, fallback: number): number => {
    const n = Number(v);
    return Number.isFinite(n) ? Math.floor(n) : fallback;
};

interface Msg {
    scope: string | null; // null = global, string = provider name
    text: string;
    ok: boolean;
}

export function ProvidersPage() {
    const { t } = useTranslation();
    const [providers, setProviders] = useState<ProviderEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState<string | null>(null); // provider name đang chạy op
    const [msg, setMsg] = useState<Msg | null>(null);
    const [status, setStatus] = useState<Record<string, ProviderStatus>>({});
    const [models, setModels] = useState<Record<string, ExternalModel[]>>({});
    const [portMsg, setPortMsg] = useState<Record<string, string>>({});
    const [newName, setNewName] = useState('');

    const load = async () => {
        try {
            setMsg(null);
            setProviders(await getProviders());
        } catch (e) {
            setMsg({ scope: null, text: ipcErrorMessage(e), ok: false });
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load();
    }, []);

    const setMsgFor = (scope: string | null, text: string, ok: boolean) =>
        setMsg({ scope, text, ok });

    const patchProvider = (name: string, patch: Partial<ProviderEntry>): ProviderEntry[] => {
        const next = providers.map((p) =>
            p.name === name ? { ...p, ...patch, config: { ...p.config, ...(patch.config ?? {}) } } : p,
        );
        setProviders(next);
        return next;
    };

    const patchConfig = (name: string, patch: Partial<ExternalConfig>) => {
        const cur = providers.find((p) => p.name === name);
        if (cur) patchProvider(name, { config: { ...cur.config, ...patch } });
    };

    const persistAll = async (list: ProviderEntry[], scope: string | null) => {
        await saveProviders(list);
        setMsgFor(scope, t('common.save'), true);
    };

    const validateCard = (p: ProviderEntry) => {
        if (!validPort(p.config.control_port) || !validPort(p.config.public_port)) {
            throw new Error(t('providers.invalid_port'));
        }
        if (p.config.control_port === p.config.public_port) {
            throw new Error(t('providers.ports_must_differ'));
        }
    };

    const saveCard = async (name: string) => {
        const p = providers.find((x) => x.name === name);
        if (!p) return;
        try {
            setBusy(name);
            setMsgFor(name, '', true);
            validateCard(p);
            await persistAll(providers, name);
        } catch (e) {
            setMsgFor(name, ipcErrorMessage(e), false);
        } finally {
            setBusy(null);
        }
    };

    const toggleEnabled = async (name: string, enabled: boolean) => {
        const next = patchProvider(name, {});
        const withFlag = next.map((p) =>
            p.name === name ? { ...p, config: { ...p.config, enabled } } : p,
        );
        setProviders(withFlag);
        try {
            setBusy(name);
            await persistAll(withFlag, name);
        } catch (e) {
            setMsgFor(name, ipcErrorMessage(e), false);
            void load(); // rollback hiển thị
        } finally {
            setBusy(null);
        }
    };

    const removeProvider = async (name: string) => {
        if (!window.confirm(`${t('common.delete')}: ${name}?`)) return;
        const next = providers.filter((p) => p.name !== name);
        setProviders(next);
        try {
            setBusy(name);
            await persistAll(next, null);
        } catch (e) {
            setMsgFor(null, ipcErrorMessage(e), false);
            void load();
        } finally {
            setBusy(null);
        }
    };

    const addProvider = async () => {
        const name = newName.trim().toLowerCase().replace(/[^a-z0-9_-]/g, '-');
        if (!name) return;
        if (providers.some((p) => p.name === name)) {
            setMsgFor(null, `duplicate provider name: ${name}`, false);
            return;
        }
        try {
            setBusy('__new__');
            // Gợi ý port trống để provider mới không đè port provider cũ.
            let control = 8964;
            let pubp = 8966;
            try {
                control = (await suggestFreePort(8964)).port;
                pubp = (await suggestFreePort(control + 1)).port;
                if (pubp === control) pubp = (await suggestFreePort(control + 2)).port;
            } catch { /* giữ default, validate sẽ báo */ }
            const entry: ProviderEntry = {
                name,
                kind: 'http-bridge',
                model_prefixes: '',
                config: { ...DEFAULT_CONFIG, control_port: control, public_port: pubp },
            };
            const next = [...providers, entry];
            setProviders(next);
            setNewName('');
            await persistAll(next, name);
        } catch (e) {
            setMsgFor(null, ipcErrorMessage(e), false);
        } finally {
            setBusy(null);
        }
    };

    const checkHealth = async (name: string) => {
        try {
            setBusy(name);
            const s = await getProviderStatus(name);
            setStatus((prev) => ({ ...prev, [name]: s }));
            if (!s.reachable || !s.enabled) {
                setMsgFor(name, s.error ?? 'unreachable', false);
            } else {
                setMsgFor(name, '', true);
            }
        } catch (e) {
            setMsgFor(name, ipcErrorMessage(e), false);
        } finally {
            setBusy(null);
        }
    };

    const loadModels = async (name: string) => {
        try {
            setBusy(name);
            const list = await refreshProviderModels(name);
            setModels((prev) => ({ ...prev, [name]: list }));
            setMsgFor(name, '', true);
        } catch (e) {
            setMsgFor(name, ipcErrorMessage(e), false);
        } finally {
            setBusy(null);
        }
    };

    const checkPort = async (name: string, port: number) => {
        try {
            const r = await checkPortAvailable(port);
            setPortMsg((prev) => ({
                ...prev,
                [name]: r.conflicts_gateway
                    ? t('providers.port_conflicts_gateway')
                    : r.available
                        ? t('providers.port_free')
                        : t('providers.port_busy'),
            }));
        } catch (e) {
            setPortMsg((prev) => ({ ...prev, [name]: ipcErrorMessage(e) }));
        }
    };

    const autoPort = async (name: string, field: 'control_port' | 'public_port') => {
        const p = providers.find((x) => x.name === name);
        if (!p) return;
        try {
            const r = await suggestFreePort(p.config[field]);
            patchConfig(name, { [field]: r.port } as Partial<ExternalConfig>);
        } catch (e) {
            setMsgFor(name, ipcErrorMessage(e), false);
        }
    };

    const renderMsg = (scope: string | null) => {
        if (!msg || msg.scope !== scope || !msg.text) return null;
        return <p className={msg.ok ? 'success-message' : 'error-message'}>{msg.text}</p>;
    };

    if (loading) return <div>{t('common.loading')}</div>;

    return (
        <div>
            <h1>{t('providers.title')} <span className="badge badge-info">Anti-API</span></h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)' }}>{t('providers.description')}</p>
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                    <input
                        className="input"
                        style={{ flex: 1, maxWidth: 320 }}
                        value={newName}
                        placeholder={t('providers.new_name_placeholder')}
                        onChange={(e) => setNewName(e.target.value)}
                        disabled={busy !== null}
                    />
                    <button className="btn btn-primary" onClick={() => void addProvider()} disabled={busy !== null || !newName.trim()}>
                        {t('providers.add_provider')}
                    </button>
                </div>
                {renderMsg(null)}
                {providers.length === 0 && (
                    <p style={{ color: 'var(--text-secondary)', marginTop: '1rem' }}>{t('providers.empty')}</p>
                )}
            </div>

            {providers.map((p) => {
                const st = status[p.name];
                const list = models[p.name] ?? [];
                const isBusy = busy === p.name;
                const dashUrl = `${p.config.base_url.replace(/\/$/, '')}/quota`;
                return (
                    <div className="card" key={p.name} style={{ marginTop: '1rem' }}>
                        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', flexWrap: 'wrap' }}>
                            <code style={{ fontSize: '1rem', fontWeight: 700 }}>{p.name}</code>
                            <span className="badge badge-info">{p.kind}</span>
                            {st && (st.reachable
                                ? <span className="badge badge-success">OK{st.http_status ? ` ${st.http_status}` : ''}{st.latency_ms != null ? ` · ${st.latency_ms}ms` : ''}</span>
                                : <span className="badge badge-warning">{st.enabled ? 'down' : 'off'}</span>)}
                            <span style={{ flex: 1 }} />
                            <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', fontSize: '0.85rem' }}>
                                <input type="checkbox" checked={p.config.enabled} disabled={isBusy} onChange={(e) => void toggleEnabled(p.name, e.target.checked)} />
                                {t('providers.enabled')}
                            </label>
                            <button className="btn btn-danger" disabled={isBusy} onClick={() => void removeProvider(p.name)}>
                                {t('common.delete')}
                            </button>
                        </div>

                        {/* ── Account (theo mẫu CodeBuddy, không scheduler) ── */}
                        <h3 style={{ marginTop: '1rem' }}>{t('providers.account_section')}</h3>
                        <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('providers.login_note')}</p>
                        <input
                            className="input"
                            style={{ width: '100%', marginBottom: '0.5rem' }}
                            readOnly
                            value={dashUrl}
                            onFocus={(e) => e.target.select()}
                        />
                        <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.5rem' }}>
                            <button className="btn" disabled={isBusy || !p.config.enabled} onClick={() => void checkHealth(p.name)}>
                                {t('providers.check_health')}
                            </button>
                            <button className="btn" disabled={isBusy || !p.config.enabled} onClick={() => void loadModels(p.name)}>
                                {t('providers.refresh_models')} ({list.length})
                            </button>
                        </div>
                        {list.length > 0 && (
                            <ul style={{ maxHeight: 200, overflow: 'auto', fontSize: '0.85rem' }}>
                                {list.map((m) => (
                                    <li key={m.id}>
                                        <code>{m.id}</code>{' '}
                                        <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{m.owned_by}</span>
                                    </li>
                                ))}
                            </ul>
                        )}

                        {/* ── Configuration ── */}
                        <h3 style={{ marginTop: '1rem' }}>{t('providers.config_section')}</h3>
                        <div style={{ display: 'grid', gap: '0.75rem', maxWidth: 560 }}>
                            <label>
                                {t('providers.base_url')}
                                <input
                                    className="input" style={{ width: '100%' }}
                                    value={p.config.base_url}
                                    placeholder="http://127.0.0.1:8964"
                                    disabled={!p.config.enabled || isBusy}
                                    onChange={(e) => patchConfig(p.name, { base_url: e.target.value })}
                                />
                            </label>
                            <label>
                                {t('providers.model_prefixes')}
                                <input
                                    className="input" style={{ width: '100%' }}
                                    value={p.model_prefixes}
                                    placeholder="gpt-|codex|copilot-"
                                    disabled={!p.config.enabled || isBusy}
                                    onChange={(e) => {
                                        const next = providers.map((x) => x.name === p.name ? { ...x, model_prefixes: e.target.value } : x);
                                        setProviders(next);
                                    }}
                                />
                                <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{t('providers.prefixes_hint')}</span>
                            </label>
                            <label>
                                {t('providers.api_key')}
                                <input
                                    className="input" style={{ width: '100%' }} type="password"
                                    value={p.config.api_key}
                                    placeholder="optional"
                                    disabled={!p.config.enabled || isBusy}
                                    onChange={(e) => patchConfig(p.name, { api_key: e.target.value })}
                                />
                            </label>
                            <div style={{ display: 'flex', gap: '0.75rem' }}>
                                <label style={{ flex: 1 }}>
                                    {t('providers.control_port')}
                                    <div style={{ display: 'flex', gap: '0.4rem' }}>
                                        <input
                                            className="input" style={{ width: '100%' }} type="number" min={1} max={65535}
                                            value={p.config.control_port}
                                            disabled={!p.config.enabled || isBusy}
                                            onChange={(e) => patchConfig(p.name, { control_port: num(e.target.value, p.config.control_port) })}
                                        />
                                        <button className="btn" disabled={!p.config.enabled || isBusy} onClick={() => void checkPort(p.name, p.config.control_port)}>?</button>
                                        <button className="btn" disabled={!p.config.enabled || isBusy} onClick={() => void autoPort(p.name, 'control_port')}>Auto</button>
                                    </div>
                                </label>
                                <label style={{ flex: 1 }}>
                                    {t('providers.public_port')}
                                    <div style={{ display: 'flex', gap: '0.4rem' }}>
                                        <input
                                            className="input" style={{ width: '100%' }} type="number" min={1} max={65535}
                                            value={p.config.public_port}
                                            disabled={!p.config.enabled || isBusy}
                                            onChange={(e) => patchConfig(p.name, { public_port: num(e.target.value, p.config.public_port) })}
                                        />
                                        <button className="btn" disabled={!p.config.enabled || isBusy} onClick={() => void checkPort(p.name, p.config.public_port)}>?</button>
                                        <button className="btn" disabled={!p.config.enabled || isBusy} onClick={() => void autoPort(p.name, 'public_port')}>Auto</button>
                                    </div>
                                </label>
                            </div>
                            <div style={{ display: 'flex', gap: '0.75rem' }}>
                                <label style={{ flex: 1 }}>
                                    {t('providers.timeout')}
                                    <input
                                        className="input" style={{ width: '100%' }} type="number" min={1} max={120}
                                        value={p.config.timeout_seconds}
                                        disabled={!p.config.enabled || isBusy}
                                        onChange={(e) => patchConfig(p.name, { timeout_seconds: num(e.target.value, p.config.timeout_seconds) })}
                                    />
                                </label>
                                <label style={{ flex: 1 }}>
                                    {t('providers.chat_timeout')}
                                    <input
                                        className="input" style={{ width: '100%' }} type="number" min={1} max={3600}
                                        value={p.config.chat_timeout_seconds}
                                        disabled={!p.config.enabled || isBusy}
                                        onChange={(e) => patchConfig(p.name, { chat_timeout_seconds: num(e.target.value, p.config.chat_timeout_seconds) })}
                                    />
                                </label>
                            </div>
                            <label style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                                <input
                                    type="checkbox"
                                    checked={p.config.auto_fallback}
                                    disabled={!p.config.enabled || isBusy}
                                    onChange={(e) => patchConfig(p.name, { auto_fallback: e.target.checked })}
                                />
                                {t('providers.auto_fallback')}
                            </label>
                        </div>
                        {portMsg[p.name] && <p style={{ color: 'var(--text-secondary)' }}>{portMsg[p.name]}</p>}
                        <div className="form-actions" style={{ marginTop: '1rem', display: 'flex', gap: '0.5rem' }}>
                            <button className="btn btn-primary" disabled={isBusy} onClick={() => void saveCard(p.name)}>
                                {t('common.save')}
                            </button>
                        </div>
                        {renderMsg(p.name)}
                    </div>
                );
            })}
        </div>
    );
}
