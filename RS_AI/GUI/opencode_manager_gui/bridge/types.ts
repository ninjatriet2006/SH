/*
[INTEGRITY NOTES]
- Mục đích: Kiểu dữ liệu TypeScript đồng bộ với struct Rust của backend.
- Trách nhiệm: Giúp Frontend dùng đúng kiểu, bắt lỗi ở thời gian biên dịch.
- Tương tác: Được import bởi mọi bridge và component.
*/

/** Trạng thái kết nối của một provider. */
export type StatusKind = 'alive' | 'no_credits' | 'invalid_key' | 'offline';

export interface StatusView {
    provider_id: string;
    kind: StatusKind;
    message: string;
}

/** Provider hiển thị trong danh sách. `api_key_masked` KHÔNG phải key thật. */
export interface ProviderView {
    id: string;
    name: string;
    base_url: string;
    api_key_masked: string;
    has_api_key: boolean;
    npm: string | null;
    model_count: number;
    models: string[];
    /** Provider tích hợp (khoá nằm ở auth.json) hay tự thêm. */
    is_builtin: boolean;
}

/** Mẫu provider có sẵn để chọn khi thêm. */
export interface PresetView {
    id: string;
    name: string;
    base_url: string;
    id_prefix: string;
    npm: string | null;
}

export interface DuplicateInfo {
    id: string;
    name: string;
}

/**
 * Kết quả lưu provider. `saved_id === null` và `duplicate_of !== null` nghĩa là
 * CHƯA lưu — phải hỏi người dùng có gộp vào provider trùng hay không.
 */
export interface SaveResult {
    saved_id: string | null;
    duplicate_of: DuplicateInfo | null;
    normalized_base_url: string | null;
}

/** Một model sau khi quét từ provider. */
export interface ScannedModel {
    id: string;
    in_config: boolean;
    /** Còn trong config nhưng provider không còn hỗ trợ. */
    stale: boolean;
    /** Khả năng model đã lưu trong config (nếu có) — để UI sửa lại. */
    caps: ModelCapsView | null;
}

/** Dạng phẳng của capability model trong config (case `hy3`). */
export interface ModelCapsView {
    tool_call: boolean | null;
    reasoning: boolean | null;
    interleaved: string | null;
}

/**
 * Capability model do UI gửi khi lưu. Field `null`/thiếu = "không đổi",
 * `true/false` = ghi đè. `interleaved` rỗng = bỏ qua.
 */
export interface ModelCaps {
    tool_call?: boolean | null;
    reasoning?: boolean | null;
    interleaved?: string | null;
}

export interface BadProvider {
    id: string;
    name: string;
    kind: StatusKind;
    message: string;
    is_builtin: boolean;
}

export interface ConfigPaths {
    opencode_json: string;
    auth_json: string;
}

export interface GuiSettings {
    language: string;
    theme_id: string;
}

export interface Theme {
    id: string;
    name: string;
    type: 'dark' | 'light';
    colors: Record<string, string>;
}

/** Kết quả thêm nhanh nhiều provider (backend `api/bulk.rs`). */
export interface BulkAddResult {
    added: number;
    skipped_existing: number;
    skipped_duplicate_input: number;
    created_ids: string[];
    normalized_endpoint: string;
}

/** Provider CKey (endpoint LLM `https://api.xah.io/v1`). */
export interface CkeyProviderView {
    provider_id: string;
    name: string;
    has_account_key: boolean;
}

export interface CkeyAccountOption {
    provider_id: string;
    key_masked: string;
}

export interface CkeyProfileView {
    username: string;
    name: string;
    email: string;
    balance: string;
    balance_raw: number;
    created_at: string;
}

export interface CkeyStatsView {
    requests: number;
    success_requests: number;
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
    charged_vnd_text: string;
}

export interface CkeyKeyView {
    id: number;
    key_name: string;
    key_prefix: string;
    is_active: boolean;
    created_at_text: string;
    key_masked: string;
}

export interface CkeyModelView {
    public_name: string;
    display_name: string;
    input_price_per_million_vnd: number;
    output_price_per_million_vnd: number;
    context_tokens_limit: number;
    max_output_tokens_limit: number;
    cache_enabled: boolean;
}

export interface CkeyDashboard {
    profile: CkeyProfileView | null;
    stats: CkeyStatsView | null;
    keys: CkeyKeyView[];
    models: CkeyModelView[];
    errors: string[];
}

export interface CkeyUsageItemView {
    request_id: string;
    model_name: string;
    http_status: number;
    total_tokens: number;
    charged_vnd: number;
    status: string;
    latency_ms: number;
    created_at_text: string;
}

export interface CkeyUsageView {
    items: CkeyUsageItemView[];
    page: number;
    total_pages: number;
}

export interface CkeyImportItem {
    id: string;
    display_name: string;
    input_price: number;
    output_price: number;
    context_limit: number;
    output_limit: number;
    in_config: boolean;
    stale: boolean;
}

export interface CkeyImportResult {
    added: number;
    removed: number;
    kept: number;
    provider_created: boolean;
}
