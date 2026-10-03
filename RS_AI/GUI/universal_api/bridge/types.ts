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
    plan_tier?: string | null;
    is_current?: boolean;
    quota_details?: any;
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
    // 1. Appearance & UI
    language: string;
    theme: string;
    font: string;
    theme_color?: string;
    ui_scale?: number;
    close_behavior?: string;
    startup_minimized?: boolean;
    app_auto_launch_enabled?: boolean;
    reduced_motion_enabled?: boolean;
    default_terminal?: string;
    side_nav_layout_mode?: 'classic' | 'original' | string;
    remember_main_window_state?: boolean;
    floating_card_show_on_startup?: boolean;
    floating_card_always_on_top?: boolean;
    show_top_promo?: boolean;
    startup_page?: string;
    color_pack?: string;
    allow_external_network?: boolean;
    webdav_allowed_domains?: string;

    // 2. Session Keeper & Background Refresh
    token_keeper_enabled?: boolean;
    auto_refresh_minutes?: number;
    auto_import_from_local_enabled?: boolean;
    share_sessions_on_switch?: boolean;

    // 3. Network & Proxy
    global_proxy_enabled?: boolean;
    global_proxy_url?: string;
    global_proxy_no_proxy?: string;
    ws_port?: number;

    // 4. IDE Paths & Automation
    vscode_app_path?: string;
    antigravity_app_path?: string;
    antigravity_desktop_app_path?: string;
    cursor_app_path?: string;
    trae_app_path?: string;
    zed_app_path?: string;
    codebuddy_app_path?: string;
    codebuddy_cn_app_path?: string;
    launch_on_switch?: boolean;

    // 5. Quota Alerts & Auto Switch
    quota_alert_enabled?: boolean;
    quota_alert_threshold?: number;
    auto_switch_enabled?: boolean;
    auto_switch_threshold?: number;

    // 6. Backup & Cloud Sync (WebDAV)
    auto_backup_enabled?: boolean;
    auto_backup_retention_days?: number;
    webdav_sync_enabled?: boolean;
    webdav_sync_url?: string;
    webdav_sync_username?: string;
    webdav_sync_password?: string;
    webdav_sync_remote_dir?: string;
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

/// Một provider ngoài: Account (trạng thái/models/login) + Configuration
/// (endpoint/key/timeouts/ports/prefixes). KHÔNG scheduler, KHÔNG quick actions
/// (2 thứ đó là đặc thù CodeBuddy: tasks bảo trì account + chạy tay).
export interface ProviderEntry {
    name: string;
    kind: string;
    model_prefixes: string;
    config: ExternalConfig;
}

export interface ProviderStatus {
    name: string;
    enabled: boolean;
    reachable: boolean;
    http_status: number | null;
    latency_ms: number | null;
    error: string | null;
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
    keepalive_hours: number[];
    keepalive_enabled: boolean;
}
