import { invoke as tauriInvoke } from '@tauri-apps/api/core';

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type Req<T> = { schema_version: 1; request_id: string | null; payload: T };
export type Res<T> = { schema_version: 1; request_id: string | null; data: T };
export type IpcErrorCode = 'invalid_argument' | 'not_found' | 'conflict' | 'unauthorized' |
  'forbidden' | 'unavailable' | 'io' | 'validation' | 'cancelled' | 'internal';
export type IpcError = { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null };

const camelToSnake = (key: string) => key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);

function payloadFor(command: string, args: Record<string, unknown>): Record<string, unknown> {
  const payload = Object.fromEntries(
    Object.entries(args).map(([key, value]) => [camelToSnake(key), value === undefined ? null : value]),
  );
  const nullable: Record<string, string[]> = {
    list_files: ['pane'],
    fs_copy: ['task_id'],
    fs_move: ['task_id'],
    fs_read_text: ['max_bytes'],
    sys_open_with: ['exec_cmd', 'app'],
  };
  for (const field of nullable[command] ?? []) {
    if (!(field in payload)) payload[field] = null;
  }
  return payload;
}

export function unwrap<T>(response: Res<T>): T {
  if (!response || response.schema_version !== 1 || !('data' in response)) {
    throw new Error('Malformed IPC response envelope');
  }
  return response.data;
}

export async function send<T>(command: string, payload: Record<string, unknown> = {}): Promise<T> {
  const request: Req<Record<string, unknown>> = {
    schema_version: 1,
    request_id: null,
    payload: payloadFor(command, payload),
  };
  return unwrap(await tauriInvoke<Res<T>>(command, { request }));
}

/** Drop-in compatibility for existing callers while enforcing the A.1 boundary. */
export const invoke = send;
