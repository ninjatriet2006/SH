/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc duyệt file (Explorer).
Trách nhiệm: Gọi lệnh list_remote, file_operations.
Các module tương tác: frontend/src/main.ts, backend/src/explorer.rs
*/

// @ts-ignore
import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../frontend/src/store.ts';

export async function listFiles(remote: string, path: string): Promise<FileItem[]> {
  try {
    const files = await invoke<FileItem[]>('list_files', { remote, path });
    return files;
  } catch (error) {
    console.error(`Lỗi khi lấy file từ ${remote}:${path}:`, error);
    return [];
  }
}

export async function fsMkdir(remote: string, path: string): Promise<void> {
  await invoke('fs_mkdir', { remote, path });
}

export async function fsDelete(remote: string, path: string): Promise<void> {
  await invoke('fs_delete', { remote, path });
}

export async function fsRename(remote: string, oldPath: string, newPath: string): Promise<void> {
  await invoke('fs_rename', { remote, oldPath, newPath });
}

export async function fsCopy(srcRemote: string, srcPath: string, destRemote: string, destPath: string): Promise<void> {
  await invoke('fs_copy', { srcRemote, srcPath, destRemote, destPath });
}

export async function fsMove(srcRemote: string, srcPath: string, destRemote: string, destPath: string): Promise<void> {
  await invoke('fs_move', { srcRemote, srcPath, destRemote, destPath });
}

export interface StatInfo {
  size: number;
  file_count: number;
  dir_count: number;
  permissions: number;
  uid: number;
  gid: number;
}

export async function fsStatAdvanced(remote: string, path: string): Promise<StatInfo> {
  return invoke<StatInfo>('fs_stat_advanced', { remote, path });
}

export interface SearchResultItem {
  item: FileItem;
  path: string;
  score?: number;
}

export async function fsSearch(remote: string, path: string, query: string): Promise<SearchResultItem[]> {
  return invoke<SearchResultItem[]>('fs_search', { remote, path, query });
}
