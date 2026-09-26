/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho tác vụ File (View, Stat, Edit, Permissions).
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng các interface trong `types.ts`, phối hợp cùng `jobs_bridge.ts` cho các thao tác nặng.
*/

import { jobEnqueue } from './jobs_bridge';
import {
  invokeCommand,
  type ConflictInfo,
  type FileItem,
  type Job,
  type PermissionPolicy,
  type SearchResultItem,
  type StatInfo,
  type UserPlace,
} from './types';

/** Liệt kê thư mục. `pane` để backend gắn watcher theo thư mục pane đang xem. */
export async function listFiles(path: string, pane?: string | null): Promise<FileItem[]> {
  try {
    return await invokeCommand<FileItem[], { path: string; pane: string | null }>('list_files', {
      path,
      pane: pane ?? null,
    });
  } catch (error) {
    console.error(`Lỗi list_files ${path}:`, error);
    return [];
  }
}

/** Kiểm tra xung đột trước khi chép nhiều nguồn vào đích. */
export async function checkConflicts(srcs: string[], destPath: string): Promise<ConflictInfo[]> {
  try {
    return await invokeCommand<ConflictInfo[], { srcs: VecString; dest_path: string }>(
      'fs_check_conflicts',
      { srcs, dest_path: destPath },
    );
  } catch (error) {
    console.error('Lỗi fs_check_conflicts:', error);
    return [];
  }
}

type VecString = string[];

/** Kiểm tra xung đột trước khi đổi tên đường dẫn. */
export async function checkRenameConflict(oldPath: string, newPath: string): Promise<ConflictInfo | null> {
  try {
    return await invokeCommand<ConflictInfo | null, { old_path: string; new_path: string }>(
      'fs_check_rename_conflict',
      { old_path: oldPath, new_path: newPath },
    );
  } catch (error) {
    console.error('Lỗi fs_check_rename_conflict:', error);
    return null;
  }
}

/** Thống kê nâng cao (dung lượng, đếm file/dir, quyền). */
export async function statAdvanced(path: string): Promise<StatInfo | null> {
  try {
    return await invokeCommand<StatInfo, { path: string }>('fs_stat_advanced', { path });
  } catch (error) {
    console.error(`Lỗi fs_stat_advanced ${path}:`, error);
    return null;
  }
}

/** Tìm kiếm theo tên trong cây thư mục. */
export async function searchFiles(path: string, query: string): Promise<SearchResultItem[]> {
  try {
    return await invokeCommand<SearchResultItem[], { path: string; query: string }>('fs_search', {
      path,
      query,
    });
  } catch (error) {
    console.error('Lỗi fs_search:', error);
    return [];
  }
}

/** Thư mục nhà người dùng ($HOME). */
export async function getHomeDir(): Promise<string> {
  return await invokeCommand<string>('get_home_dir');
}

/** Thư mục XDG chuẩn còn tồn tại (Home, Desktop, Downloads...). */
export async function getUserPlaces(): Promise<UserPlace[]> {
  try {
    return await invokeCommand<UserPlace[]>('get_user_places');
  } catch (error) {
    console.error('Lỗi get_user_places:', error);
    return [];
  }
}

/** Mở terminal tại thư mục. */
export async function openInTerminal(path: string): Promise<void> {
  await invokeCommand<void, { path: string }>('open_in_terminal', { path });
}

/** Thumbnail (ảnh/video/pdf trong whitelist). Ngoài whitelist → null. */
export async function getThumbnail(path: string): Promise<string | null> {
  try {
    return await invokeCommand<string | null, { path: string }>('fs_get_thumbnail', { path });
  } catch (error) {
    console.error(`Lỗi fs_get_thumbnail ${path}:`, error);
    return null;
  }
}

/** Thư mục tạm hệ điều hành. */
export async function getTempDir(): Promise<string> {
  return await invokeCommand<string>('fs_temp_dir');
}

/** Đổi mode POSIX — chỉ ổ Local. */
export async function chmodPath(path: string, mode: number): Promise<void> {
  await invokeCommand<void, { path: string; mode: number }>('fs_chmod', { path, mode });
}

/** Đổi chủ sở hữu — chỉ ổ Local (cần quyền root). */
export async function chownPath(path: string, uid: number, gid: number): Promise<void> {
  await invokeCommand<void, { path: string; uid: number; gid: number }>('fs_chown', {
    path,
    uid,
    gid,
  });
}

/** Chính sách quyền hiện tại của phiên (`deny`/`ask_once`/`allow_system`). */
export async function getPermissionPolicy(): Promise<PermissionPolicy> {
  return (await invokeCommand<string>('get_permission_policy')) as PermissionPolicy;
}

/** Đổi chính sách quyền (chỉ RAM, tắt app mất). */
export async function setPermissionPolicy(policy: PermissionPolicy): Promise<PermissionPolicy> {
  return (await invokeCommand<string, { policy: string }>('set_permission_policy', {
    policy,
  })) as PermissionPolicy;
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

/** Nhận diện lỗi cần cấp quyền (backend trả marker `PERMISSION_CONSENT`). */
export function isPermissionConsentError(e: unknown): boolean {
  try {
    return JSON.stringify(e ?? '').includes('PERMISSION_CONSENT');
  } catch {
    return String(e ?? '').includes('PERMISSION_CONSENT');
  }
}
