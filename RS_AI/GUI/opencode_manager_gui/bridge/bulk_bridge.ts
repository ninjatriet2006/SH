/*
[INTEGRITY NOTES]
- Mục đích: Bridge gọi API thêm nhanh nhiều provider xuống backend Rust.
- Trách nhiệm: Gọi lệnh Tauri và NÉM lỗi ra cho UI xử lý. Không nuốt lỗi.
- Tương tác: UI form bulk-add, backend `api/bulk.rs`.

Tham số dùng snake_case khớp `rename_all = "snake_case"` ở backend.
*/

import { invokeIpc, ipcErrorMessage } from './ipc';
import type { BulkAddResult } from './types';

/** 1 endpoint + N key (mỗi dòng 1 key) → N provider. */
export async function bulkAddProviders(endpoint: string, keys: string): Promise<BulkAddResult> {
    try {
        return await invokeIpc<BulkAddResult, { endpoint: string; keys: string[] }>('bulk_add_providers', {
            endpoint,
            keys: keys.split(/[\n,]/).map(key => key.trim()).filter(Boolean),
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}
