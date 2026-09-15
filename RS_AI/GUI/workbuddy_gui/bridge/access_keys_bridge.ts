import { invokeIpc } from './ipc';
import type { AccessKey, CreatedAccessKey } from './types';

export async function listAccessKeys(): Promise<AccessKey[]> {
    return invokeIpc<AccessKey[]>('list_access_keys');
}

export async function createAccessKey(label: string): Promise<CreatedAccessKey> {
    return invokeIpc<CreatedAccessKey>('create_access_key', { label });
}

export async function revokeAccessKey(id: number): Promise<void> {
    await invokeIpc<Record<string, never>>('revoke_access_key', { id });
}
