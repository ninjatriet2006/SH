import { useEffect, useState } from 'react';
import {
    getExternalConfig,
    saveExternalConfig,
    getExternalStatus,
    refreshExternalModels,
    checkPortAvailable,
    suggestFreePort,
} from '../../../bridge/external_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { ExternalConfig, ExternalModel, ExternalStatus } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

const DEFAULT_CFG: ExternalConfig = {
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

export function ProvidersPage() {
    const { t } = useTranslation();
    const [cfg, setCfg] = useState<ExternalConfig>(DEFAULT_CFG);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [status, setStatus] = useState<ExternalStatus | null>(null);
    const [checking, setChecking] = useState(false);
    const [models, setModels] = useState<ExternalModel[]>([]);
    const [modelsLoading, setModelsLoading] = useState(false);
    const [message, setMessage] = useState<string | null>(null);
    const [portMsg, setPortMsg] = useState<string | null>(null);

    const load = async () => {
        try {
            setMessage(null);
            setCfg(await getExternalConfig());
        } catch (e) {
            setMessage(ipcErrorMessage(e));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load();
    }, []);

    const save = async () => {
        try {
            setSaving(true);
            setMessage(null);
            if (!validPort(cfg.control_port) || !validPort(cfg.public_port)) {
                throw new Error(t('providers.invalid_port'));
            }
            if (cfg.control_port === cfg.public_port) {
                throw new Error(t('providers.ports_must_differ'));
            }
            await saveExternalConfig(cfg);
            setMessage(t('common.save'));
        } catch (e) {
            setMessage(ipcErrorMessage(e));
        } finally {
            setSaving(false);
        }
    };

    const checkHealth = async () => {
        try {
            setChecking(true);
            setMessage(null);
            setStatus(await getExternalStatus());
        } catch (e) {
            setMessage(ipcErrorMessage(e));
        } finally {
            setChecking(false);
        }
    };

    const loadModels = async () => {
        try {
            setModelsLoading(true);
            setMessage(null);
            setModels(await refreshExternalModels());
        } catch (e) {
            setMessage(ipcErrorMessage(e));
        } finally {
            setModelsLoading(false);
        }
    };

    const checkPort = async (port: number) => {
        try {
            setPortMsg(null);
            const r = await checkPortAvailable(port);
            if (r.conflicts_gateway) {
                setPortMsg(t('providers.port_conflicts_gateway'));
            } else {
                setPortMsg(r.available ? t('providers.port_free') : t('providers.port_busy'));
            }
        } catch (e) {
            setPortMsg(ipcErrorMessage(e));
        }
    };

    const autoPort = async (field: 'control_port' | 'public_port') => {
        try {
            const r = await suggestFreePort(cfg[field]);
            setCfg({ ...cfg, [field]: r.port });
        } catch (e) {
            setMessage(ipcErrorMessage(e));
        }
    };

    const num = (v: string, fallback: number): number => {
        const n = Number(v);
        return Number.isFinite(n) ? Math.floor(n) : fallback;
    };

    if (loading) return <div>{t('common.loading')}</div>;

    return (
        <div>
            <h1>{t('providers.title')} <span className="badge badge-info">Anti-API</span></h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)' }}>{t('providers.description')}</p>
                <div style={{ marginTop: '0.75rem', padding: '0.75rem', border: '1px solid var(--border)', borderRadius: '0.5rem' }}>
                    <strong>{t('providers.login_title')}</strong>
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', margin: '0.4rem 0' }}>
                        {t('providers.login_note')}
                    </p>
                    <input
                        className="input"
                        style={{ width: '100%' }}
                        readOnly
                        value={`${cfg.base_url.replace(/\/$/, '')}/quota`}
                        onFocus={(e) => e.target.select()}
                    />
                </div>
                <label style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginTop: '1rem' }}>
                    <input
                        type="checkbox"
                        checked={cfg.enabled}
                        onChange={(e) => setCfg({ ...cfg, enabled: e.target.checked })}
                    />
                    {t('providers.enabled')}
                </label>

                <div style={{ display: 'grid', gap: '0.75rem', marginTop: '1rem', maxWidth: 560 }}>
                    <label>
                        {t('providers.base_url')}
                        <input
                            className="input"
                            style={{ width: '100%' }}
                            value={cfg.base_url}
                            onChange={(e) => setCfg({ ...cfg, base_url: e.target.value })}
                            placeholder="http://127.0.0.1:8964"
                            disabled={!cfg.enabled}
                        />
                    </label>
                    <label>
                        {t('providers.api_key')}
                        <input
                            className="input"
                            style={{ width: '100%' }}
                            type="password"
                            value={cfg.api_key}
                            onChange={(e) => setCfg({ ...cfg, api_key: e.target.value })}
                            placeholder="optional"
                            disabled={!cfg.enabled}
                        />
                    </label>
                    <div style={{ display: 'flex', gap: '0.75rem' }}>
                        <label style={{ flex: 1 }}>
                            {t('providers.control_port')}
                            <div style={{ display: 'flex', gap: '0.4rem' }}>
                                <input
                                    className="input"
                                    style={{ width: '100%' }}
                                    type="number"
                                    min={1}
                                    max={65535}
                                    value={cfg.control_port}
                                    onChange={(e) => setCfg({ ...cfg, control_port: num(e.target.value, cfg.control_port) })}
                                    disabled={!cfg.enabled}
                                />
                                <button className="btn" onClick={() => void checkPort(cfg.control_port)} disabled={!cfg.enabled}>
                                    ?
                                </button>
                                <button className="btn" onClick={() => void autoPort('control_port')} disabled={!cfg.enabled}>
                                    Auto
                                </button>
                            </div>
                        </label>
                        <label style={{ flex: 1 }}>
                            {t('providers.public_port')}
                            <div style={{ display: 'flex', gap: '0.4rem' }}>
                                <input
                                    className="input"
                                    style={{ width: '100%' }}
                                    type="number"
                                    min={1}
                                    max={65535}
                                    value={cfg.public_port}
                                    onChange={(e) => setCfg({ ...cfg, public_port: num(e.target.value, cfg.public_port) })}
                                    disabled={!cfg.enabled}
                                />
                                <button className="btn" onClick={() => void checkPort(cfg.public_port)} disabled={!cfg.enabled}>
                                    ?
                                </button>
                                <button className="btn" onClick={() => void autoPort('public_port')} disabled={!cfg.enabled}>
                                    Auto
                                </button>
                            </div>
                        </label>
                    </div>
                    <div style={{ display: 'flex', gap: '0.75rem' }}>
                        <label style={{ flex: 1 }}>
                            {t('providers.timeout')}
                            <input
                                className="input"
                                style={{ width: '100%' }}
                                type="number"
                                min={1}
                                max={120}
                                value={cfg.timeout_seconds}
                                onChange={(e) => setCfg({ ...cfg, timeout_seconds: num(e.target.value, cfg.timeout_seconds) })}
                                disabled={!cfg.enabled}
                            />
                        </label>
                        <label style={{ flex: 1 }}>
                            {t('providers.chat_timeout')}
                            <input
                                className="input"
                                style={{ width: '100%' }}
                                type="number"
                                min={1}
                                max={3600}
                                value={cfg.chat_timeout_seconds}
                                onChange={(e) => setCfg({ ...cfg, chat_timeout_seconds: num(e.target.value, cfg.chat_timeout_seconds) })}
                                disabled={!cfg.enabled}
                            />
                        </label>
                    </div>
                    <label style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                        <input
                            type="checkbox"
                            checked={cfg.auto_fallback}
                            onChange={(e) => setCfg({ ...cfg, auto_fallback: e.target.checked })}
                            disabled={!cfg.enabled}
                        />
                        {t('providers.auto_fallback')}
                    </label>
                </div>

                {portMsg && <p style={{ color: 'var(--text-secondary)' }}>{portMsg}</p>}

                <div className="form-actions" style={{ marginTop: '1rem', display: 'flex', gap: '0.5rem' }}>
                    <button className="btn btn-primary" onClick={() => void save()} disabled={saving}>
                        {t('common.save')}
                    </button>
                    <button className="btn" onClick={() => void checkHealth()} disabled={checking || !cfg.enabled}>
                        {t('providers.check_health')}
                    </button>
                    <button className="btn" onClick={() => void loadModels()} disabled={modelsLoading || !cfg.enabled}>
                        {t('providers.refresh_models')}
                    </button>
                </div>
                {message && <p className={message === t('common.save') ? 'success-message' : 'error-message'}>{message}</p>}
                {status && (
                    <p style={{ color: status.reachable ? 'var(--success, green)' : 'var(--danger, red)' }}>
                        {status.reachable
                            ? `OK${status.http_status ? ` HTTP ${status.http_status}` : ''}${status.latency_ms != null ? ` · ${status.latency_ms}ms` : ''}`
                            : (status.error ?? 'unreachable')}
                    </p>
                )}
            </div>

            <div className="card" style={{ marginTop: '1rem' }}>
                <h3>
                    {t('providers.models')} ({models.length})
                </h3>
                {models.length === 0 ? (
                    <p style={{ color: 'var(--text-secondary)' }}>{t('providers.no_models')}</p>
                ) : (
                    <ul style={{ maxHeight: 320, overflow: 'auto' }}>
                        {models.map((m) => (
                            <li key={m.id}>
                                <code>{m.id}</code>{' '}
                                <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>{m.owned_by}</span>
                            </li>
                        ))}
                    </ul>
                )}
            </div>
        </div>
    );
}
