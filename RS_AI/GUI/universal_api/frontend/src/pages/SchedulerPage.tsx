import { useEffect, useState } from 'react';
import { getSchedule, runKeepalive, saveSchedule } from '../../../bridge/scheduler_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { ScheduleConfig } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

// Trang lịch rút gọn: chỉ keepalive (bản intl không có checkin/travel/activity).
// Giữ 1 dòng duy nhất: giờ chạy + bật/tắt + chạy ngay.

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
    const [hoursText, setHoursText] = useState('');
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [busy, setBusy] = useState(false);
    const [message, setMessage] = useState<string | null>(null);
    const [isError, setIsError] = useState(false);

    const load = async () => {
        try {
            setMessage(null);
            const s = await getSchedule();
            setSchedule(s);
            setHoursText(hoursToText(s.keepalive_hours ?? []));
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

    const run = async () => {
        try {
            setBusy(true);
            setMessage(null);
            const result = await runKeepalive();
            setMessage(result.message);
            setIsError(!result.success);
        } catch (e) {
            setMessage(ipcErrorMessage(e));
            setIsError(true);
        } finally {
            setBusy(false);
        }
    };

    const save = async () => {
        if (!schedule) return;
        try {
            setSaving(true);
            setMessage(null);
            const next: ScheduleConfig = {
                ...schedule,
                keepalive_hours: textToHours(hoursText),
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
                        <tr>
                            <td>{t('scheduler.keepalive')}</td>
                            <td>
                                <input
                                    className="input"
                                    style={{ width: '12rem' }}
                                    value={hoursText}
                                    placeholder="22"
                                    onChange={(e) => setHoursText(e.target.value)}
                                />
                            </td>
                            <td>
                                <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', cursor: 'pointer' }}>
                                    <input
                                        type="checkbox"
                                        checked={!!schedule.keepalive_enabled}
                                        onChange={() => setSchedule({ ...schedule, keepalive_enabled: !schedule.keepalive_enabled })}
                                    />
                                    <span className={`badge ${schedule.keepalive_enabled ? 'badge-success' : 'badge-warning'}`}>
                                        {schedule.keepalive_enabled ? 'ON' : 'OFF'}
                                    </span>
                                </label>
                            </td>
                            <td>
                                <button
                                    className="btn btn-primary"
                                    onClick={() => void run()}
                                    disabled={busy || saving}
                                >
                                    {busy ? t('common.loading') : t('scheduler.run_now')}
                                </button>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <div className="form-actions">
                    <button className="btn" onClick={() => void load()} disabled={saving || busy}>
                        {t('common.refresh')}
                    </button>
                    <button className="btn btn-primary" onClick={() => void save()} disabled={saving || busy}>
                        {t('common.save')}
                    </button>
                </div>
                {message && <p className={isError ? 'error-message' : 'success-message'}>{message}</p>}
            </div>
        </div>
    );
}
