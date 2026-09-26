/*
 * Lệnh file — tương ứng `backend/src/api/files_view.rs` + `files_edit.rs`.
 * Đọc/liệt kê (list/stat/search/thumbnail...) và tạo/sửa/xoá trực tiếp.
 * Copy/move/delete NẶNG đi qua hàng chờ (`jobs.ts`), không gọi trực tiếp ở đây.
 */

import { invoke } from './ipc';
import { jobEnqueue } from './jobs';
import type { ConflictInfo, FileItem, Job, SearchResultItem, StatInfo, UserPlace } from './types';

/** Liệt kê thư mục. `pane` để backend gắn watcher theo thư mục pane đang xem. */
export async function listFiles(path: string, pane?: 'left' | 'right'): Promise<FileItem[]> {
  try {
    return await invoke<FileItem[]>('list_files', { path, pane: pane ?? null });
  } catch (error) {
    console.error(`Lỗi list_files ${path}:`, error);
    return [];
  }
}

/** Kiểm tra xung đột trước khi chép nhiều nguồn vào đích. */
export async function checkConflicts(srcs: string[], destPath: string): Promise<ConflictInfo[]> {
  try {
    return await invoke<ConflictInfo[]>('fs_check_conflicts', { srcs, dest_path: destPath });
  } catch (error) {
    console.error('Lỗi fs_check_conflicts:', error);
    return [];
  }
}

/** Thống kê nâng cao (dung lượng, đếm file/dir, quyền). */
export async function statAdvanced(path: string): Promise<StatInfo | null> {
  try {
    return await invoke<StatInfo>('fs_stat_advanced', { path });
  } catch (error) {
    console.error(`Lỗi fs_stat_advanced ${path}:`, error);
    return null;
  }
}

/** Tìm kiếm theo tên trong cây thư mục. */
export async function searchFiles(path: string, query: string): Promise<SearchResultItem[]> {
  try {
    return await invoke<SearchResultItem[]>('fs_search', { path, query });
  } catch (error) {
    console.error('Lỗi fs_search:', error);
    return [];
  }
}

/** Thư mục nhà người dùng ($HOME). */
export async function getHomeDir(): Promise<string> {
  return await invoke<string>('get_home_dir');
}

/** Thư mục XDG chuẩn còn tồn tại (Home, Desktop, Downloads...). */
export async function getUserPlaces(): Promise<UserPlace[]> {
  try {
    return await invoke<UserPlace[]>('get_user_places');
  } catch (error) {
    console.error('Lỗi get_user_places:', error);
    return [];
  }
}

/** Mở terminal tại thư mục. */
export async function openInTerminal(path: string): Promise<void> {
  await invoke('open_in_terminal', { path });
}

/** Thumbnail (ảnh/video/pdf trong whitelist). Ngoài whitelist → null. */
export async function getThumbnail(path: string): Promise<string | null> {
  try {
    return await invoke<string | null>('fs_get_thumbnail', { path });
  } catch (error) {
    console.error(`Lỗi fs_get_thumbnail ${path}:`, error);
    return null;
  }
}

/** Thư mục tạm hệ điều hành. */
export async function getTempDir(): Promise<string> {
  return await invoke<string>('fs_temp_dir');
}

/** Tạo thư mục (qua hàng chờ Job Queue). */
export async function makeDir(path: string): Promise<Job> {
  return await jobEnqueue('mkdir', path);
}

/** Tạo file rỗng (qua hàng chờ Job Queue). */
export async function touchFile(path: string): Promise<Job> {
  return await jobEnqueue('touch', path);
}

/** Xoá vĩnh viễn (qua hàng chờ Job Queue, tôn trọng policy hiện tại). */
export async function deletePath(path: string): Promise<Job> {
  return await jobEnqueue('delete', path);
}

/** Đổi tên/di chuyển trong cùng chỗ (qua hàng chờ Job Queue). */
export async function renamePath(oldPath: string, newPath: string): Promise<Job> {
  return await jobEnqueue('rename', oldPath, newPath);
}

/** Đổi mode POSIX — chỉ ổ Local. */
export async function chmodPath(path: string, mode: number): Promise<void> {
  await invoke('fs_chmod', { path, mode });
}

/** Đổi chủ sở hữu — chỉ ổ Local (cần quyền, có thể hỏi consent). */
export async function chownPath(path: string, uid: number, gid: number): Promise<void> {
  await invoke('fs_chown', { path, uid, gid });
}
