import { invoke } from "@tauri-apps/api/core";
import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type { IpcError, IpcErrorCode, JobCommand, JobEvent, JobRequest, JobResponse, PickerKind, PickerSelectRequest, PickerSelectResult, PickerSelection, Res } from "./contract";
import { jobTopics, request } from "./contract";

export interface BridgeAdapter {
  invoke<T>(command: string, args: { request: unknown }): Promise<T>;
  listen<T>(topic: string, handler: (event: Event<T>) => void): Promise<UnlistenFn>;
}

const tauriBridge: BridgeAdapter = { invoke, listen };

const IPC_ERROR_CODES: ReadonlySet<string> = new Set<IpcErrorCode>([
  "invalid_argument", "not_found", "conflict", "unauthorized", "forbidden",
  "unavailable", "io", "validation", "cancelled", "internal",
]);

function ipcErrorCode(value: unknown): IpcErrorCode {
  return typeof value === "string" && IPC_ERROR_CODES.has(value) ? value as IpcErrorCode : "internal";
}

export function ipcError(error: unknown): IpcError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const candidate = error as { code?: unknown; message: unknown; retryable?: unknown; details?: unknown };
    return {
      code: ipcErrorCode(candidate.code),
      message: String(candidate.message),
      retryable: candidate.retryable === true,
      details: isJsonValue(candidate.details) ? candidate.details : null,
    };
  }
  return { code: "internal", message: String(error), retryable: false, details: null };
}

function isJsonValue(value: unknown): value is IpcError["details"] {
  if (value === null || ["boolean", "number", "string"].includes(typeof value)) return true;
  if (Array.isArray(value)) return value.every(isJsonValue);
  return typeof value === "object" && value !== null && Object.values(value).every(isJsonValue);
}

export class JobClient {
  private active: { requestId: string; release: () => void } | null = null;

  constructor(private readonly bridge: BridgeAdapter = tauriBridge) {}

  async run<K extends JobCommand>(
    command: K,
    payload: JobRequest<K>,
    onEvent: (event: JobEvent<JobResponse<K>>) => void,
  ): Promise<Res<JobResponse<K>>> {
    if (this.active) throw new Error("another job is active");
    const requestId = crypto.randomUUID();
    let unlisten: UnlistenFn | null = null;
    let released = false;
    const release = (): void => {
      if (released) return;
      released = true;
      const callback = unlisten;
      unlisten = null;
      callback?.();
      if (this.active?.requestId === requestId) this.active = null;
    };
    this.active = { requestId, release };
    try {
      unlisten = await this.bridge.listen<JobEvent<JobResponse<K>>>(jobTopics[command], ({ payload: event }) => {
        if (event.job_id !== requestId || event.payload.request_id !== requestId) return;
        onEvent(event);
        if (["completed", "failed", "cancelled"].includes(event.state)) release();
      });
      if (released) unlisten();
      return await this.bridge.invoke<Res<JobResponse<K>>>(command, { request: request(payload, requestId) });
    } catch (error) {
      release();
      throw ipcError(error);
    }
  }

  dispose(): void {
    this.active?.release();
  }
}

export async function call<T>(command: string, payload: unknown, bridge: BridgeAdapter = tauriBridge): Promise<T> {
  const response = await bridge.invoke<Res<T>>(command, { request: request(payload, null) });
  return response.data;
}

export async function pickerSelect(
  kind: PickerKind,
  selection: PickerSelection,
  bridge: BridgeAdapter = tauriBridge,
): Promise<PickerSelectResult> {
  const payload: PickerSelectRequest = { kind, selection };
  return call<PickerSelectResult>("picker_select", payload, bridge);
}
