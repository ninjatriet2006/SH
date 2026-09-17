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
