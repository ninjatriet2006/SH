export const SCHEMA_VERSION = 1 as const;

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type IpcErrorCode =
  | "invalid_argument"
  | "not_found"
  | "conflict"
  | "unauthorized"
  | "forbidden"
  | "unavailable"
  | "io"
  | "validation"
  | "cancelled"
  | "internal";
export interface Req<T> { schema_version: 1; request_id: string | null; payload: T }
export interface Res<T> { schema_version: 1; request_id: string | null; data: T }
export interface IpcError { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null }
export interface PathRef { path: string }
export type FileType = "video" | "audio" | "image" | "document" | "archive" | "directory" | "unknown";
export interface DependencyReport { is_ok: boolean; missing: string[] }
export interface Classification { path: string; file_type: FileType; size_bytes: number }
export interface ScanReport { directory: string; files: Classification[]; total: number }
export interface BatchConvertReport { output_directory: PathRef; converted: PathRef[]; failed: PathRef[] }
export type NativeOperation = "install" | "uninstall";
export interface NativeOperationResult { operation: NativeOperation; installation_id: string; completed: boolean }
export type Language = "vi" | "en";
export type Theme = "system" | "light" | "dark";
export type FontId = "system-default" | "dejavusans";
export interface Preferences { language: Language; theme: Theme; font_id: FontId }
export type PickerKind = "input" | "output" | "artifact";
export type PickerSelection = "file" | "files" | "directory";
export interface PickerSelectRequest { kind: PickerKind; selection: PickerSelection }
export interface PickerSelectResult { kind: PickerKind; paths: PathRef[] }
export type JobState = "started" | "progress" | "completed" | "failed" | "cancelled";
export type JobResult<T> =
  | { status: "completed"; request_id: string; value: T }
  | { status: "failed"; request_id: string; error: IpcError }
  | { status: "cancelled"; request_id: string; error: IpcError };
export interface JobEvent<T> {
  schema_version: 1;
  job_id: string;
  seq: number;
  state: JobState;
  payload: {
    request_id: string;
    completed: number | null;
    total: number | null;
    message: string | null;
    result: JobResult<T> | null;
  };
}

export type EmptyRequest = Record<string, never>;
export interface ClassifyFileRequest { path: string }
export interface ScanDirectoryRequest { directory: string; allowed_types: FileType[] }
export interface BatchConvertRequest {
  files: PathRef[];
  output_directory: PathRef;
  output_format: string;
  overwrite: boolean;
}
export interface NativeInstallRequest { artifact: PathRef; target_dir: PathRef; confirmed: boolean }
export interface NativeUninstallRequest { installation_id: string; confirmed: boolean }

export interface JobCommandMap {
  dependencies_check: { request: EmptyRequest; response: DependencyReport; topic: "job.dependencies_check" };
  classify_file: { request: ClassifyFileRequest; response: Classification; topic: "job.classify_file" };
  scan_directory: { request: ScanDirectoryRequest; response: ScanReport; topic: "job.scan_directory" };
  batch_convert: { request: BatchConvertRequest; response: BatchConvertReport; topic: "job.batch_convert" };
  native_install: { request: NativeInstallRequest; response: NativeOperationResult; topic: "job.native_install" };
  native_uninstall: { request: NativeUninstallRequest; response: NativeOperationResult; topic: "job.native_uninstall" };
}

export type JobCommand = keyof JobCommandMap;
export type JobRequest<K extends JobCommand> = JobCommandMap[K]["request"];
export type JobResponse<K extends JobCommand> = JobCommandMap[K]["response"];
export type JobTopic<K extends JobCommand> = JobCommandMap[K]["topic"];

export const jobTopics = {
  dependencies_check: "job.dependencies_check",
  classify_file: "job.classify_file",
  scan_directory: "job.scan_directory",
  batch_convert: "job.batch_convert",
  native_install: "job.native_install",
  native_uninstall: "job.native_uninstall",
} as const satisfies { [K in JobCommand]: JobTopic<K> };

export const invokeRegistry = [
  "dependencies_check",
  "classify_file",
  "scan_directory",
  "batch_convert",
  "native_install",
  "native_uninstall",
  "preferences_get",
  "preferences_set",
  "picker_select",
] as const;

export function request<T>(payload: T, requestId: string | null): Req<T> {
  return { schema_version: SCHEMA_VERSION, request_id: requestId, payload };
}
