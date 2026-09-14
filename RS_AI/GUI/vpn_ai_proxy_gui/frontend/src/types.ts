export type TunnelProtocol = "socks5" | "http" | "direct";

export interface OutboundTunnel {
  id: string;
  name: string;
  protocol: TunnelProtocol;
  endpoint: string; // e.g. "127.0.0.1:1080"
  enabled: boolean;
  last_exit_ip?: string;
  last_latency_ms?: number;
  tags: string[];
}

export interface TunnelTestResult {
  tunnel_id: string;
  success: boolean;
  exit_ip?: string;
  latency_ms?: number;
  error?: string;
}

export interface RouteRule {
  id: string;
  name: string;
  port: number; // Distinct local port (e.g. 3000, 3001, 3002...)
  path_prefix: string; // e.g. "/v1", "/mirror", "/"
  target_base_url: string; // "https://abc.xyz/v1", "https://api.openai.com/v1"
  tunnel_id: string; // Foreign key to OutboundTunnel
  enabled: boolean;
  strip_prefix: boolean;
  custom_auth_token?: string;
}

export interface FingerprintProfile {
  mode: string;
  custom_user_agent?: string;
  strip_sdk_headers: boolean;
  strip_ide_headers: boolean;
  strip_sec_ch_ua: boolean;
  remove_empty_headers: boolean;
  mask_local_paths_in_body: boolean;
}

export interface GatewayConfig {
  tunnels: OutboundTunnel[];
  routes: RouteRule[];
  fingerprint_profile: FingerprintProfile;
  max_log_entries: number;
}

export interface LeakFinding {
  category: string;
  field: string;
  value: string;
  severity: "low" | "medium" | "high";
  description: string;
}

export interface RequestLog {
  id: string;
  timestamp: string;
  method: string;
  path: string;
  target_url: string;
  status_code: number;
  duration_ms: number;
  leaked_findings: LeakFinding[];
  client_headers: [string, string][];
  forwarded_headers: [string, string][];
  prompt_preview?: string;
  response_preview?: string;
  is_streaming: boolean;
  bytes_sent: number;
  bytes_received: number;
}
