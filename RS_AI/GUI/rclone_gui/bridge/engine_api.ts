/*
[INTEGRITY NOTES]
Mục đích: API bridge cho cụm Engine (GlobalFlags) — S2 wire qua IPC.
Trách nhiệm: get/set cờ transfer toàn cục (transfers/checkers/fast-list/across/dry-run/backup-dir).
Tương tác: backend `settings::engine` qua `bridge/ipc.ts`, UI settings trong `frontend/src/main.ts`.
*/

import { invoke } from './ipc';

/** UNIVERSAL: cờ engine dùng chung Local + mọi backend cloud. */
export interface EngineFlags {
  transfers: number;
  checkers: number;
  fast_list: boolean;
  server_side_across: boolean;
  dry_run: boolean;
  backup_dir: string | null;
}

/** UNIVERSAL: đọc cờ hiện tại (thiếu tệp → default 4/8, các cờ tắt). */
export async function getEngineFlags(): Promise<EngineFlags> {
  return invoke<EngineFlags>('get_engine_flags');
}

/** UNIVERSAL: lưu cờ đã validate (backend chặn transfers/checkers/backup_dir rỗng). */
export async function setEngineFlags(flags: EngineFlags): Promise<EngineFlags> {
  return invoke<EngineFlags>('set_engine_flags', { flags });
}
