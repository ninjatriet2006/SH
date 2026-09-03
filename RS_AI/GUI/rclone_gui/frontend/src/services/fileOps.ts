/*
[INTEGRITY NOTES]
- Mục đích: Quản lý và thực thi các thao tác liên quan đến file/thư mục (Tạo, Sửa, Xóa, Copy, Move, ...).
- Trách nhiệm: Đóng vai trò cầu nối mỏng (thin wrapper), chuyển tiếp nguyên vẹn Full Path xuống Backend xử lý.
- Tương tác: Giao tiếp với bridge/explorer_api.ts. Không xử lý logic phức tạp, không xử lý sudo hay path parsing ở đây.
*/

import { invoke } from '@tauri-apps/api/core';
import {
  fsMkdir,
  fsDelete,
  fsRename,
  fsCopy,
  fsMove,
  listFiles,
  fsSearch,
  fsStatAdvanced,
  fsChmod,
  fsChown,
} from '../../../bridge/explorer_api.ts';
import { getAbout } from '../../../bridge/remote_api.ts';
import type { StatInfo, SearchResultItem } from '../../../bridge/explorer_api.ts';
import type { FileItem } from '../store';

export type SearchResult = SearchResultItem;
export type { StatInfo };

export async function listLocal(path: string): Promise<FileItem[]> {
    return listFiles(path);
}

export async function searchLocal(path: string, query: string): Promise<SearchResult[]> {
    return fsSearch(path, query);
}

export async function mkdir(path: string): Promise<void> {
    return fsMkdir(path);
}

export async function remove(path: string): Promise<void> {
    return fsDelete(path);
}

export async function rename(path: string, newName: string): Promise<void> {
    // Chuyển đổi tên mới thành đường dẫn tuyệt đối mới
    const parentDir = path.substring(0, path.lastIndexOf('/'));
    const newPath = parentDir ? `${parentDir}/${newName}` : `/${newName}`;
    return fsRename(path, newPath);
}

export async function copy(src: string, dest: string, taskId?: number): Promise<void> {
    return fsCopy(src, dest, taskId);
}

export async function move(src: string, dest: string, taskId?: number): Promise<void> {
    return fsMove(src, dest, taskId);
}

export async function cpLocal(from: string, to: string, _overwrite: boolean = true, taskId?: number): Promise<void> {
    return fsCopy(from, to, taskId);
}

export async function moveLocal(from: string, to: string, taskId?: number): Promise<void> {
    return fsMove(from, to, taskId);
}

export async function upload(local: string, remoteTarget: string): Promise<void> {
    return fsCopy(local, remoteTarget);
}

export async function download(remoteSource: string, local: string): Promise<void> {
    return fsCopy(remoteSource, local);
}

export async function write(path: string, content: string): Promise<void> {
    if (content === "") {
        await invoke('fs_touch', { path });
    } else {
        alert("Tính năng ghi nội dung trực tiếp chưa được hỗ trợ.");
    }
}

export async function open(path: string): Promise<void> {
    await invoke('sys_open_with', { path, execCmd: null, app: null });
}

export async function statAdvanced(path: string): Promise<StatInfo> {
    return fsStatAdvanced(path);
}

/** Đổi quyền file (chỉ ổ Local). Ném lỗi nếu backend từ chối. */
export async function chmod(path: string, mode: number): Promise<void> {
    return fsChmod(path, mode);
}

/** Đổi chủ sở hữu file (chỉ ổ Local, cần quyền root). */
export async function chown(path: string, uid: number, gid: number): Promise<void> {
    return fsChown(path, uid, gid);
}

export async function getAboutSpace(path: string): Promise<{ total?: number, used?: number, free?: number }> {
    let remote = path.split('::')[0] || 'Local';
    if (remote !== 'Local' && !remote.endsWith(':')) {
        remote += ':';
    }
    return await getAbout(remote);
}