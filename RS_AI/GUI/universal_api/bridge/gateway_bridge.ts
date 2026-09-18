import { invokeIpc } from './ipc';
import type { GatewayInfo, ServingInfo } from './types';

export async function getGatewayStatus(): Promise<GatewayInfo> {
    return await invokeIpc<GatewayInfo>('get_gateway_status');
}

export async function getServingAccount(): Promise<ServingInfo> {
    return await invokeIpc<ServingInfo>('get_serving_account');
}

export interface GatewayModel {
    id: string;
    object: string;
    created: number;
    owned_by: string;
    context_length?: number;
    max_output_tokens?: number;
}

/// Refresh tay danh sách model động (không chờ TTL). Fail → throw kèm lý do.
export async function refreshGatewayModels(): Promise<GatewayModel[]> {
    return await invokeIpc<GatewayModel[]>('refresh_gateway_models');
}

export async function startGateway(configPath?: string): Promise<void> {
    await invokeIpc<Record<string, never>>('start_gateway', { config_path: configPath ?? null });
}

export async function stopGateway(): Promise<void> {
    await invokeIpc<Record<string, never>>('stop_gateway');
}
