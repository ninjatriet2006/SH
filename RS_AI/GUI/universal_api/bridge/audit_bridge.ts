import { invokeIpc } from './ipc';
import type { TrafficAuditLog } from './types';

export async function getTrafficLogs(): Promise<TrafficAuditLog[]> {
  return invokeIpc<TrafficAuditLog[]>('get_traffic_logs');
}

export async function clearTrafficLogs(): Promise<void> {
  await invokeIpc<Record<string, never>>('clear_traffic_logs');
}
