/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho quản lý Thùng rác (Trash).
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng TrashItemLocal và FileItem từ `types.ts`.
*/

import { invokeCommand, type FileItem, type TrashItemLocal } from './types';

/** Liệt kê danh sách các mục trong thùng rác hệ điều hành cục bộ (Local). */
export async function listTrashLocal(): Promise<TrashItemLocal[]> {
  try {
    return await invokeCommand<TrashItemLocal[]>('fs_trash_list_local');
  } catch (error) {
    console.error('Lỗi fs_trash_list_local:', error);
    return [];
  }
}

/** Khôi phục một mục từ thùng rác cục bộ về vị trí ban đầu. */
export async function restoreTrashLocal(itemId: string): Promise<void> {
  await invokeCommand<void, { item_id: string }>('fs_trash_restore_local', { item_id: itemId });
}

/** Xoá vĩnh viễn một mục khỏi thùng rác cục bộ. */
export async function deleteTrashLocal(itemId: string): Promise<void> {
  await invokeCommand<void, { item_id: string }>('fs_trash_delete_local', { item_id: itemId });
}

/** Dọn sạch toàn bộ thùng rác cục bộ. */
export async function emptyTrashLocal(): Promise<void> {
  await invokeCommand<void>('fs_trash_empty_local');
}

/** Liệt kê danh sách các mục trong thùng rác đám mây của một remote. */
export async function listTrashRemote(account: string): Promise<FileItem[]> {
  try {
    return await invokeCommand<FileItem[], { account: string }>('fs_trash_list_remote_terminal', {
      account,
    });
  } catch (error) {
    console.error(`Lỗi fs_trash_list_remote_terminal ${account}:`, error);
    return [];
  }
}

/** Khôi phục một mục từ thùng rác đám mây. */
export async function restoreTrashRemote(account: string, path: string): Promise<void> {
  await invokeCommand<void, { account: string; path: string }>('fs_trash_restore_remote_terminal', {
    account,
    path,
  });
}

/** Xoá vĩnh viễn một mục trong thùng rác đám mây. */
export async function deleteTrashRemote(account: string, path: string): Promise<void> {
  await invokeCommand<void, { account: string; path: string }>('fs_trash_delete_remote_terminal', {
    account,
    path,
  });
}

/** Dọn sạch toàn bộ thùng rác đám mây của một remote. */
export async function emptyTrashRemote(account: string): Promise<void> {
  await invokeCommand<void, { account: string }>('fs_trash_empty_remote_terminal', {
    account,
  });
}
