/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) trên giao diện (Frontend).
- Trách nhiệm: Gọi xuống backend Tauri để thực thi việc liệt kê, xóa vĩnh viễn, khôi phục file.
- Tương tác: Được gọi từ TrashView.ts hoặc ContextMenu. Giao tiếp với backend.
*/

import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../store';

// ====================================================================================
// BLOCK: CÁC HÀM TIỆN ÍCH DÙNG CHUNG (UTILITIES)
// ====================================================================================

/** Tên hàm: toError | Mô tả: Hàm hỗ trợ ép kiểu lỗi (Error) chuẩn hóa từ kết quả ném ra (throw) */
function toError(e: unknown): Error {
  if (e instanceof Error) return e;
  return new Error(String(e));
}

// ====================================================================================
// BLOCK: QUẢN LÝ THÙNG RÁC ĐÁM MÂY (REMOTE TRASH)
// ====================================================================================

/** Tên hàm: listRemoteTrash | Mô tả: Liệt kê các file nằm trong thùng rác của Cloud (nếu Cloud hỗ trợ) */
export async function listRemoteTrash(account?: string): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_trash_list_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: restoreRemoteTrash | Mô tả: Khôi phục một file cụ thể từ thùng rác đám mây về vị trí gốc */
export async function restoreRemoteTrash(
  idx: number,
  account?: string,
): Promise<void> {
  try {
    await invoke('fs_trash_restore_remote_terminal', { account, idx });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: deleteRemoteTrash | Mô tả: Xóa vĩnh viễn một file chỉ định ra khỏi thùng rác đám mây */
export async function deleteRemoteTrash(
  idx: number,
  account?: string,
): Promise<void> {
  try {
    await invoke('fs_trash_delete_remote_terminal', { account, idx });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: emptyRemoteTrash | Mô tả: Dọn dẹp sạch sẽ toàn bộ thùng rác đám mây */
export async function emptyRemoteTrash(account?: string): Promise<void> {
  try {
    await invoke('fs_trash_empty_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

// ====================================================================================
// BLOCK: QUẢN LÝ THÙNG RÁC CỤC BỘ (LOCAL OS TRASH)
// ====================================================================================

/** Cấu trúc dữ liệu đại diện cho một file trong thùng rác cục bộ của Hệ Điều Hành */
export interface TrashItemLocal {
  id: string;
  name: string;
  original_path: string;
  time_deleted: string;
}

/** Tên hàm: listLocalTrash | Mô tả: Liệt kê các file trong thùng rác cục bộ trên hệ thống (Linux) */
export async function listLocalTrash(): Promise<TrashItemLocal[]> {
  try {
    return await invoke<TrashItemLocal[]>('fs_trash_list_local');
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: restoreLocalTrash | Mô tả: Khôi phục file trên máy cục bộ bằng UUID của nó */
export async function restoreLocalTrash(itemId: string): Promise<void> {
  try {
    await invoke('fs_trash_restore_local', { itemId });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: emptyLocalTrash | Mô tả: Làm trống thùng rác hệ thống (Local OS Trash) */
export async function emptyLocalTrash(): Promise<void> {
  try {
    await invoke('fs_trash_empty_local');
  } catch (e) {
    throw toError(e);
  }
}
