import { invoke } from "@tauri-apps/api/core";
import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  IpcError, IpcErrorCode, JobCommand, JobEvent, JobRequest, JobResponse,
  ManagerConfig, PickerKind, PickerSelectResult, Preferences, Res,
} from "./contract";
import { jobTopics, request } from "./contract";

// --- Error normalization ---

const IPC_ERROR_CODES = new Set<IpcErrorCode>([
  "invalid_argument", "not_found", "conflict", "unauthorized", "forbidden", "unavailable",
  "io", "validation", "cancelled", "internal",
]);

export function ipcError(error: unknown): IpcError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const value = error as { code?: unknown; message: unknown; retryable?: unknown; details?: unknown };
    return {
      code: typeof value.code === "string" && IPC_ERROR_CODES.has(value.code as IpcErrorCode)
        ? value.code as IpcErrorCode : "internal",
      message: String(value.message), retryable: value.retryable === true,
      details: isJson(value.details) ? value.details : null,
    };
  }
  return { code: "internal", message: String(error), retryable: false, details: null };
}

function isJson(value: unknown): value is IpcError["details"] {
  if (value === null || ["boolean", "number", "string"].includes(typeof value)) return true;
  if (Array.isArray(value)) return value.every(isJson);
  return typeof value === "object" && value !== null && Object.values(value).every(isJson);
}

// --- Job streaming client ---

export class JobClient {
  private active: { requestId: string; release: () => void } | null = null;

  async run<K extends JobCommand>(
    command: K, payload: JobRequest<K>, onEvent: (event: JobEvent<JobResponse<K>>) => void,
  ): Promise<Res<JobResponse<K>>> {
    if (this.active) throw new Error("another job is active");
    const requestId = crypto.randomUUID();
    let unlisten: UnlistenFn | null = null;
    let released = false;
    let terminalSeen = false;
    let lastSeq = -1;
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
      unlisten = await listen<JobEvent<JobResponse<K>>>(jobTopics[command], ({ payload: event }: Event<JobEvent<JobResponse<K>>>) => {
        if (terminalSeen || event.job_id !== requestId || event.payload.request_id !== requestId || event.seq <= lastSeq) return;
        lastSeq = event.seq;
        terminalSeen = ["completed", "failed", "cancelled"].includes(event.state);
        onEvent(event);
        if (terminalSeen) release();
      });
      if (released) unlisten();
      return await invoke<Res<JobResponse<K>>>(command, { request: request(payload, requestId) });
    } catch (error) {
      release();
      throw ipcError(error);
    }
  }

  dispose(): void { this.active?.release(); }
}

// --- Simple request/response calls ---

async function call<T>(command: string, payload: unknown): Promise<T> {
  try {
    return (await invoke<Res<T>>(command, { request: request(payload, null) })).data;
  } catch (error) {
    throw ipcError(error);
  }
}

export const api = {
  loadConfig: () => call<ManagerConfig>("config_load", {}),
  saveConfig: (config: ManagerConfig) => call<ManagerConfig>("config_save", config),
  getPreferences: () => call<Preferences>("preferences_get", {}),
  setPreferences: (preferences: Preferences) => call<Preferences>("preferences_set", preferences),
  pickerSelect: (kind: PickerKind) => call<PickerSelectResult>("picker_select", { kind }),
};
