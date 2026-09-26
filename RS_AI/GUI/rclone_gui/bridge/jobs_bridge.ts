/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho hàng đợi công việc (Job Queue).
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng các kiểu Job, JobKind trong `types.ts` và lắng nghe sự kiện `job_update`.
*/

import { listen, type UnlistenFn } from './ipc';
import { invokeCommand, type Job, type JobKind } from './types';

/**
 * Đặt một việc vào hàng chờ. `src`/`dst` dạng `Remote::/path` (Local trần).
 * `skipPaths` (tùy chọn): mảng đường dẫn con bỏ qua nếu xảy ra trùng lặp.
 */
export async function jobEnqueue(
  kind: JobKind,
  src?: string | null,
  dst?: string | null,
  skipPaths?: string[],
): Promise<Job> {
  return await invokeCommand<
    Job,
    { kind: string; src: string | null; dst: string | null; skip_paths: string[] | null }
  >('job_enqueue', {
    kind,
    src: src ?? null,
    dst: dst ?? null,
    skip_paths: skipPaths ?? null,
  });
}

/** Liệt kê snapshot toàn bộ job trong hệ thống. */
export async function jobList(): Promise<Job[]> {
  try {
    return await invokeCommand<Job[]>('job_list');
  } catch (error) {
    console.error('Lỗi job_list:', error);
    return [];
  }
}

/** Huỷ job (queued → cancelled ngay; running → dừng ở bước tiếp theo). */
export async function jobCancel(jobId: string): Promise<Job> {
  return await invokeCommand<Job, { job_id: string }>('job_cancel', { job_id: jobId });
}

/** Lấy danh sách ID các job đang chờ trong hàng đợi theo thứ tự thực thi. */
export async function jobGetQueue(): Promise<string[]> {
  try {
    return await invokeCommand<string[]>('job_get_queue');
  } catch (error) {
    console.error('Lỗi job_get_queue:', error);
    return [];
  }
}

/** Đổi thứ tự toàn bộ hàng đợi theo mảng ID được cấp. */
export async function jobReorder(orderedIds: string[]): Promise<void> {
  await invokeCommand<void, { ordered_ids: string[] }>('job_reorder', { ordered_ids: orderedIds });
}

/** Đẩy job lên trước 1 vị trí trong hàng đợi. */
export async function jobMoveUp(jobId: string): Promise<void> {
  await invokeCommand<void, { job_id: string }>('job_move_up', { job_id: jobId });
}

/** Đẩy job xuống sau 1 vị trí trong hàng đợi. */
export async function jobMoveDown(jobId: string): Promise<void> {
  await invokeCommand<void, { job_id: string }>('job_move_down', { job_id: jobId });
}

/** Đưa job lên đầu hàng đợi chờ. */
export async function jobMoveToTop(jobId: string): Promise<void> {
  await invokeCommand<void, { job_id: string }>('job_move_to_top', { job_id: jobId });
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
