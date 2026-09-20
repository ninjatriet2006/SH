import { invokeIpc } from './ipc';
import type { ScheduleConfig, TaskResult } from './types';

// Chỉ còn keepalive (bản intl không có checkin/travel/activity).
export async function runKeepalive(): Promise<TaskResult> {
    return invokeIpc<TaskResult>('run_keepalive_now');
}

export async function getSchedule(): Promise<ScheduleConfig> {
    return await invokeIpc<ScheduleConfig>('get_schedule');
}

export async function saveSchedule(schedule: ScheduleConfig): Promise<void> {
    await invokeIpc<Record<string, never>>('save_schedule', { schedule });
}
