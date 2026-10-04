import { useEffect, useState } from 'react';
import { getZedConfig, saveZedConfig } from '../../../../bridge/zed_bridge';
import { ipcErrorMessage } from '../../../../bridge/ipc';
import { useTranslation } from '../../utils/i18n';

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

    if (loading) {
        return <div className="muted">{t('common.loading')}</div>;
    }

    return (
        <div className="card">
            <h2>Zed Cloud Configuration</h2>
            <p className="muted">
                {t('zed.config_notice') || 'Tùy chọn backend Zed: kích hoạt endpoint upstream và định danh system_id'}
            </p>

            {msg && (
                <div className={`status-bar ${msg.ok ? 'ok' : 'err'}`} role="status">
                    {msg.text}
                </div>
            )}

            <div className="form-group">
                <label className="checkbox-label">
                    <input
                        type="checkbox"
                        checked={zedEnabled}
                        onChange={(e) => setZedEnabledCfg(e.target.checked)}
                    />
                    <span>{t('zed.enable_upstream') || 'Kích hoạt định tuyến Zed Cloud'}</span>
                </label>
            </div>

            <div className="form-group">
                <label htmlFor="zed-system-id">System ID (tùy chọn — dùng gán telemetry / định danh máy trạm)</label>
                <input
                    id="zed-system-id"
                    type="text"
                    value={systemId}
                    onChange={(e) => setSystemId(e.target.value)}
                    placeholder="ví dụ: dev-workstation-01"
                />
            </div>

            <div className="card-actions">
                <button
                    type="button"
                    className="btn primary"
                    onClick={() => void saveCfg()}
                    disabled={working}
                >
                    {working ? t('common.loading') : t('common.save')}
                </button>
            </div>
        </div>
    );
}
