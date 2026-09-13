import { invoke } from '@tauri-apps/api/core';

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type Req<T> = { schema_version: 1; request_id: string | null; payload: T };
export type Res<T> = { schema_version: 1; request_id: string | null; data: T };
export type IpcErrorCode = 'invalid_argument' | 'not_found' | 'conflict' | 'unauthorized' |
    'forbidden' | 'unavailable' | 'io' | 'validation' | 'cancelled' | 'internal';
export type IpcError = { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null };

export async function invokeIpc<T, P extends object = Record<string, never>>(
    command: string,
    payload: P = {} as P,
): Promise<T> {
    const request: Req<P> = { schema_version: 1, request_id: null, payload };
    const response = await invoke<Res<T>>(command, { request });
    if (response.schema_version !== 1 || response.request_id !== null || !Object.prototype.hasOwnProperty.call(response, 'data')) {
        throw { code: 'internal', message: 'Phản hồi IPC không đúng envelope A.1.', retryable: false, details: null } satisfies IpcError;
    }
    return response.data;
}

export function ipcErrorMessage(error: unknown): string {
    if (typeof error === 'object' && error !== null && 'message' in error) {
        const message = (error as { message?: unknown }).message;
        if (typeof message === 'string') return message;
    }
    return String(error);
}
