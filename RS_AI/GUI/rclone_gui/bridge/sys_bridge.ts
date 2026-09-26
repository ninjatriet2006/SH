/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho tương tác hệ điều hành (Open with, clipboard, custom actions).
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng DesktopApp, OSClipboardData, OSClipboardItem, CustomAction, SimpleFileItem từ `types.ts`.
*/

import {
  invokeCommand,
  type CustomAction,
  type DesktopApp,
  type OSClipboardData,
  type OSClipboardItem,
  type SimpleFileItem,
} from './types';

/** Mở tệp tin bằng ứng dụng chỉ định hoặc ứng dụng mặc định của hệ thống. */
export async function sysOpenWith(path: string, execCmd?: string | null, app?: string | null): Promise<void> {
  await invokeCommand<void, { path: string; exec_cmd: string | null; app: string | null }>('sys_open_with', {
    path,
    exec_cmd: execCmd ?? null,
    app: app ?? null,
  });
}

/** Liệt kê danh sách các ứng dụng Desktop có cài đặt trong hệ thống. */
export async function sysListApps(): Promise<DesktopApp[]> {
  try {
    return await invokeCommand<DesktopApp[]>('sys_list_apps');
  } catch (error) {
    console.error('Lỗi sys_list_apps:', error);
    return [];
  }
}

/** Lưu danh sách mục được chọn vào clipboard nội bộ hệ thống (hỗ trợ copy / cut). */
export async function osClipboardSet(items: OSClipboardItem[], isCut: boolean): Promise<void> {
  await invokeCommand<void, { items: OSClipboardItem[]; is_cut: boolean }>('os_clipboard_set', {
    items,
    is_cut: isCut,
  });
}

/** Đọc trạng thái clipboard hiện tại. */
export async function osClipboardGet(): Promise<OSClipboardData | null> {
  try {
    return await invokeCommand<OSClipboardData | null>('os_clipboard_get');
  } catch (error) {
    console.error('Lỗi os_clipboard_get:', error);
    return null;
  }
}

/** Lấy toàn bộ danh sách các thao tác tuỳ chỉnh (Custom Actions) đã cấu hình. */
export async function sysGetCustomActions(): Promise<CustomAction[]> {
  try {
    return await invokeCommand<CustomAction[]>('sys_get_custom_actions');
  } catch (error) {
    console.error('Lỗi sys_get_custom_actions:', error);
    return [];
  }
}

/** Lấy danh sách thao tác tuỳ chỉnh hợp lệ dựa trên các file/folder đang được chọn. */
export async function sysGetValidActions(files: SimpleFileItem[]): Promise<CustomAction[]> {
  try {
    return await invokeCommand<CustomAction[], { files: SimpleFileItem[] }>('sys_get_valid_actions', {
      files,
    });
  } catch (error) {
    console.error('Lỗi sys_get_valid_actions:', error);
    return [];
  }
}

/** Thực thi một hành động tuỳ chỉnh với template và danh sách tệp tin. */
export async function sysExecuteCustomAction(
  execTemplate: string,
  basePath: string,
  fileNames: string[],
): Promise<void> {
  await invokeCommand<
    void,
    { exec_template: string; base_path: string; file_names: string[] }
  >('sys_execute_custom_action', {
    exec_template: execTemplate,
    base_path: basePath,
    file_names: fileNames,
  });
}
