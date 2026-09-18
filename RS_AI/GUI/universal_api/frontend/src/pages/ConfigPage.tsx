import { useEffect, useState } from 'react';
import { getConfig, saveConfig } from '../../../bridge/config_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useTranslation } from '../utils/i18n';

type Cfg = Record<string, any>;

function get(cfg: Cfg, path: string[]): any {
    let cur: any = cfg;
    for (const p of path) {
        if (cur == null || typeof cur !== 'object') return undefined;
        cur = cur[p];
    }
    return cur;
}

function set(cfg: Cfg, path: string[], value: any): Cfg {
    const next: Cfg = Array.isArray(cfg) ? [...cfg] : { ...cfg };
    let cur: any = next;
    for (let i = 0; i < path.length - 1; i++) {
        const p = path[i];
        const child = cur[p];
        cur[p] = child != null && typeof child === 'object' ? (Array.isArray(child) ? [...child] : { ...child }) : {};
        cur = cur[p];
    }
    cur[path[path.length - 1]] = value;
    return next;
}

function TextField(props: {
    label: string;
    value: string;
    password?: boolean;
    placeholder?: string;
    disabled?: boolean;
    onChange: (v: string) => void;
}) {
    return (
        <label style={{ display: 'grid', gap: '0.3rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
            <code style={{ color: 'var(--text-primary)' }}>{props.label}</code>
            <input
                className="input"
                type={props.password ? 'password' : 'text'}
                value={props.value}
                placeholder={props.placeholder}
                disabled={props.disabled}
                onChange={(e) => props.onChange(e.target.value)}
            />
        </label>
    );
}

function NumField(props: {
    label: string;
    value: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
    onChange: (v: number) => void;
}) {
    return (
        <label style={{ display: 'grid', gap: '0.3rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
            <code style={{ color: 'var(--text-primary)' }}>{props.label}</code>
            <input
                className="input"
                type="number"
                value={Number.isFinite(props.value) ? props.value : 0}
                min={props.min}
                max={props.max}
                step={props.step ?? 1}
                disabled={props.disabled}
                onChange={(e) => props.onChange(Number(e.target.value))}
            />
        </label>
    );
}

function BoolField(props: { label: string; value: boolean; disabled?: boolean; onChange: (v: boolean) => void }) {
    return (
        <label style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', fontSize: '0.85rem', cursor: 'pointer' }}>
            <input type="checkbox" checked={!!props.value} disabled={props.disabled} onChange={(e) => props.onChange(e.target.checked)} />
            <code>{props.label}</code>
        </label>
    );
}

function Section(props: { title: string; children: React.ReactNode }) {
    return (
        <div className="card">
            <h3 style={{ marginBottom: '0.75rem' }}>{props.title}</h3>
            <div style={{ display: 'grid', gap: '0.75rem', maxWidth: '36rem' }}>{props.children}</div>
        </div>
    );
}

export function ConfigPage() {
    const { t } = useTranslation();
    const [cfg, setCfg] = useState<Cfg | null>(null);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [message, setMessage] = useState<string | null>(null);
    const [isError, setIsError] = useState(false);
    const [showRaw, setShowRaw] = useState(false);
    const [raw, setRaw] = useState('');

    const load = async () => {
        try {
            setMessage(null);
            const c = (await getConfig()) as Cfg;
            setCfg(c);
            setRaw(JSON.stringify(c, null, 2));
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load();
    }, []);

    const update = (path: string[], value: any) => {
        setCfg((prev) => (prev ? set(prev, path, value) : prev));
    };

    const applyRaw = () => {
        try {
            const parsed = JSON.parse(raw);
            if (parsed == null || typeof parsed !== 'object') throw new Error('Config must be a JSON object');
            setCfg(parsed);
            setMessage(t('config.raw_applied'));
            setIsError(false);
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        }
    };

    const save = async () => {
        if (!cfg) return;
        try {
            setSaving(true);
            setMessage(null);
            const listen = String(get(cfg, ['listen']) ?? '').trim();
            if (!listen) throw new Error(t('config.err_listen'));
            for (const p of [
                ['server', 'max_body_mb'],
                ['upstream', 'timeout_seconds'],
                ['pool', 'max_in_flight'],
                ['pool', 'breaker_threshold'],
            ]) {
                const v = get(cfg, p);
                if (v !== undefined && (!Number.isFinite(Number(v)) || Number(v) < 0)) {
                    throw new Error(`${p.join('.')}: must be a number >= 0`);
                }
            }
            await saveConfig(JSON.stringify(cfg));
            setRaw(JSON.stringify(cfg, null, 2));
            setMessage(t('common.save'));
            setIsError(false);
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        } finally {
            setSaving(false);
        }
    };

    if (loading) return <div>{t('common.loading')}</div>;
    if (!cfg) return <div><p className="error-message">{message}</p></div>;

    const str = (p: string[]): string => {
        const v = get(cfg, p);
        return v == null ? '' : String(v);
    };

    return (
        <div>
            <h1>{t('config.title')}</h1>
            <p style={{ color: 'var(--text-secondary)' }}>{t('config.description')}</p>

            <Section title={t('config.sec_connection')}>
                <TextField label="listen" value={str(['listen'])} placeholder=":7863" onChange={(v) => update(['listen'], v)} />
                <TextField label="api_key" password value={str(['api_key'])} placeholder="(empty = open access)" onChange={(v) => update(['api_key'], v)} />
                <TextField label="auth_dir" value={str(['auth_dir'])} onChange={(v) => update(['auth_dir'], v)} />
                <TextField label="state_file" value={str(['state_file'])} onChange={(v) => update(['state_file'], v)} />
            </Section>

            <Section title={t('config.sec_upstream')}>
                <TextField label="upstream.proxy_url" value={str(['upstream', 'proxy_url'])} placeholder="(empty = direct)" onChange={(v) => update(['upstream', 'proxy_url'], v)} />
                <TextField label="upstream.user_agent" value={str(['upstream', 'user_agent'])} placeholder="(empty = default)" onChange={(v) => update(['upstream', 'user_agent'], v)} />
                <TextField label="upstream.realm" value={str(['upstream', 'realm'])} placeholder="(empty = auto)" onChange={(v) => update(['upstream', 'realm'], v)} />
                <NumField label="upstream.timeout_seconds" value={Number(get(cfg, ['upstream', 'timeout_seconds']) ?? 120)} min={1} onChange={(v) => update(['upstream', 'timeout_seconds'], v)} />
                <NumField label="upstream.header_timeout_seconds" value={Number(get(cfg, ['upstream', 'header_timeout_seconds']) ?? 0)} min={0} onChange={(v) => update(['upstream', 'header_timeout_seconds'], v)} />
                <NumField label="upstream.idle_timeout_seconds" value={Number(get(cfg, ['upstream', 'idle_timeout_seconds']) ?? 0)} min={0} onChange={(v) => update(['upstream', 'idle_timeout_seconds'], v)} />
                <NumField label="server.max_body_mb" value={Number(get(cfg, ['server', 'max_body_mb']) ?? 8)} min={1} onChange={(v) => update(['server', 'max_body_mb'], v)} />
            </Section>

            <Section title={t('config.sec_pool')}>
                <NumField label="pool.max_in_flight" value={Number(get(cfg, ['pool', 'max_in_flight']) ?? 3)} min={1} onChange={(v) => update(['pool', 'max_in_flight'], v)} />
                <NumField label="pool.breaker_threshold" value={Number(get(cfg, ['pool', 'breaker_threshold']) ?? 3)} min={1} onChange={(v) => update(['pool', 'breaker_threshold'], v)} />
                <TextField label="pool.breaker_cooldown" value={str(['pool', 'breaker_cooldown'])} placeholder="30m" onChange={(v) => update(['pool', 'breaker_cooldown'], v)} />
                <TextField label="pool.breaker_cooldown_max" value={str(['pool', 'breaker_cooldown_max'])} placeholder="6h" onChange={(v) => update(['pool', 'breaker_cooldown_max'], v)} />
                <NumField label="pool.idle_weight_per_hour" value={Number(get(cfg, ['pool', 'idle_weight_per_hour']) ?? 0.5)} min={0} step={0.1} onChange={(v) => update(['pool', 'idle_weight_per_hour'], v)} />
                <NumField label="pool.idle_weight_max" value={Number(get(cfg, ['pool', 'idle_weight_max']) ?? 5)} min={0} step={0.5} onChange={(v) => update(['pool', 'idle_weight_max'], v)} />
            </Section>

            <Section title={t('config.sec_prompt')}>
                <label style={{ display: 'grid', gap: '0.3rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                    <code style={{ color: 'var(--text-primary)' }}>prompt.mode</code>
                    <select
                        className="input"
                        value={str(['prompt', 'mode']) || 'custom'}
                        onChange={(e) => update(['prompt', 'mode'], e.target.value)}
                    >
                        <option value="custom">custom</option>
                        <option value="passthrough">passthrough</option>
                    </select>
                </label>
                <TextField label="prompt.file" value={str(['prompt', 'file'])} placeholder="(empty = built-in)" onChange={(v) => update(['prompt', 'file'], v)} />
                <TextField label="cooldown.soft_rate" value={str(['cooldown', 'soft_rate'])} onChange={(v) => update(['cooldown', 'soft_rate'], v)} />
                <TextField label="cooldown.soft_rate_max" value={str(['cooldown', 'soft_rate_max'])} onChange={(v) => update(['cooldown', 'soft_rate_max'], v)} />
                <BoolField label="session_sticky.enabled" value={!!get(cfg, ['session_sticky', 'enabled'])} onChange={(v) => update(['session_sticky', 'enabled'], v)} />
                <TextField label="session_sticky.ttl" value={str(['session_sticky', 'ttl'])} onChange={(v) => update(['session_sticky', 'ttl'], v)} />
                <TextField label="session_sticky.gc_interval" value={str(['session_sticky', 'gc_interval'])} onChange={(v) => update(['session_sticky', 'gc_interval'], v)} />
                <BoolField label="features.sanitize_blacklist_fingerprints" value={!!get(cfg, ['features', 'sanitize_blacklist_fingerprints'])} onChange={(v) => update(['features', 'sanitize_blacklist_fingerprints'], v)} />
            </Section>

            <div className="card">
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('config.owned_note')}</p>
            </div>

            <div className="card">
                <button className="btn" onClick={() => { setRaw(JSON.stringify(cfg, null, 2)); setShowRaw((s) => !s); }}>
                    {showRaw ? t('config.hide_raw') : t('config.show_raw')}
                </button>
                {showRaw && (
                    <div style={{ marginTop: '0.75rem', display: 'grid', gap: '0.5rem' }}>
                        <textarea className="config-editor" value={raw} onChange={(e) => setRaw(e.target.value)} spellCheck={false} />
                        <div><button className="btn" onClick={applyRaw}>{t('config.apply_raw')}</button></div>
                    </div>
                )}
            </div>

            <div className="form-actions" style={{ position: 'sticky', bottom: 0, padding: '0.75rem 0' }}>
                <button className="btn" onClick={() => void load()} disabled={saving}>{t('common.refresh')}</button>
                <button className="btn btn-primary" onClick={() => void save()} disabled={saving}>{t('common.save')}</button>
            </div>
            {message && <p className={isError ? 'error-message' : 'success-message'}>{message}</p>}
        </div>
    );
}
