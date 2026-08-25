// fileOps.ts — Module wrapper quanh `invoke` cho mọi fs_* command.
//
// Mỗi hàm trả Promise và bọc try/catch để lỗi command không làm crash UI.
// Tên command khớp với contract trong src-tauri/src/lib.rs (docs/app-shell.md §3.2).
import { invoke } from '@tauri-apps/api/core';
import { transferManager } from '../features/transferManager';
import { baseName, joinPath } from '../features/dragDrop';
import type { FileItem } from '../store';
import { undoManager } from './undoManager';

/** Chuyển lỗi invoke thành Error có message rõ ràng. */
function toError(e: unknown): Error {
  if (e instanceof Error) return e;
  return new Error(String(e));
}

async function runWithSudoFallback<T>(action: string, args: string[], fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (e: any) {
    const errStr = String(e).toLowerCase();
    if (errStr.includes('permission denied') || errStr.includes('access is denied') || errStr.includes('os error 13')) {
      if (confirm(`Lỗi phân quyền (Permission Denied).\nBạn có muốn thử lại thao tác này với quyền quản trị viên (Root/Admin) không?`)) {
        await invoke('fs_sudo_exec', { action, args });
        return undefined as T;
      }
    }
    throw toError(e);
  }
}

// ── Liệt kê ────────────────────────────────────────────────────────────────
export async function listRemote(
  path: string,
  account?: string,
): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_list_remote_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

export async function listLocal(path: string): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('fs_list_local', { path });
  } catch (e) {
    throw toError(e);
  }
}

// ── Thư mục ────────────────────────────────────────────────────────────────
export async function mkdir(path: string, account?: string): Promise<void> {
  try {
    await invoke('fs_mkdir_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

export async function mkdirLocal(path: string): Promise<void> {
  await runWithSudoFallback('mkdir', [path], async () => {
    await invoke('fs_mkdir_local', { path });
  });
}

// ── Xoá / đổi tên / sao chép / di chuyển ──────────────────────────────────
export async function remove(path: string, account?: string): Promise<void> {
  try {
    await invoke('fs_delete_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

export async function rename(
  path: string,
  newName: string,
  account?: string,
): Promise<void> {
  try {
    await invoke('fs_rename_terminal', { account, path, new_name: newName });
    // Undo
    const lastSlash = path.replace(/\\/g, '/').lastIndexOf('/');
    const parentDir = lastSlash >= 0 ? path.substring(0, lastSlash) : '';
    const dest = joinPath(parentDir, newName);
    undoManager.push({
      type: 'rename',
      src: path,
      dest: dest,
      account,
      isLocal: !account
    });
  } catch (e) {
    throw toError(e);
  }
}

export async function renameLocal(path: string, newName: string): Promise<void> {
  const parent = baseName(path) ? path.substring(0, path.lastIndexOf('/')) : path;
  const newPath = parent ? `${parent}/${newName}` : `/${newName}`;
  
  await runWithSudoFallback('mv', [path, newPath], async () => {
    await invoke('fs_rename_local', { path, new_name: newName });
    undoManager.push({
      type: 'rename',
      src: path,
      dest: newPath,
      isLocal: true
    });
  });
}

export async function copy(
  src: string,
  dest: string,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _account?: string,
): Promise<void> {
  const name = baseName(src);
  await transferManager.enqueue('copy', name, src, dest, false, false);
}

export async function move(
  src: string,
  dest: string,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _account?: string,
): Promise<void> {
  const name = baseName(src);
  await transferManager.enqueue('move', name, src, dest, false, false, true);
}

// ── Local copy/paste (copy/paste giữa pane local-local) ────────────────────
export async function cpLocal(
  from: string,
  to: string,
  _overwrite = true,
): Promise<void> {
  // TransferManager xử lý chạy nền, tạm thời chỉ catch lỗi ở mức UI queue
  const name = baseName(from);
  await transferManager.enqueue('copy', name, from, to, true, true);
}

export async function moveLocal(from: string, to: string): Promise<void> {
  const name = baseName(from);
  await transferManager.enqueue('move', name, from, to, true, true, true);
}

export async function cpBatch(
  srcs: string[],
  dstDir: string,
  overwrite = true,
): Promise<void> {
  for (const src of srcs) {
    const dst = joinPath(dstDir, baseName(src));
    await cpLocal(src, dst, overwrite);
  }
}

export async function rmLocal(path: string): Promise<void> {
  await runWithSudoFallback('rm', [path], async () => {
    await invoke('fs_rm_local', { path });
  });
}

// ── Upload / Download ──────────────────────────────────────────────────────
export async function upload(
  local: string,
  remote: string,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _account?: string,
): Promise<void> {
  const name = baseName(local);
  await transferManager.enqueue('upload', name, local, remote, true, false);
}

export async function download(
  remote: string,
  local: string,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  _account?: string,
): Promise<void> {
  const name = baseName(remote);
  await transferManager.enqueue('download', name, remote, local, false, true);
}

// ── Đọc / ghi nội dung ─────────────────────────────────────────────────────
export async function cat(path: string, account?: string): Promise<string> {
  try {
    return await invoke<string>('fs_cat_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

export async function write(
  path: string,
  content: string,
  account?: string,
): Promise<void> {
  try {
    await invoke('fs_write_terminal', { account, path, content });
  } catch (e) {
    throw toError(e);
  }
}

export async function writeLocal(path: string, content: string): Promise<void> {
  try {
    await invoke('fs_write_local', { path, content });
  } catch (e) {
    throw toError(e);
  }
}

// ── Link công khai ─────────────────────────────────────────────────────────
export async function linkCreate(
  path: string,
  account?: string,
): Promise<string> {
  try {
    return await invoke<string>('fs_link_create_terminal', { account, path });
  } catch (e) {
    throw toError(e);
  }
}

export async function linksList(
  account?: string,
): Promise<[string, string][]> {
  try {
    return await invoke<[string, string][]>('fs_links_list_terminal', { account });
  } catch (e) {
    throw toError(e);
  }
}

// ── Mở file ngoài hệ thống ─────────────────────────────────────────────────
export async function open(path: string): Promise<void> {
  try {
    await invoke('fs_open', { path });
  } catch (e) {
    throw toError(e);
  }
}

export interface StatInfo {
  size: number;
  file_count: number;
  dir_count: number;
  permissions: number;
  uid: number;
  gid: number;
}

export async function statAdvanced(path: string): Promise<StatInfo> {
  try {
    return await invoke<StatInfo>('fs_stat_advanced', { path });
  } catch (e) {
    throw toError(e);
  }
}

export async function chmod(path: string, mode: number): Promise<void> {
  try {
    await invoke('fs_chmod', { path, mode });
  } catch (e) {
    throw toError(e);
  }
}

export async function chown(path: string, uid: number, gid: number): Promise<void> {
  return invoke('fs_chown', { path, uid, gid });
}

export async function getFreeSpace(path: string): Promise<number> {
  if (path.startsWith('trash://')) return 0;
  try {
    return await invoke<number>('fs_get_free_space', { path });
  } catch (e) {
    throw toError(e);
  }
}

export interface SearchResult {
  item: FileItem;
  path: string;
  score?: number;
}

export interface SearchOptions {
  fuzzy: boolean;
  content_query?: string | null;
  min_size?: number | null;
  max_size?: number | null;
}

export async function searchLocal(path: string, query: string, options?: SearchOptions): Promise<SearchResult[]> {
  try {
    return await invoke<SearchResult[]>('fs_search_local', { path, query, options });
  } catch (e) {
    throw toError(e);
  }
}