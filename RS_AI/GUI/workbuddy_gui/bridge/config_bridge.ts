import { invokeIpc } from './ipc';

export async function getConfig(): Promise<unknown> {
    return invokeIpc<unknown>('get_config');
}

export async function getDefaultConfig(): Promise<unknown> {
    return invokeIpc<unknown>('get_default_config');
}

export async function saveConfig(configJson: string): Promise<void> {
    await invokeIpc<Record<string, never>>('save_config', { config_json: configJson });
}
