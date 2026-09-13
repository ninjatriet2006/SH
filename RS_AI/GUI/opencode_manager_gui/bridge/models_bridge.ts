/*
[INTEGRITY NOTES]
- Mục đích: Bridge cho ma trận so sánh model + chọn model chính.
- Trách nhiệm: Gọi lệnh Tauri, ném lỗi ra UI.
- Tương tác: `pages/ModelsPage.tsx`, `components/ModelsModal.tsx`, backend
  `api/models.rs`.
*/

import { invokeIpc, ipcErrorMessage } from './ipc';
import type { ModelMatrixRow } from './types';

/** Ma trận so sánh mọi model trong config (đã làm giàu từ cache models.dev). */
export async function listModelMatrix(): Promise<ModelMatrixRow[]> {
    try {
        return await invokeIpc<ModelMatrixRow[]>('list_model_matrix');
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/**
 * Đặt model chính (field `model` của opencode.json).
 * Truyền null cho cả hai → bỏ chọn (dùng mặc định của OpenCode).
 * Trả về chuỗi model mới (vd "ckey/provider/gpt") hoặc null.
 */
export async function setPrimaryModel(
    providerId: string | null,
    modelId: string | null,
): Promise<string | null> {
    try {
        return await invokeIpc<string | null, { provider_id: string | null; model_id: string | null }>('set_primary_model', {
            provider_id: providerId,
            model_id: modelId,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Đồng bộ limit (context/output) của model conflict về giá trị models.dev. */
export async function syncLimitsFromDev(): Promise<number> {
    try {
        return await invokeIpc<number>('sync_limits_from_dev');
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}
