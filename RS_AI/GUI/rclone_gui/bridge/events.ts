/*
 * Đăng ký nghe sự kiện backend đẩy lên (không phải lệnh gọi-đáp).
 * - `job_update` (tiến độ job) → gói trong `jobs.ts` cạnh lệnh job.
 * - `local-dir-changed` (thư mục đang xem đổi ngoài app) → ở đây.
 * - `backend-log` (dòng nhật ký mới) → ở đây; lịch sử cũ lấy qua `getBackendLog`.
 */

import { listen, type UnlistenFn } from './ipc';
import type { BackendLogEvent } from './types';

/** Nghe thư mục đang xem bị đổi từ ngoài app (để UI tự nạp lại). */
export async function subscribeLocalDirChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('local-dir-changed', () => {
    try {
      cb();
    } catch (e) {
      console.error('local-dir-changed handler fail:', e);
    }
  });
}

/** Nghe dòng nhật ký backend mới (cho DebugView). */
export async function subscribeBackendLog(cb: (entry: BackendLogEvent) => void): Promise<UnlistenFn> {
  return listen<BackendLogEvent>('backend-log', (event) => {
    try {
      cb(event.payload);
    } catch (e) {
      console.error('backend-log handler fail:', e);
    }
  });
}
