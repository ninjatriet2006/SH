import { invoke as tauriInvoke } from '@tauri-apps/api/core';

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type IpcErrorCode = 'invalid_argument' | 'not_found' | 'conflict' | 'unauthorized' |
  'forbidden' | 'unavailable' | 'io' | 'validation' | 'cancelled' | 'internal';
export interface IpcError { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null }
export interface Req<T> { schema_version: 1; request_id: string | null; payload: T }
export interface Res<T> { schema_version: 1; request_id: string | null; data: T }

export interface BridgeAdapter {
  invoke<T>(command: string, args: { request: Req<unknown>; [key: string]: unknown }): Promise<T>;
}

const bridge: BridgeAdapter = { invoke: tauriInvoke };
const codes = new Set<IpcErrorCode>([
  'invalid_argument', 'not_found', 'conflict', 'unauthorized', 'forbidden',
  'unavailable', 'io', 'validation', 'cancelled', 'internal',
]);

export function request<T>(payload: T): Req<T> {
  return { schema_version: 1, request_id: null, payload };
}

export function ipcError(value: unknown): IpcError {
  if (typeof value === 'object' && value !== null && 'message' in value) {
    const error = value as { code?: unknown; message: unknown; retryable?: unknown; details?: unknown };
    return {
      code: typeof error.code === 'string' && codes.has(error.code as IpcErrorCode) ? error.code as IpcErrorCode : 'internal',
      message: String(error.message),
      retryable: error.retryable === true,
      details: isJson(error.details) ? error.details : null,
    };
  }
  return { code: 'internal', message: String(value), retryable: false, details: null };
}

function isJson(value: unknown): value is JsonValue {
  if (value === null || ['boolean', 'number', 'string'].includes(typeof value)) return true;
  if (Array.isArray(value)) return value.every(isJson);
  return typeof value === 'object' && value !== null && Object.values(value).every(isJson);
}

/** A.1 facade: wraps legacy payloads and unwraps Res.data for call-site compatibility. */
export async function invoke<T>(
  command: string,
  payload: Record<string, unknown> = {},
  adapter: BridgeAdapter = bridge,
  transport: Record<string, unknown> = {},
): Promise<T> {
  try {
    if (Object.values(payload).some(value => value === undefined)) {
      throw { code: 'invalid_argument', message: 'IPC payload cannot contain undefined', retryable: false, details: null };
    }
    const response = await adapter.invoke<Res<T>>(command, { request: request(payload), ...transport });
    if (response.schema_version !== 1 || response.request_id !== null || !Object.prototype.hasOwnProperty.call(response, 'data')) {
      throw { code: 'internal', message: 'invalid IPC response envelope', retryable: false, details: null };
    }
    return response.data;
  } catch (error) {
    throw ipcError(error);
  }
}
