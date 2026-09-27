export type TunnelProtocol = "socks5" | "socks5h" | "http" | "https" | "direct";
export type TunnelStatus = "online" | "offline" | "unknown";

export interface OutboundTunnel {
  id: string;
  name: string;
  protocol: TunnelProtocol;
  endpoint: string;
  auth_user?: string;
  auth_pass?: string;
  enabled: boolean;
  status?: TunnelStatus;
  max_concurrent_streams?: number;
  min_request_interval_ms?: number;
  start_command?: string;
  stop_command?: string;
  last_checked_at?: string;
  last_error?: string;
  last_exit_ip?: string;
  last_latency_ms?: number;
  tags: string[];
  adguard_location?: string;
}

export interface AdguardLocationItem {
  id: string;
  name: string;
  ping_ms?: number;
}

export interface TunnelTestResult {
  tunnel_id: string;
  success: boolean;
  exit_ip?: string;
  latency_ms?: number;
  error?: string;
}

export interface TunnelEvent {
  ts: string;
  kind: string; // "info" | "ok" | "error"
  msg: string;
}

export interface EndpointKeyManager {
  key_file_path?: string;
  failed_key_file_path?: string;
  current_key_index: number;
  total_keys: number;
  current_key_preview?: string;
  last_switched_at?: string;
}

export type ProtocolAdapter = "none" | "openai_to_1min" | "openai_to_anthropic";

export interface RouteRule {
  id: string;
  name: string;
  port: number;
  path_prefix: string;
  target_base_url: string;
  tunnel_id: string;
  enabled: boolean;
  status?: "Active" | "Inactive" | "Unknown";
  last_error?: string;
  min_request_interval_ms?: number;
  key_manager: EndpointKeyManager;
  custom_auth_token?: string;
  /** Per-endpoint fingerprint (index vào fingerprint_pool). null/undefined = global default. */
  fingerprint_index?: number | null;
  /** Protocol Adapter chuyển đổi format giữa các AI API providers */
  protocol_adapter?: ProtocolAdapter;
}

export interface FingerprintProfile {
  mode: string;
  custom_user_agent?: string;
  strip_sdk_headers: boolean;
  strip_ide_headers: boolean;
  strip_sec_ch_ua: boolean;
  remove_empty_headers: boolean;
  mask_local_paths_in_body: boolean;
  spoof_headers?: Record<string, string>;
}

export interface GatewayConfig {
  config_version?: number;
  tunnels: OutboundTunnel[];
  routes: RouteRule[];
  fingerprint_profile: FingerprintProfile;
  fingerprint_pool?: FingerprintProfile[];
  active_fingerprint_index?: number;
  max_log_entries: number;
  max_disk_log_entries?: number;
  max_key_failures?: number;
}

export interface LeakFinding {
  category: string;
  field: string;
  value: string;
  severity: "low" | "medium" | "high";
  description: string;
}

// 100% Full Raw Wire Traffic Log
export interface RawTrafficLog {
  id: string;
  timestamp: string;
  route_id: string;
  method: string;
  port: number;
  path: string;
  target_url: string;
  tunnel_id: string;
  key_used_preview?: string;
  status_code: number;
  duration_ms: number;
  is_streaming: boolean;
  raw_request_headers: [string, string][];
  raw_forwarded_headers: [string, string][];
  raw_request_body: string;
  raw_response_headers: [string, string][];
  raw_response_body: string;
  raw_forwarded_body: string;
  leaked_findings: LeakFinding[];
}
