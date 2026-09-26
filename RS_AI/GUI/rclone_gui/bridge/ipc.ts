/*
 * Tầng kết nối IPC Tauri v2 trực tiếp (bare-core).
 * Gọi thẳng tauri::command và nghe tauri::event không qua bao thư bọc giả.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';

export type { EventCallback, UnlistenFn };

/**
 * Gọi lệnh Tauri Command trực tiếp.
 */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return await tauriInvoke<T>(cmd, args);
}

/**
 * Lắng nghe sự kiện phát ra từ Tauri Backend.
 */
export async function listen<T>(event: string, handler: EventCallback<T>): Promise<UnlistenFn> {
  return await tauriListen<T>(event, handler);
}

/**
 * Trả về dữ liệu trực tiếp (dành cho tương thích các test bare-core).
 */
export function unwrap<T>(data: T): T {
  return data;
}
