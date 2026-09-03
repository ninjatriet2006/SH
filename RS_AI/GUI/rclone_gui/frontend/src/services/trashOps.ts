/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) trên giao diện (Frontend).
- Trách nhiệm: Gọi xuống backend Tauri để liệt kê, khôi phục, xoá vĩnh viễn.
- Tương tác: Được gọi từ DualPaneExplorer (khi path là `trash://...`) và ContextMenu.

Quy ước đường dẫn ảo:
  `trash://local`      → thùng rác hệ điều hành (chuẩn FreeDesktop)
  `trash://<remote>`   → thùng rác của một remote rclone (Drive/Jottacloud/PikPak)
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

/** Tiền tố nhận diện đường dẫn thùng rác ảo. */
export const TRASH_PREFIX = 'trash://';

/** Tên hàm: isTrashPath | Mô tả: Kiểm tra một path có phải đường dẫn thùng rác ảo. */
export function isTrashPath(path: string): boolean {
  return path.startsWith(TRASH_PREFIX);
}

/**
 * Tên hàm: parseTrashPath
 * Mô tả: Bóc `trash://local` / `trash://GDrive` thành đích cụ thể.
 * Trả `null` nếu không phải đường dẫn thùng rác.
 */
export function parseTrashPath(path: string): { isLocal: boolean; remote?: string } | null {
  if (!isTrashPath(path)) return null;
  const target = path.slice(TRASH_PREFIX.length).replace(/\/+$/, '');
  if (!target || target === 'local') return { isLocal: true };
  return { isLocal: false, remote: target };
}

// ====================================================================================
// BLOCK: QUẢN LÝ THÙNG RÁC ĐÁM MÂY (REMOTE TRASH)
// ====================================================================================

/** Tên hàm: listRemoteTrash | Mô tả: Liệt kê các mục trong thùng rác của Cloud. */
export async function listRemoteTrash(account: string): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_trash_list_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

/**
 * Tên hàm: restoreRemoteTrash
 * Mô tả: Khôi phục một mục từ thùng rác đám mây về vị trí gốc.
 * `path` là đường dẫn tương đối trong thùng rác (không dùng chỉ số mảng — chỉ số
 * lệch ngay khi danh sách thay đổi và có thể tác động lên sai mục).
 */
export async function restoreRemoteTrash(path: string, account: string): Promise<void> {
  try {
    await invoke('fs_trash_restore_remote_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: deleteRemoteTrash | Mô tả: Xoá vĩnh viễn một mục khỏi thùng rác đám mây. */
export async function deleteRemoteTrash(path: string, account: string): Promise<void> {
  try {
    await invoke('fs_trash_delete_remote_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: emptyRemoteTrash | Mô tả: Dọn sạch toàn bộ thùng rác đám mây. */
export async function emptyRemoteTrash(account: string): Promise<void> {
  try {
    await invoke('fs_trash_empty_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

// ====================================================================================
// BLOCK: QUẢN LÝ THÙNG RÁC CỤC BỘ (LOCAL OS TRASH)
// ====================================================================================

/** Cấu trúc dữ liệu đại diện cho một mục trong thùng rác cục bộ của Hệ Điều Hành */
export interface TrashItemLocal {
  /** Tên mục trong `Trash/files/` — định danh để khôi phục / xoá vĩnh viễn. */
  id: string;
  name: string;
  original_path: string;
  time_deleted: string;
}

/** Tên hàm: listLocalTrash | Mô tả: Liệt kê các mục trong thùng rác cục bộ. */
export async function listLocalTrash(): Promise<TrashItemLocal[]> {
  try {
    return await invoke<TrashItemLocal[]>('fs_trash_list_local');
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: restoreLocalTrash | Mô tả: Khôi phục một mục về vị trí gốc. */
export async function restoreLocalTrash(itemId: string): Promise<void> {
  try {
    await invoke('fs_trash_restore_local', { itemId });
  } catch (e) {
    throw toError(e);
  }
}

/** Tên hàm: deleteLocalTrash | Mô tả: Xoá vĩnh viễn một mục khỏi thùng rác cục bộ. */
export async function deleteLocalTrash(itemId: string): Promise<void> {
  try {
    await invoke('fs_trash_delete_local', { itemId });
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

/**
 * Tên hàm: listTrash
 * Mô tả: Liệt kê thùng rác theo đường dẫn ảo, quy đổi về `FileItem` để pane render
 * được như thư mục thường. `uuid` giữ định danh dùng cho khôi phục / xoá.
 */
export async function listTrash(path: string): Promise<FileItem[]> {
  const target = parseTrashPath(path);
  if (!target) return [];

  if (target.isLocal) {
    const items = await listLocalTrash();
    return items.map((it) => ({
      uuid: it.id,
      name: it.name,
      is_dir: false, // Thùng rác XDG không ghi rõ kiểu; hiển thị phẳng.
      size: 0,
      mod_time: it.time_deleted,
      file_type: it.original_path, // Cột Type dùng để hiện vị trí gốc.
    }));
  }

  return listRemoteTrash(target.remote!);
}
