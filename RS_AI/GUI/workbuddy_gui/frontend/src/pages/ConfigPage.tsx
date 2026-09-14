import { useTranslation } from '../utils/i18n';

export function ConfigPage() {
    const { t } = useTranslation();
    const [value, setValue] = useState('');
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [message, setMessage] = useState<string | null>(null);
    const load = async () => {
        try { setMessage(null); setValue(JSON.stringify(await getConfig(), null, 2)); }
        catch (e) { setMessage(ipcErrorMessage(e)); }
        finally { setLoading(false); }
    };
    useEffect(() => { void load(); }, []);
    const save = async () => {
        try { JSON.parse(value); setSaving(true); setMessage(null); await saveConfig(value); setMessage(t('common.save')); }
        catch (e) { setMessage(ipcErrorMessage(e)); }
        finally { setSaving(false); }
    };
    return (
        <div>
            <h1>{t('config.title')}</h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)' }}>{t('config.description')}</p>
                <textarea className="config-editor" value={value} onChange={(e) => setValue(e.target.value)} disabled={loading || saving} spellCheck={false} />
                <div className="form-actions"><button className="btn" onClick={() => void load()} disabled={saving}>{t('common.refresh')}</button><button className="btn btn-primary" onClick={() => void save()} disabled={loading || saving}>{t('common.save')}</button></div>
                {message && <p className={message === t('common.save') ? 'success-message' : 'error-message'}>{message}</p>}
            </div>
        </div>
    );
}
import { useEffect, useState } from 'react';
import { getConfig, saveConfig } from '../../../bridge/config_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
