import { useEffect, useState } from 'react';
import { getSchedule, runTask, saveSchedule, type SchedulerTask } from '../../../bridge/scheduler_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { ScheduleConfig } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

type TaskKey = SchedulerTask;

const TASK_ORDER: TaskKey[] = ['checkin', 'travel', 'activity', 'keepalive'];

function hoursToText(hours: number[]): string {
    return [...hours].sort((a, b) => a - b).join(', ');
}

function textToHours(text: string): number[] {
    const out: number[] = [];
    for (const part of text.split(',')) {
        const t = part.trim();
        if (t === '') continue;
        const n = Number(t);
        if (!Number.isInteger(n) || n < 0 || n > 23) {
            throw new Error(`Invalid hour: "${t}" (must be 0..23)`);
        }
        if (!out.includes(n)) out.push(n);
    }
    return out.sort((a, b) => a - b);
}

export function SchedulerPage() {
    const { t } = useTranslation();
    const [schedule, setSchedule] = useState<ScheduleConfig | null>(null);
    const [hoursText, setHoursText] = useState<Record<TaskKey, string>>({
        checkin: '',
        travel: '',
        activity: '',
        keepalive: '',
    });
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [busy, setBusy] = useState<SchedulerTask | null>(null);
    const [message, setMessage] = useState<string | null>(null);
    const [isError, setIsError] = useState(false);

    const load = async () => {
        try {
            setMessage(null);
            const s = await getSchedule();
            setSchedule(s);
            setHoursText({
                checkin: hoursToText(s.checkin_hours),
                travel: hoursToText(s.travel_hours),
                activity: hoursToText(s.activity_hours),
                keepalive: hoursToText(s.keepalive_hours),
            });
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

    const run = async (task: SchedulerTask) => {
        try {
            setBusy(task);
            setMessage(null);
            const result = await runTask(task);
            setMessage(result.message);
            setIsError(!result.success);
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        } finally {
            setBusy(null);
        }
    };

    const save = async () => {
        if (!schedule) return;
        try {
            setSaving(true);
            setMessage(null);
            const next: ScheduleConfig = {
                ...schedule,
                checkin_hours: textToHours(hoursText.checkin),
                travel_hours: textToHours(hoursText.travel),
                activity_hours: textToHours(hoursText.activity),
                keepalive_hours: textToHours(hoursText.keepalive),
            };
            await saveSchedule(next);
            setSchedule(next);
            setMessage(t('common.save'));
            setIsError(false);
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        } finally {
            setSaving(false);
        }
    };

    const toggle = (task: TaskKey) => {
        if (!schedule) return;
        setSchedule({ ...schedule, [`${task}_enabled`]: !schedule[`${task}_enabled` as keyof ScheduleConfig] });
    };

    if (loading) return <div>{t('common.loading')}</div>;
    if (!schedule) return <div><p className="error-message">{message}</p></div>;

    return (
        <div>
            <h1>{t('scheduler.title')}</h1>
            <div className="card">
                <h3>{t('scheduler.tasks')}</h3>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('scheduler.hint')}</p>
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
                        {TASK_ORDER.map((task) => {
                            const enabled = schedule[`${task}_enabled` as keyof ScheduleConfig] as boolean;
                            return (
                                <tr key={task}>
                                    <td>{t(`scheduler.${task}`)}</td>
                                    <td>
                                        <input
                                            className="input"
                                            style={{ width: '12rem' }}
                                            value={hoursText[task]}
                                            placeholder="9, 21"
                                            onChange={(e) => setHoursText({ ...hoursText, [task]: e.target.value })}
                                        />
                                    </td>
                                    <td>
                                        <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', cursor: 'pointer' }}>
                                            <input type="checkbox" checked={enabled} onChange={() => toggle(task)} />
                                            <span className={`badge ${enabled ? 'badge-success' : 'badge-warning'}`}>
                                                {enabled ? 'ON' : 'OFF'}
                                            </span>
                                        </label>
                                    </td>
                                    <td>
                                        <button
                                            className="btn btn-primary"
                                            onClick={() => void run(task)}
                                            disabled={busy !== null || saving}
                                        >
                                            {busy === task ? t('common.loading') : t('scheduler.run_now')}
                                        </button>
                                    </td>
                                </tr>
                            );
                        })}
                    </tbody>
                </table>
                <div className="form-actions">
                    <button className="btn" onClick={() => void load()} disabled={saving || busy !== null}>
                        {t('common.refresh')}
                    </button>
                    <button className="btn btn-primary" onClick={() => void save()} disabled={saving || busy !== null}>
                        {t('common.save')}
                    </button>
                </div>
                {message && <p className={isError ? 'error-message' : 'success-message'}>{message}</p>}
            </div>
        </div>
    );
}
