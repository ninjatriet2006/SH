/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho cấu hình rclone.conf và engine flags.
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng EngineSettings và DebugSettings từ `types.ts`.
*/

import { invokeCommand, type DebugSettings, type EngineSettings } from './types';

/** Đọc trực tiếp nội dung thô của file rclone.conf. */
export async function getConfigContent(): Promise<string> {
  return await invokeCommand<string>('get_config_content');
}

/** Lưu đè nội dung file rclone.conf. */
export async function setConfigContent(content: string): Promise<void> {
  await invokeCommand<void, { content: string }>('set_config_content', { content });
}

/** Đổi thứ tự các remote trong rclone.conf theo danh sách tên. */
export async function reorderConfig(names: string[]): Promise<void> {
  await invokeCommand<void, { names: string[] }>('reorder_config', { names });
}

/** Liệt kê danh sách các bản sao lưu (snapshot) rclone.conf. */
export async function listConfigSnapshots(): Promise<string[]> {
  try {
    return await invokeCommand<string[]>('list_config_snapshots');
  } catch (error) {
    console.error('Lỗi list_config_snapshots:', error);
    return [];
  }
}

/** Khôi phục cấu hình từ một bản sao lưu snapshot cụ thể. */
export async function restoreConfigSnapshot(name: string): Promise<void> {
  await invokeCommand<void, { name: string }>('restore_config_snapshot', { name });
}

/** Xuất khối cấu hình INI của một remote cụ thể. */
export async function exportConfigRemote(name: string): Promise<string> {
  return await invokeCommand<string, { name: string }>('export_config_remote', { name });
}

/** Nhập một remote từ chuỗi INI vào rclone.conf. */
export async function importConfigRemote(name: string, ini: string): Promise<void> {
  await invokeCommand<void, { name: string; ini: string }>('import_config_remote', { name, ini });
}

/** Lấy các cờ tuỳ chỉnh hiệu năng rclone engine (transfers, checkers, fast-list...). */
export async function getEngineFlags(): Promise<EngineSettings> {
  return await invokeCommand<EngineSettings>('get_engine_flags');
}

/** Lưu các cờ tuỳ chỉnh hiệu năng rclone engine. */
export async function setEngineFlags(flags: EngineSettings): Promise<EngineSettings> {
  return await invokeCommand<EngineSettings, { flags: EngineSettings }>('set_engine_flags', { flags });
}

/** Lấy cấu hình chẩn đoán và xoay vòng log. */
export async function getDebugSettings(): Promise<DebugSettings> {
  return await invokeCommand<DebugSettings>('get_debug_settings');
}

/** Lưu cấu hình chẩn đoán và xoay vòng log. */
export async function setDebugSettings(settings: DebugSettings): Promise<DebugSettings> {
  return await invokeCommand<DebugSettings, { settings: DebugSettings }>('set_debug_settings', { settings });
}

/**
 * Đọc nội dung file backend.log theo yêu cầu (Pull-on-demand).
 * Không sử dụng luồng sự kiện (event stream) trực tiếp nhằm bảo vệ tài nguyên frontend.
 */
export async function getBackendLog(): Promise<string> {
  try {
    return await invokeCommand<string>('get_backend_log');
  } catch (error) {
    console.error('Lỗi get_backend_log:', error);
    return '';
  }
}
