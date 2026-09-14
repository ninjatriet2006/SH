import { useEffect, useState } from 'react';
import { getGatewayStatus, startGateway, stopGateway } from '../../../bridge/gateway_bridge';
import { runTask, type SchedulerTask } from '../../../bridge/scheduler_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { GatewayInfo, TaskResult } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

export function DashboardPage() {
    const { t } = useTranslation();
    const [gateway, setGateway] = useState<GatewayInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [taskResult, setTaskResult] = useState<TaskResult | null>(null);

    const refresh = async () => {
        try {
            setError(null);
            setGateway(await getGatewayStatus());
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setLoading(false); }
    };

    useEffect(() => { void refresh(); }, []);

    const toggleGateway = async () => {
        try {
            setBusy(true); setError(null);
            if (gateway?.status === 'running') await stopGateway(); else await startGateway();
            await refresh();
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const executeTask = async (task: SchedulerTask) => {
        try { setBusy(true); setError(null); setTaskResult(await runTask(task)); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const status = gateway?.status ?? 'stopped';
    return (
        <div>
            <h1>{t('dashboard.title')}</h1>
            <div className="card">
                <h3>{t('dashboard.gateway_status')}</h3>
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
                        <div>{gateway?.listen ?? '--'}</div>
                    </div>
                    <div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>Requests</div>
                        <div>{gateway?.requests_total ?? 0}</div>
                    </div>
                </div>
                <div style={{ marginTop: '1rem' }}>
                    <button className="btn btn-primary" onClick={() => void toggleGateway()} disabled={busy || loading}>{status === 'running' ? t('dashboard.stop') : t('dashboard.start')}</button>
                    <button className="btn" onClick={() => void refresh()} disabled={busy}>{t('common.refresh')}</button>
                </div>
                {gateway?.error && <p className="error-message">{gateway.error}</p>}
                {error && <p className="error-message">{error}</p>}
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
