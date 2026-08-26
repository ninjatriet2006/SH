import { invoke } from '@tauri-apps/api/core';
import { fsMkdir, fsDelete, fsRename, fsCopy, fsMove, listFiles, fsSearch, fsStatAdvanced } from '../../../bridge/explorer_api.ts';
import { getAbout } from '../../../bridge/remote_api.ts';
import type { StatInfo, SearchResultItem } from '../../../bridge/explorer_api.ts';
import type { FileItem } from '../store';

export interface SearchOptions {
  fuzzy: boolean;
  content_query?: string | null;
  min_size?: number | null;
  max_size?: number | null;
}

export type SearchResult = SearchResultItem;
export type { StatInfo };

export async function listLocal(path: string): Promise<FileItem[]> {
    const { remote, realPath } = parseRemotePath(path);
    return listFiles(remote, realPath);
}

export async function searchLocal(path: string, query: string, _options?: SearchOptions): Promise<SearchResult[]> {
    const { remote, realPath } = parseRemotePath(path);
    return fsSearch(remote, realPath, query);
}

/** Tách chuỗi GDrive::/Documents thành remote và path thực tế */
export function parseRemotePath(fullPath: string): { remote: string, realPath: string } {
    let remote = 'Local';
    let realPath = fullPath;
    if (fullPath.includes('::')) {
        const parts = fullPath.split('::');
        remote = parts[0];
        realPath = parts.slice(1).join('::');
    } else if (!fullPath || fullPath === '/') {
        remote = '';
    }
    return { remote, realPath };
}

export async function mkdir(path: string, _account?: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(path);
    await fsMkdir(remote, realPath);
}

export async function mkdirLocal(path: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(path);
    await runWithSudoFallback('mkdir', [realPath], remote, () => fsMkdir(remote, realPath));
}

export async function remove(path: string, _account?: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(path);
    await fsDelete(remote, realPath);
}

export async function rmLocal(path: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(path);
    await runWithSudoFallback('rm', [realPath], remote, () => fsDelete(remote, realPath));
}

export async function rename(path: string, newName: string, _account?: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(path);
    const parentDir = realPath.substring(0, realPath.lastIndexOf('/'));
    const newPath = parentDir ? `${parentDir}/${newName}` : `/${newName}`;
    await fsRename(remote, realPath, newPath);
}

export async function renameLocal(oldPath: string, newName: string): Promise<void> {
    const { remote, realPath } = parseRemotePath(oldPath);
    const parent = realPath.substring(0, realPath.lastIndexOf('/'));
    const newPath = parent ? `${parent}/${newName}` : `/${newName}`;
    await runWithSudoFallback('mv', [realPath, newPath], remote, () => fsRename(remote, realPath, newPath));
}

export async function copy(src: string, dest: string, _account?: string): Promise<void> {
    const srcParsed = parseRemotePath(src);
    const destParsed = parseRemotePath(dest);
    await fsCopy(srcParsed.remote, srcParsed.realPath, destParsed.remote, destParsed.realPath);
}

export async function move(src: string, dest: string, _account?: string): Promise<void> {
    const srcParsed = parseRemotePath(src);
    const destParsed = parseRemotePath(dest);
    await fsMove(srcParsed.remote, srcParsed.realPath, destParsed.remote, destParsed.realPath);
}

export async function copyLocal(from: string, to: string, _overwrite: boolean = false): Promise<void> {
    const pFrom = parseRemotePath(from);
    const pTo = parseRemotePath(to);
    await runWithSudoFallback('cp', [pFrom.realPath, pTo.realPath], pFrom.remote, () => fsCopy(pFrom.remote, pFrom.realPath, pTo.remote, pTo.realPath));
}

export async function moveLocal(from: string, to: string): Promise<void> {
    const pFrom = parseRemotePath(from);
    const pTo = parseRemotePath(to);
    await runWithSudoFallback('mv', [pFrom.realPath, pTo.realPath], pFrom.remote, () => fsMove(pFrom.remote, pFrom.realPath, pTo.remote, pTo.realPath));
}

async function runWithSudoFallback<T>(action: string, args: string[], remote: string, fn: () => Promise<T>): Promise<T> {
    try {
        return await fn();
    } catch (e: any) {
        const errStr = String(e).toLowerCase();
        if ((errStr.includes('permission denied') || errStr.includes('access is denied') || errStr.includes('os error 13')) && remote === 'Local') {
            if (confirm(`Lỗi phân quyền (Permission Denied).\nBạn có muốn thử lại thao tác này với quyền quản trị viên (Root/Admin) không?`)) {
                await invoke('fs_sudo_exec', { action, args });
                return undefined as T;
            }
        }
        throw e;
    }
}



export async function cpLocal(from: string, to: string, _overwrite = true): Promise<void> {
    return copyLocal(from, to, _overwrite);
}

export async function upload(local: string, remoteTarget: string, _account?: string): Promise<void> {
    return copy(local, remoteTarget);
}

export async function download(remoteSource: string, local: string, _account?: string): Promise<void> {
    return copy(remoteSource, local);
}

export async function cpBatch(srcs: string[], dstDir: string, _overwrite = true): Promise<void> {
    for (const src of srcs) {
        const name = src.substring(src.lastIndexOf('/') + 1);
        const dst = dstDir.endsWith('/') ? `${dstDir}${name}` : `${dstDir}/${name}`;
        await copy(src, dst);
    }
}

// Các tính năng đọc/mở file tạm thời stub vì rclone_gui hiện chỉ quản lý file
export async function cat(_path: string, _account?: string): Promise<string> {
    console.warn("cat not implemented for rclone yet");
    return "";
}

export async function write(_path: string, _content: string, _account?: string): Promise<void> {
    console.warn("write not implemented for rclone yet");
}

export async function writeLocal(_path: string, _content: string): Promise<void> {
    console.warn("writeLocal not implemented for rclone yet");
}

export async function open(_path: string): Promise<void> {
    console.warn("open not implemented for rclone yet");
}

// Removed duplicate StatInfo interface
export async function statAdvanced(path: string): Promise<StatInfo> {
    const { remote, realPath } = parseRemotePath(path);
    return fsStatAdvanced(remote, realPath);
}
export async function chmod(_path: string, _mode: number): Promise<void> {}
export async function chown(_path: string, _uid: number, _gid: number): Promise<void> {}
export async function getFreeSpace(path: string): Promise<number> {
    const { remote } = parseRemotePath(path);
    if (!remote) return 0;
    
    let remoteName = remote;
    if (remoteName !== 'Local' && !remoteName.endsWith(':')) {
        remoteName += ':';
    }
    
    const about = await getAbout(remoteName);
    return about.free || 0;
}
export async function getAboutSpace(path: string): Promise<{ total?: number, used?: number, free?: number }> {
    const { remote } = parseRemotePath(path);
    if (!remote) return {};
    
    let remoteName = remote;
    if (remoteName !== 'Local' && !remoteName.endsWith(':')) {
        remoteName += ':';
    }
    
    return await getAbout(remoteName);
}