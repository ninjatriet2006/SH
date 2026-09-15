import { invokeIpc } from './ipc';
import type { TraceRecord, UsageLog, UsageSummary } from './types';

export async function listUsageLogs(limit = 100): Promise<UsageLog[]> {
    return invokeIpc<UsageLog[]>('list_usage_logs', { limit });
}

export async function getUsageSummary(): Promise<UsageSummary[]> {
    return invokeIpc<UsageSummary[]>('get_usage_summary');
}

export async function getDebugTraces(): Promise<TraceRecord[]> {
    return invokeIpc<TraceRecord[]>('get_debug_traces');
}
