import { invokeIpc } from './ipc';

export interface ZedAccount {
    id: string;
    label: string;
    org_id: string;
    enabled: boolean;
}

export interface ZedImportResult {
    id: string;
    label: string;
    org_id: string | null;
    plan: string;
    period: string;
}

export interface ZedTestResult {
    id: string;
    ok: boolean;
    login: string;
    plan: string;
    quota: string;
    message: string;
}

export interface ZedModel {
    id: string;
    display_name?: string | null;
    provider: string;
    supports_tools: boolean;
    max_output_tokens?: number | null;
}

export async function listZedAccounts(): Promise<ZedAccount[]> {
    return await invokeIpc<ZedAccount[]>('list_zed_accounts');
}

export async function importZedAccount(filePath: string): Promise<ZedImportResult> {
    return await invokeIpc<ZedImportResult>('import_zed_account', { file_path: filePath });
}

export async function setZedEnabled(id: string, enabled: boolean): Promise<void> {
    await invokeIpc<Record<string, never>>('set_zed_enabled', { id, enabled });
}

export async function removeZedAccount(id: string): Promise<void> {
    await invokeIpc<Record<string, never>>('remove_zed_account', { id });
}

export async function testZedAccount(id: string): Promise<ZedTestResult> {
    return await invokeIpc<ZedTestResult>('test_zed_account', { id });
}

export async function refreshZedModels(): Promise<ZedModel[]> {
    return await invokeIpc<ZedModel[]>('refresh_zed_models');
}

export interface ZedConfig {
    enabled: boolean;
    system_id: string;
}

export async function getZedConfig(): Promise<ZedConfig> {
    return await invokeIpc<ZedConfig>('get_zed_config');
}

export async function saveZedConfig(cfg: ZedConfig): Promise<void> {
    await invokeIpc<Record<string, never>>('save_zed_config', cfg);
}
