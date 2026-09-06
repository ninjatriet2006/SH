/*
[INTEGRITY NOTES]
- Mục đích: Bridge gọi API CKey (ckey.vn) xuống backend Rust.
- Trách nhiệm: Gọi lệnh Tauri và NÉM lỗi ra cho UI xử lý. Không nuốt lỗi, không
  lưu key thật trong state lâu hơn cần thiết.
- Tương tác: UI trang CKey, backend `api/ckey.rs`.

Phân biệt hai loại khoá: account key (quản lý, lưu ở ckey.json) và AI key
(ck-..., gọi LLM). Bridge không bao giờ tự suy ra loại khoá, chỉ truyền đúng
tham số mà backend yêu cầu.

Tham số dùng snake_case khớp `rename_all = "snake_case"` ở backend.
*/

import { invoke } from '@tauri-apps/api/core';
import type {
    CkeyAccountOption, CkeyDashboard, CkeyImportItem, CkeyImportResult,
    CkeyProviderView, CkeyUsageView,
} from './types';

export async function listCkeyProviders(): Promise<CkeyProviderView[]> {
    try {
        return await invoke<CkeyProviderView[]>('list_ckey_providers');
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function listCkeyAccounts(): Promise<CkeyAccountOption[]> {
    try {
        return await invoke<CkeyAccountOption[]>('list_ckey_accounts');
    } catch (error) {
        throw new Error(String(error));
    }
}

/** Gán account key: nhập mới (`accountKey`) hoặc dùng lại (`copyFromProviderId`). */
export async function setCkeyAccountKey(args: {
    providerId: string;
    accountKey?: string;
    copyFromProviderId?: string;
}): Promise<void> {
    try {
        await invoke<void>('set_ckey_account_key', {
            provider_id: args.providerId,
            account_key: args.accountKey ?? null,
            copy_from_provider_id: args.copyFromProviderId ?? null,
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function deleteCkeyAccountKey(providerId: string): Promise<void> {
    try {
        await invoke<void>('delete_ckey_account_key', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

/** Profile + stats + keys + models trong một lần gọi (backend chạy song song). */
export async function fetchCkeyDashboard(providerId: string): Promise<CkeyDashboard> {
    try {
        return await invoke<CkeyDashboard>('fetch_ckey_dashboard', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function fetchCkeyUsage(providerId: string, page: number, limit: number): Promise<CkeyUsageView> {
    try {
        return await invoke<CkeyUsageView>('fetch_ckey_usage', {
            provider_id: providerId,
            page,
            limit,
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function listCkeyImportItems(providerId: string): Promise<CkeyImportItem[]> {
    try {
        return await invoke<CkeyImportItem[]>('list_ckey_import_items', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

/** `selected` là danh sách CUỐI CÙNG — model không có trong đây sẽ bị xoá. */
export async function importCkeyModels(providerId: string, selected: string[]): Promise<CkeyImportResult> {
    try {
        return await invoke<CkeyImportResult>('import_ckey_models', {
            provider_id: providerId,
            selected,
        });
    } catch (error) {
        throw new Error(String(error));
    }
}
