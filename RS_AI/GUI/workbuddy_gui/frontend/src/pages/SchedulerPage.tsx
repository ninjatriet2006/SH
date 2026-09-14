import { useTranslation } from '../utils/i18n';

export function SchedulerPage() {
    const { t } = useTranslation();
    const [busy, setBusy] = useState<SchedulerTask | null>(null);
    const [message, setMessage] = useState<string | null>(null);
    const run = async (task: SchedulerTask) => {
        try { setBusy(task); setMessage(null); const result = await runTask(task); setMessage(result.message); }
        catch (e) { setMessage(ipcErrorMessage(e)); }
        finally { setBusy(null); }
    };
    const rows: Array<[SchedulerTask, string, string]> = [['checkin', t('scheduler.checkin'), '09:00, 21:00'], ['travel', t('scheduler.travel'), '09:00, 21:00'], ['activity', t('scheduler.activity'), '10:00'], ['keepalive', t('scheduler.keepalive'), '22:00']];
    return (
        <div>
            <h1>{t('scheduler.title')}</h1>
            <div className="card">
                <h3>{t('scheduler.tasks')}</h3>
                <table>
                    <thead>
                        <tr>
                            <th>{t('scheduler.col_task')}</th>
                            <th>{t('scheduler.col_hours')}</th>
                            <th>{t('scheduler.col_enabled')}</th>
                            <th>{t('scheduler.col_actions')}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {rows.map(([task, name, hours]) => <tr key={task}><td>{name}</td><td>{hours}</td><td><span className="badge badge-success">ON</span></td><td><button className="btn btn-primary" onClick={() => void run(task)} disabled={busy !== null}>{busy === task ? t('common.loading') : t('scheduler.run_now')}</button></td></tr>)}
                    </tbody>
                </table>
                {message && <p className="status-message">{message}</p>}
            </div>
        </div>
    );
}
import { useState } from 'react';
import { runTask, type SchedulerTask } from '../../../bridge/scheduler_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
