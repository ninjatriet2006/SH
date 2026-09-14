import { invokeIpc } from './ipc';
import type { GatewayInfo } from './types';

export async function getGatewayStatus(): Promise<GatewayInfo> {
    return await invokeIpc<GatewayInfo>('get_gateway_status');
}

export async function startGateway(configPath?: string): Promise<void> {
    await invokeIpc<Record<string, never>>('start_gateway', { config_path: configPath ?? null });
}

export async function stopGateway(): Promise<void> {
    await invokeIpc<Record<string, never>>('stop_gateway');
}
