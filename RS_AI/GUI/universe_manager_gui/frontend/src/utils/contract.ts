export const SCHEMA_VERSION = 1 as const;

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export type IpcErrorCode =
  | "invalid_argument" | "not_found" | "conflict" | "unauthorized"
  | "forbidden" | "unavailable" | "io" | "validation" | "cancelled" | "internal";

export interface Req<T> { schema_version: 1; request_id: string | null; payload: T }
export interface Res<T> { schema_version: 1; request_id: string | null; data: T }
export interface IpcError { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null }
export interface PathRef { path: string }

// --- Domain types ---

export type InstallType = "InPlace" | "Moved";
export interface AppEntry {
  id: string; name: string; install_type: InstallType; source_path: string | null;
  install_path: string; exec_path: string; icon_path: string | null; desktop_file: string;
  symlink_file: string | null; added_at: string; is_custom: boolean | null;
  start_cmd: string | null; stop_cmd: string | null; category: string | null;
  package_type: string | null; inventory_sources: string[]; registry_key: string | null;
  product_code: string | null; about_url: string | null; publisher: string | null;
  version: string | null; uninstall_cmd: string | null;
  status?: "Running" | "Stopped" | string | null;
}

export interface ManagerConfig { settings: { managed_dir: string }; apps: AppEntry[] }

export interface DetectionReport {
  is_appimage: boolean; suggested_name: string; executables: PathRef[];
  icons: PathRef[]; desktop_templates: PathRef[];
}

export interface OperationResult { app_id: string; operation: "start" | "stop"; completed: boolean }

export interface SearchResult { name: string; id: string; version: string; source: string }
export interface SearchReport { query: string; results: SearchResult[] }

export type Language = "vi" | "en";
export type Theme = "system" | "light" | "dark";
export type FontId = "system-default" | "dejavusans";
export interface Preferences { language: Language; theme: Theme; font_id: FontId }

export type PickerKind = "managed" | "source";
export interface PickerSelectResult { kind: PickerKind; path: PathRef }

// --- Job streaming ---

export type JobState = "started" | "progress" | "completed" | "failed" | "cancelled";

export type JobResult<T> =
  | { status: "completed"; request_id: string; value: T }
  | { status: "failed"; request_id: string; error: IpcError }
  | { status: "cancelled"; request_id: string; error: IpcError };

export interface JobEvent<T> {
  schema_version: 1; job_id: string; seq: number; state: JobState;
  payload: {
    request_id: string; completed: number | null; total: number | null;
    message: string | null; result: JobResult<T> | null;
  };
}

// --- Command map (type-safe invoke) ---

export type EmptyRequest = Record<string, never>;

export interface JobCommandMap {
  scan_apps: { request: EmptyRequest; response: AppEntry[]; topic: "job:scan_apps" };
  detect_app: { request: { path: PathRef }; response: DetectionReport; topic: "job:detect_app" };
  start_app: { request: { app_id: string; confirmed: boolean }; response: OperationResult; topic: "job:start_app" };
  stop_app: { request: { app_id: string; confirmed: boolean }; response: OperationResult; topic: "job:stop_app" };
  search_apps: { request: { query: string }; response: SearchReport; topic: "job:search_apps" };
}

export type JobCommand = keyof JobCommandMap;
export type JobRequest<K extends JobCommand> = JobCommandMap[K]["request"];
export type JobResponse<K extends JobCommand> = JobCommandMap[K]["response"];
export type JobTopic<K extends JobCommand> = JobCommandMap[K]["topic"];

export const jobTopics = {
  scan_apps: "job:scan_apps", detect_app: "job:detect_app", start_app: "job:start_app",
  stop_app: "job:stop_app", search_apps: "job:search_apps",
} as const satisfies { [K in JobCommand]: JobTopic<K> };

export function request<T>(payload: T, requestId: string | null): Req<T> {
  return { schema_version: SCHEMA_VERSION, request_id: requestId, payload };
}
