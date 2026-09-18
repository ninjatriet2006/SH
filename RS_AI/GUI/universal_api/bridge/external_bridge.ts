import { invokeIpc } from './ipc';
import type { ExternalConfig, ExternalModel, ExternalStatus, PortCheck } from './types';

export async function getExternalConfig(): Promise<ExternalConfig> {
    return await invokeIpc<ExternalConfig>('get_external_config');
}

export async function saveExternalConfig(config: ExternalConfig): Promise<void> {
    await invokeIpc<Record<string, never>>('save_external_config', { config });
}

export async function getExternalStatus(): Promise<ExternalStatus> {
    return await invokeIpc<ExternalStatus>('get_external_status');
}

export async function refreshExternalModels(): Promise<ExternalModel[]> {
    return await invokeIpc<ExternalModel[]>('refresh_external_models');
}

export async function checkPortAvailable(port: number): Promise<PortCheck> {
    return await invokeIpc<PortCheck>('check_port_available', { port });
}

export async function suggestFreePort(preferred: number): Promise<{ port: number }> {
    return await invokeIpc<{ port: number }>('suggest_free_port', { preferred });
}
