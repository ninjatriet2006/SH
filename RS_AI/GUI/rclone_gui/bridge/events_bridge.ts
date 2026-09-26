/*
[INTEGRITY NOTES]
- Mục đích: Đăng ký lắng nghe các sự kiện chung từ Backend phát ra (Watcher).
- Trách nhiệm: Bọc Tauri listen an toàn cho thông báo thay đổi thư mục ổ đĩa cục bộ.
- Tương tác: Dùng `listen` từ `ipc.ts`.
*/

import { listen, type UnlistenFn } from './ipc';

/**
 * Lắng nghe thông báo thay đổi thư mục cục bộ (từ notify watcher).
 * Thường kích hoạt reload danh sách file trên pane đang mở.
 */
export async function subscribeLocalDirChanged(cb: () => void): Promise<UnlistenFn> {
  return listen<void>('local-dir-changed', () => {
    try {
      cb();
    } catch (e) {
      console.error('Lỗi trong callback local-dir-changed:', e);
    }
  });
}
