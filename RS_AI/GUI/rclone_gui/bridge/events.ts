/*
 * Đăng ký nghe sự kiện backend đẩy lên (không phải lệnh gọi-đáp).
 * - `job_update` (tiến độ job) → gói trong `jobs.ts` cạnh lệnh job.
 * - `local-dir-changed` (thư mục đang xem đổi ngoài app) → ở đây.
 * Nhật ký chẩn đoán backend đọc qua API getBackendLog(), không truyền thẳng qua event.
 */

import { listen, type UnlistenFn } from './ipc';

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
