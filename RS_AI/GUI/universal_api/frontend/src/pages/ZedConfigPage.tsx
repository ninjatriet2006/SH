import { useEffect, useState } from 'react';
import { getZedConfig, saveZedConfig } from '../../../bridge/zed_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useTranslation } from '../utils/i18n';

// Zed/Configuration — song song CodeBuddy/Config: enabled + system_id.
// Host/timeout cố định theo tham chiếu (không phơi tham số thừa).
export function ZedConfigPage() {
    const { t } = useTranslation();
    const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);
    const [working, setWorking] = useState(false);
    const [loading, setLoading] = useState(true);
    const [zedEnabled, setZedEnabledCfg] = useState(true);
    const [systemId, setSystemId] = useState('');

    const reload = async () => {
        try {
            const cfg = await getZedConfig();
            setZedEnabledCfg(cfg.enabled);
            setSystemId(cfg.system_id);
        } catch (e) {
            setMsg({ text: ipcErrorMessage(e), ok: false });
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void reload();
    }, []);

    const saveCfg = async () => {
        try {
            setWorking(true);
            setMsg(null);
            await saveZedConfig({ enabled: zedEnabled, system_id: systemId.trim() });
            setMsg({ text: t('common.save'), ok: true });
        } catch (e) {
            setMsg({ text: ipcErrorMessage(e), ok: false });
        } finally {
            setWorking(false);
        }
    };

    if (loading) return <div>{t('common.loading')}</div>;

    return (
        <div>
            <h1>{t('sidebar.zed')} — {t('providers.config_section')}</h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('providers.zed_config_note')}</p>
                <label style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginTop: '0.5rem' }}>
                    <input
                        type="checkbox"
                        checked={zedEnabled}
                        disabled={working}
                        onChange={(e) => setZedEnabledCfg(e.target.checked)}
                    />
                    {t('providers.enabled')}
                </label>
                <label style={{ display: 'grid', gap: '0.3rem', marginTop: '0.75rem', maxWidth: 480 }}>
                    {t('providers.zed_system_id')}
                    <input
                        className="input"
                        value={systemId}
                        placeholder="A-Za-z0-9._:- (trống = dùng env)"
                        disabled={working}
                        onChange={(e) => setSystemId(e.target.value)}
                    />
                </label>
                <div style={{ marginTop: '0.75rem' }}>
                    <button className="btn btn-primary" disabled={working} onClick={() => void saveCfg()}>
                        {t('common.save')}
                    </button>
                </div>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '0.5rem' }}>{t('providers.zed_hint_refresh')}</p>
            </div>

            {msg && <p className={msg.ok ? 'success-message' : 'error-message'}>{msg.text}</p>}
        </div>
    );
}
