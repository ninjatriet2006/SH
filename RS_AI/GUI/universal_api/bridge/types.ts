export interface FontInfo {
    id: string;
    name: string;
    family: string;
    src_path: string | null;
}

export interface GatewayInfo {
    status: 'stopped' | 'starting' | 'running' | 'stopping' | 'error';
    listen: string | null;
    total_accounts: number;
    healthy_accounts: number;
    error: string | null;
    requests_total: number;
    requests_failed: number;
    tokens_total: number;
}

export interface ServingInfo {
    serving_uid: string | null;
    sticky_sessions: number;
    gateway_running: boolean;
}

export interface FingerprintConfig {
    user_agent?: string | null;
    headers?: Record<string, string>;
}

export interface AccountInfo {
    uid: string;
    nickname: string;
    domain: string;
    proxy_url?: string | null;
    fingerprint_profile?: FingerprintConfig | null;
    credits: number;
    healthy: boolean;
    cooling: boolean;
    cool_kind: string | null;
    cool_remaining_sec: number | null;
    disabled: boolean;
    disabled_reason: string | null;
    success_count: number;
    err_total: number;
    in_flight: number;
}

export interface TrafficAuditLog {
    id: string;
    timestamp: string;
    route: string;
    model: string;
    status_code: number;
    duration_ms: number;
    account_uid: string;
    proxy_used?: string | null;
    raw_request_headers: [string, string][];
    raw_forwarded_headers: [string, string][];
    raw_request_body: string;
    raw_response_preview: string;
    raw_response_headers: [string, string][];
    raw_forwarded_body: string;
    is_streaming: boolean;
}

export interface TaskResult {
    task: string;
    success: boolean;
    message: string;
}

export interface GuiSettings {
    language: string;
    theme: string;
    font: string;
}

export interface ThemeInfo {
    id: string;
    name: string;
    type: string;
    colors: Record<string, string>;
}

export interface AccessKey {
    id: number;
    label: string;
    prefix: string;
    created_at: number;
    last_used_at: number | null;
    revoked: boolean;
}

export interface CreatedAccessKey {
    key: AccessKey;
    plaintext: string;
}

export interface UsageLog {
    id: number;
    access_key_prefix: string;
    route: string;
    model: string;
    status: number;
    tokens: number;
    elapsed_ms: number;
    created_at: number;
}

export interface UsageSummary {
    access_key_prefix: string;
    requests: number;
    tokens: number;
    errors: number;
}

export interface TraceRecord {
    timestamp: number;
    route: string;
    model: string;
    status: number;
    tokens: number;
    elapsed_ms: number;
    detail?: {
        req_bytes?: number;
        req_preview?: string;
        resp_preview?: string;
        error?: string | null;
    } | null;
}

export interface ExternalConfig {
    enabled: boolean;
    base_url: string;
    api_key: string;
    timeout_seconds: number;
    chat_timeout_seconds: number;
    control_port: number;
    public_port: number;
    auto_fallback: boolean;
}

export interface ExternalStatus {
    reachable: boolean;
    http_status: number | null;
    latency_ms: number | null;
    error: string | null;
}

export interface ExternalModel {
    id: string;
    object: string;
    created: number;
    owned_by: string;
    external?: boolean;
}

export interface PortCheck {
    port: number;
    available: boolean;
    conflicts_gateway: boolean;
    gateway_listen: string;
}

export interface ScheduleConfig {
    checkin_hours: number[];
    travel_hours: number[];
    activity_hours: number[];
    keepalive_hours: number[];
    checkin_enabled: boolean;
    travel_enabled: boolean;
    activity_enabled: boolean;
    keepalive_enabled: boolean;
}
