/*
 * Cấu hình rclone + engine + chẩn đoán — `backend/src/api/config.rs`.
 * Engine/Debug/Appearance lưu JSON trong thư mục config app (bền qua phiên).
 * Riêng chính sách quyền KHÔNG ở đây (chỉ RAM, xem `sys.ts`).
 */

import { invoke } from './ipc';
import type { AppearanceSettings, DebugSettings, EngineSettings } from './types';

/** Đọc toàn bộ `rclone.conf`. */
export async function getConfigContent(): Promise<string> {
  return await invoke<string>('get_config_content');
}

/** Ghi đè config (backend tự snapshot bản cũ, giữ tối đa 10). */
export async function setConfigContent(content: string): Promise<void> {
  await invoke('set_config_content', { content });
}

/** Sắp xếp lại thứ tự remote. */
export async function reorderConfig(names: string[]): Promise<void> {
  await invoke('reorder_config', { names });
}

/** Liệt kê snapshot config (cũ → mới). */
export async function listConfigSnapshots(): Promise<string[]> {
  try {
    return await invoke<string[]>('list_config_snapshots');
  } catch (error) {
    console.error('Lỗi list_config_snapshots:', error);
    return [];
  }
}

/** Khôi phục một snapshot đè lên config hiện tại. */
export async function restoreConfigSnapshot(name: string): Promise<void> {
  await invoke('restore_config_snapshot', { name });
}

/** Xuất một remote thành đoạn INI. */
export async function exportConfigRemote(name: string): Promise<string> {
  return await invoke<string>('export_config_remote', { name });
}

/** Nhập thêm một remote từ đoạn INI (trùng tên thì lỗi, không ghi đè). */
export async function importConfigRemote(name: string, ini: string): Promise<void> {
  await invoke('import_config_remote', { name, ini });
}

/** Đọc cấu hình engine (thiếu tệp → default). */
export async function getEngineSettings(): Promise<EngineSettings> {
  return await invoke<EngineSettings>('get_engine_flags');
}

/** Lưu cấu hình engine (backend validate transfers/checkers/backup_dir). */
export async function setEngineSettings(flags: EngineSettings): Promise<EngineSettings> {
  return await invoke<EngineSettings>('set_engine_flags', { flags });
}

/** Đọc cấu hình chẩn đoán (ngưỡng dọn log...). */
export async function getDebugSettings(): Promise<DebugSettings> {
  return await invoke<DebugSettings>('get_debug_settings');
}

/** Lưu cấu hình chẩn đoán (backend chặn log_rotate_mb ngoài 1..=500). */
export async function setDebugSettings(settings: DebugSettings): Promise<DebugSettings> {
  return await invoke<DebugSettings>('set_debug_settings', { settings });
}

/** Đọc lịch sử `backend.log` (phần đã ghi trước khi UI bật nghe event). */
export async function getBackendLog(): Promise<string> {
  try {
    return await invoke<string>('get_backend_log');
  } catch (error) {
    console.error('Lỗi get_backend_log:', error);
    return '';
  }
}

/** Đọc lựa chọn giao diện đã lưu (rỗng = chưa chọn, UI tự quyết mặc định). */
export async function getAppearance(): Promise<AppearanceSettings> {
  return await invoke<AppearanceSettings>('get_appearance');
}

/** Lưu lựa chọn giao diện (lang/theme/font + thư mục nguồn). */
export async function setAppearance(settings: AppearanceSettings): Promise<AppearanceSettings> {
  return await invoke<AppearanceSettings>('set_appearance', { settings });
}
