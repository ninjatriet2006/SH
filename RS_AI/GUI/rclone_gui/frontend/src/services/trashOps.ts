import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../store';

function toError(e: unknown): Error {
  if (e instanceof Error) return e;
  return new Error(String(e));
}

// ── Remote Trash (Cloud) ──────────────────────────────────────────────────
export async function listRemoteTrash(account?: string): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_trash_list_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

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

export async function emptyRemoteTrash(account?: string): Promise<void> {
  try {
    await invoke('fs_trash_empty_remote_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

// ── Local Trash (OS Trash) ────────────────────────────────────────────────
export interface TrashItemLocal {
  id: string;
  name: string;
  original_path: string;
  time_deleted: string;
}

export async function listLocalTrash(): Promise<TrashItemLocal[]> {
  try {
    return await invoke<TrashItemLocal[]>('fs_trash_list_local');
  } catch (e) {
    throw toError(e);
  }
}

export async function restoreLocalTrash(itemId: string): Promise<void> {
  try {
    await invoke('fs_trash_restore_local', { itemId });
  } catch (e) {
    throw toError(e);
  }
}

export async function emptyLocalTrash(): Promise<void> {
  try {
    await invoke('fs_trash_empty_local');
  } catch (e) {
    throw toError(e);
  }
}
