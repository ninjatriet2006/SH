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

export interface AccountInfo {
    uid: string;
    nickname: string;
    domain: string;
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
