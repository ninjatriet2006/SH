/*
[INTEGRITY NOTES]
- Mục đích: Cổng giao tiếp duy nhất giữa Frontend và Backend Tauri v2.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
- Xuất khẩu: Đầy đủ các module bridge, types, và ipc helper.
*/

export * from './types';
export * from './ipc';
export * from './files_bridge';
export * from './remote_bridge';
export * from './mount_bridge';
export * from './config_bridge';
export * from './appearance_bridge';
export * from './jobs_bridge';
export * from './trash_bridge';
export * from './sys_bridge';
export * from './events_bridge';
