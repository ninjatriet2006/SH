/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc duyệt file (Explorer).
Trách nhiệm: Gọi lệnh list_remote, file_operations.
Các module tương tác: frontend/src/main.ts, backend/src/explorer.rs
*/

// @ts-ignore
import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../frontend/src/store.ts';
import { debugStore } from '../frontend/src/services/debugStore.ts';

export async function listFiles(path: string): Promise<FileItem[]> {
  try {
    debugStore.log('API', 'list_files', { path });
    const files = await invoke<FileItem[]>('list_files', { path });
    return files;
  } catch (error) {
    console.error(`Lỗi khi lấy file từ ${path}:`, error);
    return [];
  }
}

export async function getTempDir(): Promise<string> {
  debugStore.log('API', 'fs_temp_dir', {});
  return await invoke('fs_temp_dir');
}

export async function fsMkdir(path: string): Promise<void> {
  debugStore.log('API', 'fs_mkdir', { path });
  await invoke('fs_mkdir', { path });
}

export async function fsDelete(path: string): Promise<void> {
  debugStore.log('API', 'fs_delete', { path });
  await invoke('fs_delete', { path });
}

export async function fsRename(oldPath: string, newPath: string): Promise<void> {
  debugStore.log('API', 'fs_rename', { oldPath, newPath });
  await invoke('fs_rename', { oldPath, newPath });
}

export async function fsCancel(taskId: number): Promise<void> {
  debugStore.log('API', 'fs_cancel', { taskId });
  return await invoke('fs_cancel', { taskId });
}

export async function fsCopy(src: string, dst: string, taskId?: number): Promise<void> {
  debugStore.log('API', 'fs_copy', { src, dst });
  await invoke('fs_copy', { src, dst, taskId });
}

export async function fsMove(src: string, dst: string, taskId?: number): Promise<void> {
  debugStore.log('API', 'fs_move', { src, dst });
  await invoke('fs_move', { src, dst, taskId });
}

export interface StatInfo {
  size: number;
  file_count: number;
  dir_count: number;
  permissions: number;
  uid: number;
  gid: number;
}

export async function fsStatAdvanced(path: string): Promise<StatInfo> {
  return invoke<StatInfo>('fs_stat_advanced', { path });
}

export interface SearchResultItem {
  item: FileItem;
  path: string;
  score?: number;
}

export async function fsSearch(path: string, query: string): Promise<SearchResultItem[]> {
  return invoke<SearchResultItem[]>('fs_search', { path, query });
}
