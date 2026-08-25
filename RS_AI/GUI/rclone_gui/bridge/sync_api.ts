/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc đồng bộ (Sync).
Trách nhiệm: Quản lý sync jobs.
Các module tương tác: frontend/src/main.ts, backend/src/sync.rs
*/

export async function startSyncJob(source: string, dest: string): Promise<string> {
  // TODO: Thay thế bằng window.__TAURI__.invoke('start_sync', { source, dest })
  return "job_id_mock";
}
