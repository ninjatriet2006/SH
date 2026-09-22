/*
 * Hệ điều hành + quyền + clipboard + lệnh tự tạo — `backend/src/api/sys.rs`.
 * Chính sách quyền chỉ sống trong RAM (tắt app là mất, đúng an toàn).
 */

import { invoke } from './ipc';
import type {
  CustomAction,
  DesktopApp,
  OSClipboardData,
  OSClipboardItem,
  PermissionPolicy,
  SimpleFileItem,
} from './types';

/** Mở file bằng app (`exec`/`tenApp` để trống = mặc định hệ thống). */
export async function openWith(path: string, execCmd?: string | null, app?: string | null): Promise<void> {
  await invoke('sys_open_with', { path, exec_cmd: execCmd ?? null, app: app ?? null });
}

/** Danh sách app cho hộp "Open With" (mục đầu = mặc định hệ thống). */
export async function listApps(): Promise<DesktopApp[]> {
  try {
    return await invoke<DesktopApp[]>('sys_list_apps');
  } catch (error) {
    console.error('Lỗi sys_list_apps:', error);
    return [];
  }
}

/** Ghi clipboard (copy/cắt nhiều pane). */
export async function clipboardSet(items: OSClipboardItem[], isCut: boolean): Promise<void> {
  await invoke('os_clipboard_set', { items, is_cut: isCut });
}

/** Đọc clipboard. Chưa copy gì → null. */
export async function clipboardGet(): Promise<OSClipboardData | null> {
  try {
    return await invoke<OSClipboardData | null>('os_clipboard_get');
  } catch (error) {
    console.error('Lỗi os_clipboard_get:', error);
    return null;
  }
}

/** Danh sách lệnh tự tạo (hiện là stub rỗng, đọc config ở đợt sau). */
export async function getCustomActions(): Promise<CustomAction[]> {
  try {
    return await invoke<CustomAction[]>('sys_get_custom_actions');
  } catch (error) {
    console.error('Lỗi sys_get_custom_actions:', error);
    return [];
  }
}

/** Lọc lệnh tự tạo hợp lệ theo file đang chọn. */
export async function getValidActions(files: SimpleFileItem[]): Promise<CustomAction[]> {
  try {
    return await invoke<CustomAction[]>('sys_get_valid_actions', { files });
  } catch (error) {
    console.error('Lỗi sys_get_valid_actions:', error);
    return [];
  }
}

/** Chạy một lệnh tự tạo (`%f` trong mẫu = danh sách file đã quote). */
export async function executeCustomAction(execTemplate: string, basePath: string, fileNames: string[]): Promise<void> {
  await invoke('sys_execute_custom_action', {
    exec_template: execTemplate,
    base_path: basePath,
    file_names: fileNames,
  });
}

/** Chính sách quyền hiện tại của phiên (`deny`/`ask_once`/`allow_system`). */
export async function getPermissionPolicy(): Promise<PermissionPolicy> {
  return await invoke<PermissionPolicy>('get_permission_policy');
}

/** Đổi chính sách quyền (chỉ RAM, tắt app mất). */
export async function setPermissionPolicy(policy: PermissionPolicy): Promise<PermissionPolicy> {
  return await invoke<PermissionPolicy>('set_permission_policy', { policy });
}
