/*
[INTEGRITY NOTES]
Mục đích: Bridge P3 cho hàng chờ việc backend (`core/jobs.rs` qua `ipc::job_*`).
Trách nhiệm: `job_enqueue` đặt việc, `job_list` hydrate, `job_cancel` hủy,
`subscribeJobUpdates` vẽ lại UI từ event `job_update`. IPC cũ (fs_copy/fs_move/...)
giữ nguyên — file này chỉ thêm, không thay thế.
Tương tác: `frontend/src/features/transferManager.ts` (lớp xem mỏng).
*/

import { invoke } from './ipc';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** Kind backend hiểu (`JobKind::from_str`): copy/move/delete/list/manifest. */
export type JobKind = 'copy' | 'move' | 'delete' | 'list' | 'manifest';

/** Trạng thái vòng đời backend (`JobStatus`, snake_case). */
export type JobStatus = 'queued' | 'running' | 'done' | 'error' | 'cancelled';

/** Snapshot một việc như backend trả (progress 0..100). */
export interface Job {
  id: string;
  kind: JobKind;
  src: string | null;
  dst: string | null;
  status: JobStatus;
  progress: number;
  error: string | null;
}

/**
 * Đặt một việc vào hàng chờ backend; worker chạy tuần tự và phát `job_update`.
 * `src`/`dst` là Full Path dạng `Remote::/path` (delete chỉ cần `src`).
 */
export async function jobEnqueue(kind: JobKind, src?: string | null, dst?: string | null): Promise<Job> {
  return invoke<Job>('job_enqueue', { kind, src: src ?? null, dst: dst ?? null });
}

/** Liệt kê snapshot toàn bộ job (dùng hydrate khi tải lại UI). */
export async function jobList(): Promise<Job[]> {
  return invoke<Job[]>('job_list');
}

/** Hủy job (queued → cancelled ngay; running → cờ dừng bước tiếp). */
export async function jobCancel(jobId: string): Promise<Job> {
  return invoke<Job>('job_cancel', { jobId });
}

/** Đăng ký vẽ lại từ event `job_update` của worker; trả hàm hủy đăng ký. */
export async function subscribeJobUpdates(cb: (job: Job) => void): Promise<UnlistenFn> {
  return listen<Job>('job_update', (event: any) => {
    try {
      cb(event.payload);
    } catch (e) {
      console.error('job_update handler fail:', e);
    }
  });
}
