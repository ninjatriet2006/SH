/*
[INTEGRITY NOTES]
- Mục đích: Bridge gọi API CKey (ckey.vn) xuống backend Rust.
- Trách nhiệm: Gọi lệnh Tauri và NÉM lỗi ra cho UI xử lý. Không nuốt lỗi, không
  lưu key thật trong state lâu hơn cần thiết.
- Tương tác: UI trang CKey, backend `api/ckey.rs`.

Mô hình: PROFILE = một tài khoản ckey.vn (account key quản lý). Nhiều profile,
chuyển đổi qua active. IMPORT chọn provider đích + dùng AI key của profile
đang xem. `force = true` bỏ cache TTL (nút Refresh).

Tham số dùng snake_case khớp `rename_all = "snake_case"` ở backend.
*/

import { invokeIpc, ipcErrorMessage } from './ipc';
import type {
    CkeyDashboard, CkeyDepositView, CkeyImportList, CkeyImportResult,
    CkeyProfileView, CkeyUsageView,
} from './types';

// ---------- PROFILE (tài khoản CKey) ----------

export async function listCkeyProfiles(): Promise<CkeyProfileView[]> {
    try {
        return await invokeIpc<CkeyProfileView[]>('list_ckey_profiles');
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Thêm (profileId rỗng) hoặc sửa profile. Trả về id profile. */
export async function saveCkeyProfile(args: {
    profileId?: string | null;
    name: string;
    key: string;
}): Promise<string> {
    try {
        return await invokeIpc<string, { profile_id: string | null; name: string; key: string }>('save_ckey_profile', {
            profile_id: args.profileId ?? null,
            name: args.name,
            key: args.key,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Xoá profile (dọn binding trỏ tới nó). Trả về id active mới nếu còn. */
export async function deleteCkeyProfile(profileId: string): Promise<string | null> {
    try {
        return await invokeIpc<string | null, { profile_id: string }>('delete_ckey_profile', { profile_id: profileId });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Chuyển tài khoản đang xem sang profile khác. */
export async function setActiveCkeyProfile(profileId: string): Promise<void> {
    try {
        await invokeIpc<void, { profile_id: string }>('set_active_ckey_profile', { profile_id: profileId });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

// ---------- DASHBOARD / USAGE / DEPOSIT (theo profile, có cache) ----------

/**
 * Hồ sơ + thống kê + AI keys + models trong một lần gọi.
 * `sinceDays` = chỉ tính thống kê N ngày gần nhất. `force` = bỏ cache TTL.
 */
export async function fetchCkeyDashboard(
    profileId: string,
    sinceDays?: number | null,
    force?: boolean,
): Promise<CkeyDashboard> {
    try {
        return await invokeIpc<CkeyDashboard, { profile_id: string; since_days: number | null; force: boolean | null }>('fetch_ckey_dashboard', {
            profile_id: profileId,
            since_days: sinceDays ?? null,
            force: force ?? false,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Lịch sử dùng AI, phân trang. `model` (tuỳ chọn) = lọc theo tên model. */
export async function fetchCkeyUsage(
    profileId: string,
    page: number,
    limit: number,
    model?: string | null,
    force?: boolean,
): Promise<CkeyUsageView> {
    try {
        return await invokeIpc<CkeyUsageView, {
            profile_id: string; page: number; limit: number; model: string | null; force: boolean;
        }>('fetch_ckey_usage', {
            profile_id: profileId,
            page,
            limit,
            model: model ?? null,
            force: force ?? false,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Thông tin nạp tiền (QR/nội dung CK theo số tiền) + lịch sử nạp, một lần gọi. */
export async function fetchCkeyDeposit(
    profileId: string,
    amount: number,
    page: number,
    limit: number,
    force?: boolean,
): Promise<CkeyDepositView> {
    try {
        return await invokeIpc<CkeyDepositView, {
            profile_id: string; amount: number; page: number; limit: number; force: boolean;
        }>('fetch_ckey_deposit', {
            profile_id: profileId,
            amount,
            page,
            limit,
            force: force ?? false,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

// ---------- IMPORT ----------

/**
 * Catalogue model + giá để import — TOÀN CỤC (giống nhau mọi tài khoản CKey).
 * Backend tự suy provider ĐÍCH từ binding của tài khoản đang xem (mặc định id
 * chuẩn "ckey"); kết quả kèm đích để hiển thị. Không cần chọn gì cả.
 */
export async function listCkeyImportItems(profileId: string): Promise<CkeyImportList> {
    try {
        return await invokeIpc<CkeyImportList, { profile_id: string }>('list_ckey_import_items', { profile_id: profileId });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/**
 * `selected` là danh sách CUỐI CÙNG — model không có trong đây sẽ bị xoá.
 * AI key + provider đích theo profile `profileId` (backend suy từ binding).
 */
export async function importCkeyModels(
    profileId: string,
    selected: string[],
): Promise<CkeyImportResult> {
    try {
        return await invokeIpc<CkeyImportResult, { profile_id: string; selected: string[] }>('import_ckey_models', {
            profile_id: profileId,
            selected,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}
