import { invokeIpc } from './ipc';
import type { TaskResult } from './types';

const taskCommands = {
    checkin: 'run_checkin_now',
    travel: 'run_travel_now',
    activity: 'run_activity_now',
    keepalive: 'run_keepalive_now',
} as const;

export type SchedulerTask = keyof typeof taskCommands;

export function runTask(task: SchedulerTask): Promise<TaskResult> {
    return invokeIpc<TaskResult>(taskCommands[task]);
}
