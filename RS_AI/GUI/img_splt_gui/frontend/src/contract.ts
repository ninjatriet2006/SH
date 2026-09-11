export const SCHEMA_VERSION = 1 as const;

export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
export type IpcErrorCode = "invalid_argument" | "not_found" | "conflict" | "unauthorized" |
  "forbidden" | "unavailable" | "io" | "validation" | "cancelled" | "internal";
export interface Req<T> { schema_version: 1; request_id: string | null; payload: T }
export interface Res<T> { schema_version: 1; request_id: string | null; data: T }
export interface IpcError { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null }
export interface PathRef { path: string }
export type DistributionMode = "balanced" | "greedy" | "fixed";
export interface ImageSettings {
  default_distribution_mode: DistributionMode;
  max_files_per_folder: number;
  fixed_folder_count: number;
  max_retries: number;
  min_upscale_width: number;
  target_upscale_width: number;
}
export interface ToolStatus { available: boolean; version: string | null }
export interface CapabilityReport { ffmpeg: ToolStatus; ffprobe: ToolStatus }
export interface ImageScanReport { directory: PathRef; images: PathRef[]; total: number }
export interface ProcessReport { output_directory: PathRef; processed: PathRef[]; failed: PathRef[] }
export interface DistributionReport { output_directory: PathRef; folders: PathRef[]; distributed: number }
export type Language = "en" | "vi";
export type Theme = "system" | "light" | "dark";
export type FontId = "system-default" | "dejavusans";
export interface Preferences { language: Language; theme: Theme; font_id: FontId }
export type PickerKind = "input" | "output";
export interface PickerSelectResult { kind: PickerKind; path: PathRef }
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
export interface ScanImagesRequest { directory: PathRef }
export interface ProcessImagesRequest {
  input_directory: PathRef;
  files: PathRef[];
  output_directory: PathRef;
  output_format: string | null;
  upscale: boolean;
  settings: ImageSettings;
}
export interface DistributeRequest {
  input_directory: PathRef;
  files: PathRef[];
  output_directory: PathRef;
  chapter: number | null;
  mode: DistributionMode;
  max_files_per_folder: number;
  fixed_folder_count: number;
}
export interface JobCommandMap {
  capabilities_check: { request: EmptyRequest; response: CapabilityReport; topic: "job.capabilities_check" };
  scan_images: { request: ScanImagesRequest; response: ImageScanReport; topic: "job.scan_images" };
  process_images: { request: ProcessImagesRequest; response: ProcessReport; topic: "job.process_images" };
  distribute: { request: DistributeRequest; response: DistributionReport; topic: "job.distribute" };
}
export type JobCommand = keyof JobCommandMap;
export type JobRequest<K extends JobCommand> = JobCommandMap[K]["request"];
export type JobResponse<K extends JobCommand> = JobCommandMap[K]["response"];
export type JobTopic<K extends JobCommand> = JobCommandMap[K]["topic"];

export const jobTopics = {
  capabilities_check: "job.capabilities_check",
  scan_images: "job.scan_images",
  process_images: "job.process_images",
  distribute: "job.distribute",
} as const satisfies { [K in JobCommand]: JobTopic<K> };

export const invokeRegistry = [
  "settings_load", "settings_save", "capabilities_check", "scan_images", "process_images",
  "distribute", "preferences_get", "preferences_set", "picker_select",
] as const;

export function request<T>(payload: T, requestId: string | null): Req<T> {
  return { schema_version: SCHEMA_VERSION, request_id: requestId, payload };
}
