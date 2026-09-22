/*
 * Thùng rác Local + Remote — tương ứng `backend/src/api/trash_manager.rs`.
 * `account` để trống/null = Local. Remote dùng họ lệnh `_remote_terminal`.
 */

import { invoke } from './ipc';
import type { FileItem, TrashItemLocal } from './types';

/** Liệt kê thùng rác ổ máy. */
export async function trashListLocal(): Promise<TrashItemLocal[]> {
  try {
    return await invoke<TrashItemLocal[]>('fs_trash_list_local');
  } catch (error) {
    console.error('Lỗi fs_trash_list_local:', error);
    return [];
  }
}

/** Khôi phục một mục thùng rác ổ máy về chỗ cũ. */
export async function trashRestoreLocal(itemId: string): Promise<void> {
  await invoke('fs_trash_restore_local', { item_id: itemId });
}

/** Xoá vĩnh viễn một mục thùng rác ổ máy. */
export async function trashDeleteLocal(itemId: string): Promise<void> {
  await invoke('fs_trash_delete_local', { item_id: itemId });
}

/** Dọn sạch thùng rác ổ máy. */
export async function trashEmptyLocal(): Promise<void> {
  await invoke('fs_trash_empty_local');
}

/** Liệt kê thùng rác remote (cloud). */
export async function trashListRemote(account?: string | null): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_trash_list_remote_terminal', { account: account ?? null });
  } catch (error) {
    console.error('Lỗi fs_trash_list_remote_terminal:', error);
    return [];
  }
}

/** Khôi phục một mục thùng rác remote. */
export async function trashRestoreRemote(account: string | null, path: string): Promise<void> {
  await invoke('fs_trash_restore_remote_terminal', { account, path });
}

/** Xoá vĩnh viễn một mục thùng rác remote. */
export async function trashDeleteRemote(account: string | null, path: string): Promise<void> {
  await invoke('fs_trash_delete_remote_terminal', { account, path });
}

/** Dọn sạch thùng rác remote. */
export async function trashEmptyRemote(account?: string | null): Promise<void> {
  await invoke('fs_trash_empty_remote_terminal', { account: account ?? null });
}
