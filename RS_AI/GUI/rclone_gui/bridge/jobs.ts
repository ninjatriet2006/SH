/*
 * Hàng chờ việc — tương ứng `backend/src/api/jobs.rs`.
 * ĐƯỜNG DUY NHẤT cho copy/move/delete/list nặng: đặt việc → worker chạy tuần
 * tự → tiến độ về qua event `job_update`. Không còn fs_copy/fs_move/fs_cancel.
 */

import { invoke, listen, type UnlistenFn } from './ipc';
import type { Job, JobKind } from './types';

/** Đặt một việc vào hàng chờ. `src`/`dst` dạng `Remote::/path` (Local trần).
 * `skipPaths` (tùy chọn): rel vé con phải bỏ — modal thu từ user, rỗng =
 * mặc định Replace (worker check tươi rồi chép đè tại đó). */
export async function jobEnqueue(
  kind: JobKind,
  src?: string | null,
  dst?: string | null,
  skipPaths?: string[],
): Promise<Job> {
  return await invoke<Job>('job_enqueue', {
    kind,
    src: src ?? null,
    dst: dst ?? null,
    skip_paths: skipPaths ?? null,
  });
}

/** Liệt kê snapshot toàn bộ job (hydrate UI khi tải lại). */
export async function jobList(): Promise<Job[]> {
  try {
    return await invoke<Job[]>('job_list');
  } catch (error) {
    console.error('Lỗi job_list:', error);
    return [];
  }
}

/** Huỷ job (queued → cancelled ngay; running → dừng ở bước tiếp theo). */
export async function jobCancel(jobId: string): Promise<Job> {
  return await invoke<Job>('job_cancel', { job_id: jobId });
}

/** Nghe tiến độ/trạng thái job từ worker; trả hàm huỷ đăng ký. */
export async function subscribeJobUpdates(cb: (job: Job) => void): Promise<UnlistenFn> {
  return listen<Job>('job_update', (event) => {
    try {
      cb(event.payload);
    } catch (e) {
      console.error('job_update handler fail:', e);
    }
  });
}
