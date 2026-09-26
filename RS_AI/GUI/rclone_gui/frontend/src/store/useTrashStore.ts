/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả ổ Local và Remote đám mây.
- Trách nhiệm: Nạp danh sách, khôi phục mục đã xoá, xoá vĩnh viễn và làm rỗng thùng rác.
- Tương tác: Dùng `trash_bridge.ts`.
*/

import { create } from 'zustand';
import {
  deleteTrashLocal,
  deleteTrashRemote,
  emptyTrashLocal,
  emptyTrashRemote,
  listTrashLocal,
  listTrashRemote,
  restoreTrashLocal,
  restoreTrashRemote,
} from '../../../bridge/trash_bridge';
import type { FileItem, TrashItemLocal } from '../../../bridge/types';

interface TrashStore {
  localItems: TrashItemLocal[];
  remoteItems: FileItem[];
  selectedRemote: string;
  isLoading: boolean;

  setSelectedRemote: (remote: string) => void;
  loadLocalTrash: () => Promise<void>;
  restoreLocal: (id: string) => Promise<void>;
  deleteLocal: (id: string) => Promise<void>;
  emptyLocal: () => Promise<void>;
  loadRemoteTrash: (remote?: string) => Promise<void>;
  restoreRemote: (path: string) => Promise<void>;
  deleteRemote: (path: string) => Promise<void>;
  emptyRemote: () => Promise<void>;
}

export const useTrashStore = create<TrashStore>((set, get) => ({
  localItems: [],
  remoteItems: [],
  selectedRemote: '',
  isLoading: false,

  setSelectedRemote: (remote: string) => {
    set({ selectedRemote: remote });
    if (remote) {
      get().loadRemoteTrash(remote);
    }
  },

  loadLocalTrash: async () => {
    set({ isLoading: true });
    try {
      const items = await listTrashLocal();
      set({ localItems: items });
    } catch (err) {
      console.error('Lỗi loadLocalTrash:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  restoreLocal: async (id: string) => {
    await restoreTrashLocal(id);
    await get().loadLocalTrash();
  },

  deleteLocal: async (id: string) => {
    await deleteTrashLocal(id);
    await get().loadLocalTrash();
  },

  emptyLocal: async () => {
    await emptyTrashLocal();
    await get().loadLocalTrash();
  },

  loadRemoteTrash: async (remote?: string) => {
    const target = remote || get().selectedRemote;
    if (!target) return;
    set({ isLoading: true });
    try {
      const items = await listTrashRemote(target);
      set({ remoteItems: items });
    } catch (err) {
      console.error(`Lỗi loadRemoteTrash ${target}:`, err);
    } finally {
      set({ isLoading: false });
    }
  },

  restoreRemote: async (path: string) => {
    const target = get().selectedRemote;
    if (!target) return;
    await restoreTrashRemote(target, path);
    await get().loadRemoteTrash();
  },

  deleteRemote: async (path: string) => {
    const target = get().selectedRemote;
    if (!target) return;
    await deleteTrashRemote(target, path);
    await get().loadRemoteTrash();
  },

  emptyRemote: async () => {
    const target = get().selectedRemote;
    if (!target) return;
    await emptyTrashRemote(target);
    await get().loadRemoteTrash();
  },
}));
