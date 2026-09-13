/*
[INTEGRITY NOTES]
- Mục đích: Bridge cho ARBITER — model trọng tài chấm điểm các model.
- Trách nhiệm: Gọi lệnh Tauri, ném lỗi ra UI.
- Tương tác: `pages/ModelsPage.tsx`, backend `api/arbiter.rs`.
*/

import { invokeIpc, ipcErrorMessage } from './ipc';
import type { ArbiterState, ArbiterVerdict, RecommendationView } from './types';

/** Trạng thái arbiter (lịch sử + kết quả đồng thuận + danh sách ứng viên). */
export async function getArbiterState(): Promise<ArbiterState> {
    try {
        return await invokeIpc<ArbiterState>('get_arbiter_state');
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/**
 * Chạy một lần đánh giá bằng model trọng tài. Trả về kết quả cuối cùng sau
 * khi cộng dồn lần chạy mới vào lịch sử (tối đa 5 lần gần nhất).
 */
export async function runArbiterEvaluation(
    arbiterProvider: string,
    arbiterModel: string,
): Promise<ArbiterVerdict[]> {
    try {
        return await invokeIpc<ArbiterVerdict[], { arbiter_provider: string; arbiter_model: string }>('run_arbiter_evaluation', {
            arbiter_provider: arbiterProvider,
            arbiter_model: arbiterModel,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/** Xoá lịch sử chấm — lần chạy kế tiếp tính lại từ "lần 1". */
export async function clearArbiterHistory(): Promise<void> {
    try {
        await invokeIpc<void>('clear_arbiter_history');
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

/**
 * Xếp hạng model cho một tác vụ ("coding" mặc định) từ kết quả đồng thuận của
 * arbiter. Trả về danh sách giảm dần theo điểm + số model chưa được judge.
 */
export async function recommendModels(task?: string | null): Promise<RecommendationView> {
    try {
        return await invokeIpc<RecommendationView, { task: string | null }>('recommend_models', {
            task: task ?? null,
        });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}
