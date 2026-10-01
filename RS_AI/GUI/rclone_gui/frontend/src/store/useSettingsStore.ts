/*
[INTEGRITY NOTES]
- Mục đích: Quản lý thiết lập Engine rclone, thông số Debug và các bản sao lưu Snapshot cấu hình.
- Trách nhiệm: Nạp và lưu cờ cấu hình engine, chẩn đoán, xem log theo yêu cầu (Pull model).
- Tương tác: Dùng `config_bridge.ts`.
*/

import { create } from 'zustand';
import {
  clearBackendLog,
  exportConfigRemote,
  getBackendLog,
  getDebugSettings,
  getEngineFlags,
  importConfigRemote,
  listConfigSnapshots,
  restoreConfigSnapshot,
  setDebugSettings,
  setEngineFlags,
} from '../../../bridge/config_bridge';
import type { DebugSettings, EngineSettings } from '../../../bridge/types';

interface SettingsStore {
  engineFlags: EngineSettings;
  debugSettings: DebugSettings;
  snapshots: string[];
  backendLog: string;
  isLoading: boolean;

  loadSettings: () => Promise<void>;
  updateEngineFlags: (flags: EngineSettings) => Promise<void>;
  updateDebugSettings: (settings: DebugSettings) => Promise<void>;
  fetchBackendLog: () => Promise<void>;
  clearLog: () => Promise<void>;
  restoreSnapshot: (name: string) => Promise<void>;
  exportRemote: (name: string) => Promise<string>;
  importRemote: (name: string, ini: string) => Promise<void>;
}

const defaultEngineFlags: EngineSettings = {
  transfers: 4,
  checkers: 8,
  queue_concurrency: 4,
  fast_list: false,
  server_side_across: false,
  dry_run: false,
  backup_dir: null,
  bulk_transfer: false,
};

const defaultDebugSettings: DebugSettings = {
  log_rotate_mb: 10,
};

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  engineFlags: defaultEngineFlags,
  debugSettings: defaultDebugSettings,
  snapshots: [],
  backendLog: '',
  isLoading: false,

  loadSettings: async () => {
    set({ isLoading: true });
    try {
      const [flags, debug, snaps] = await Promise.all([
        getEngineFlags().catch(() => defaultEngineFlags),
        getDebugSettings().catch(() => defaultDebugSettings),
        listConfigSnapshots().catch(() => []),
      ]);
      set({
        engineFlags: flags,
        debugSettings: debug,
        snapshots: snaps,
      });
    } catch (err) {
      console.error('Lỗi loadSettings:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  updateEngineFlags: async (flags: EngineSettings) => {
    const updated = await setEngineFlags(flags);
    set({ engineFlags: updated });
  },

  updateDebugSettings: async (settings: DebugSettings) => {
    const updated = await setDebugSettings(settings);
    set({ debugSettings: updated });
  },

  fetchBackendLog: async () => {
    const log = await getBackendLog();
    set({ backendLog: log });
  },

  clearLog: async () => {
    await clearBackendLog();
    set({ backendLog: '' });
  },

  restoreSnapshot: async (name: string) => {
    await restoreConfigSnapshot(name);
    await get().loadSettings();
  },

  exportRemote: async (name: string) => {
    return await exportConfigRemote(name);
  },

  importRemote: async (name: string, ini: string) => {
    await importConfigRemote(name, ini);
    await get().loadSettings();
  },
}));
