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
    /** Model đang là model chính (⭐ trong UI); null = chưa chọn. */
    primary_model: string | null;
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
    /** Adapter Auto đã probe và lưu; null nếu người dùng tự chọn. */
    detected_npm: string | null;
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
    tool_call: boolean | null;
    reasoning: boolean | null;
    interleaved: string | null;
}

export interface BadProvider {
    id: string;
    name: string;
    kind: StatusKind;
    message: string;
    is_builtin: boolean;
}

export interface ConfigPaths {
    /** Thư mục cấu hình runtime do OpenCode sở hữu. */
    opencode_config_dir: string;
    /** Thư mục trạng thái do OpenCode Manager sở hữu. */
    manager_config_dir: string;
    opencode_json: string;
    auth_json: string;
    ckey_json: string;
    arbiter_json: string;
    settings_json: string;
}

export interface GuiSettings {
    language: string;
    theme_id: string;
    font_id: string;
}

export type WebState = 'stopped' | 'starting' | 'running' | 'stopping' | 'error';

/** Exact backend-owned OpenCode Web lifecycle snapshot. */
export interface WebStatus {
    state: WebState;
    url: string | null;
    error: string | null;
    /** Backend vẫn sở hữu child handle và có thể dừng tiến trình đó. */
    has_owned_child: boolean;
    generation: number;
    revision: number;
}

/** Contract local cho năm command Web đã đăng ký và đang được UI sử dụng. */
export interface WebCommandPayloads {
    web_status: Record<string, never>;
    web_start: Record<string, never>;
    web_stop: Record<string, never>;
    launch_terminal: Record<string, never>;
    open_web_url: { url: string };
}

export interface FontInfo {
    id: string;
    name: string;
    family: string;
    src_path: string | null;
}

export interface Theme {
    id: string;
    name: string;
    type: 'dark' | 'light';
    colors: Record<string, string>;
}

/** Một dòng bảng so sánh model (backend `api/models.rs`). */
export interface ModelMatrixRow {
    provider_id: string;
    provider_name: string;
    model_id: string;
    display_name: string;
    /** Đang là model chính (field `model` của opencode.json). */
    is_primary: boolean;
    /** null = chưa khai (OpenCode dùng mặc định của model). */
    tool_call: boolean | null;
    reasoning: boolean | null;
    /** Nhận ảnh. */
    vision: boolean | null;
    context: number | null;
    output: number | null;
    /** Giá USD / 1M token (từ cache models.dev). */
    price_input: number | null;
    price_output: number | null;
    price_cache_read: number | null;
    /** Có ít nhất một field lấy từ models.dev. */
    enriched: boolean;
    /** Giới hạn theo models.dev (đối chiếu khi lệch config). */
    dev_context: number | null;
    dev_output: number | null;
    /** Ước lượng theo tên model (heuristic — nguồn yếu nhất). */
    heur_context: number | null;
    heur_output: number | null;
    /** Nguồn giá trị hiển thị: 'config' | 'models.dev' | 'name' | ''. */
    context_source: string;
    output_source: string;
    /** Config khai limit LỆCH models.dev (số mặc định cũ còn sót). */
    limit_conflict: boolean;
}

/** Điểm arbiter chấm cho một model (0-100 từng hạng mục). */
export interface ArbiterVerdict {
    /** Khóa "pid/mid". */
    model: string;
    coding: number;
    reasoning: number;
    tool_use: number;
    vision: number;
    overall: number;
    /** 0 = restricted, 100 = least/no restriction; null = unknown. */
    safety_freedom: number | null;
    safety_confidence: number | null;
    safety_evidence: string[];
    overall_version: number;
    /** Số lần chạy được tính vào đồng thuận (≤ 5). */
    runs: number;
    /** `false` = điểm chưa hội tụ giữa các lần (cần chạy thêm). */
    stable: boolean;
    /** Trung vị ước lượng limit của arbiter (null = chưa chắc). */
    context: number | null;
    output: number | null;
    /** Ghi chú nguyên văn của lần chạy mới nhất. */
    note: string;
}

/** Một lựa chọn model trọng tài trong dropdown. */
export interface ArbiterCandidate {
    provider_id: string;
    model_id: string;
    /** Đang là model chính (gợi ý mặc định). */
    is_primary: boolean;
}

/** Một điều chỉnh điểm (giải thích được) trong ranking. */
export interface ScoreAdjustment {
    /** 'ctx' | 'output' | 'tools' | 'coding_bonus' | 'price' */
    kind: string;
    delta: number;
}

/** Một model trong bảng xếp hạng theo tác vụ. */
export interface RecommendedModel {
    provider_id: string;
    model_id: string;
    display_name: string;
    /** Điểm tổng hợp 0-100. */
    score: number;
    /** 'S' (top tier) | 'A' | 'B' | 'C'. */
    tier: string;
    /** Điểm gốc của judge cho phạm vi chính của tác vụ. */
    base: number;
    adjustments: ScoreAdjustment[];
    context: number | null;
    price_input: number | null;
    price_output: number | null;
}

/** Kết quả recommend_models(task). */
export interface RecommendationView {
    task: string;
    items: RecommendedModel[];
    /** Model trong config chưa được judge (không xếp hạng được). */
    unevaluated: number;
}

export interface ArbiterLastRun {
    at: string;
    arbiter: string;
}

/** Trạng thái arbiter cho tab Models. */
export interface ArbiterState {
    overall_version: number;
    overall_weights: {
        coding: number;
        reasoning: number;
        tool_use: number;
        vision: number;
        safety_freedom: number;
    };
    arbiter: string | null;
    run_count: number;
    last_run: ArbiterLastRun | null;
    verdicts: ArbiterVerdict[];
    candidates: ArbiterCandidate[];
}

/** Kết quả thêm nhanh nhiều provider (backend `api/bulk.rs`). */
export interface BulkAddResult {
    added: number;
    skipped_existing: number;
    skipped_duplicate_input: number;
    created_ids: string[];
    normalized_endpoint: string;
}

/** Một tài khoản CKey đã lưu (key đã che). */
export interface CkeyProfileView {
    id: string;
    name: string;
    key_masked: string;
    /** Tài khoản đang xem (dashboard/usage/deposit). */
    is_active: boolean;
}

/** Hồ sơ tài khoản từ API ckey.vn. */
export interface CkeyAccountInfoView {
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

export interface CkeyDashboard {
    profile: CkeyAccountInfoView | null;
    stats: CkeyStatsView | null;
    keys: CkeyKeyView[];
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

/** Thông tin nạp tiền (docs: /api/deposit-info + /api/deposit-history). */
export interface CkeyDepositView {
    info: CkeyDepositInfoView | null;
    history: CkeyDepositHistoryItemView[];
    history_page: number;
    history_total_pages: number;
    errors: string[];
}

export interface CkeyDepositInfoView {
    transfer_content: string;
    amount_vnd: number;
    expires_at: string;
    qr_url: string;
    banks: CkeyDepositBankView[];
}

export interface CkeyDepositBankView {
    bank_name: string;
    account_owner: string;
    account_number: string;
    transfer_content: string;
    qr_url: string;
}

export interface CkeyDepositHistoryItemView {
    id: number;
    amount_text: string;
    time_text: string;
}

/**
 * Model để import — giá đầy đủ để hiển thị + SẮP XẾP.
 * Catalogue TOÀN CỤC; `id` là public_name (dạng "provider/model") nên tách
 * sẵn `provider`/`model` thành hai cột riêng.
 */
export interface CkeyImportItem {
    /** Id đầy đủ (khóa cấu hình khi import). */
    id: string;
    provider: string;
    model: string;
    display_name: string;
    input_price: number;
    output_price: number;
    cache_read_price: number;
    cache_write_price: number;
    price_per_request: number;
    min_charge_per_request: number;
    cache_enabled: boolean;
    context_limit: number;
    output_limit: number;
    in_config: boolean;
    /** Còn trong config nhưng CKey không còn cung cấp. */
    stale: boolean;
}

/** Kết quả listCkeyImportItems: đích import đã suy + danh sách model. */
export interface CkeyImportList {
    /** Provider sẽ nhận model khi import (suy từ binding tài khoản active). */
    target_provider: string;
    items: CkeyImportItem[];
}

export interface CkeyImportResult {
    added: number;
    removed: number;
    kept: number;
    provider_created: boolean;
}
