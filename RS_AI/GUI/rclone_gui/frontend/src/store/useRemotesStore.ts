/*
[INTEGRITY NOTES]
- Mục đích: Quản lý danh sách, cấu hình, và thông số chi tiết của Remote Cloud.
- Trách nhiệm: Nạp danh sách remotes, danh sách providers, kiểm tra năng lực backend và dung lượng rclone.
- Tương tác: Dùng `remote_bridge.ts`.
*/

import { create } from 'zustand';
import {
  checkFilesIntegrity,
  createRemote as apiCreateRemote,
  deleteRemote as apiDeleteRemote,
  getAbout,
  getFeatureFlags,
  getProviders,
  getSize,
  listRemotes,
  updateRemote as apiUpdateRemote,
} from '../../../bridge/remote_bridge';
import type {
  AboutInfo,
  FeatureFlags,
  IntegrityCheckResult,
  ProviderInfo,
  RemoteConfig,
  SizeInfo,
} from '../../../bridge/types';

interface RemotesStore {
  remotes: RemoteConfig[];
  providers: ProviderInfo[];
  selectedRemote: string | null;
  featuresMap: Record<string, FeatureFlags | null>;
  aboutMap: Record<string, AboutInfo>;
  sizeMap: Record<string, SizeInfo>;
  isLoading: boolean;

  loadRemotes: () => Promise<void>;
  loadProviders: () => Promise<void>;
  setSelectedRemote: (name: string | null) => void;
  fetchRemoteDetails: (name: string) => Promise<void>;
  createRemote: (name: string, provider: string, options: Record<string, string>) => Promise<void>;
  updateRemote: (name: string, options: Record<string, string>) => Promise<void>;
  deleteRemote: (name: string) => Promise<void>;
  checkIntegrity: (src: string, dst: string) => Promise<IntegrityCheckResult>;
}

export const useRemotesStore = create<RemotesStore>((set, get) => ({
  remotes: [],
  providers: [],
  selectedRemote: null,
  featuresMap: {},
  aboutMap: {},
  sizeMap: {},
  isLoading: false,

  loadRemotes: async () => {
    set({ isLoading: true });
    try {
      const list = await listRemotes();
      set({ remotes: list });
    } catch (err) {
      console.error('Lỗi nạp danh sách remotes:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  loadProviders: async () => {
    try {
      const list = await getProviders();
      set({ providers: list });
    } catch (err) {
      console.error('Lỗi nạp danh sách providers:', err);
    }
  },

  setSelectedRemote: (name: string | null) => {
    set({ selectedRemote: name });
    if (name) {
      get().fetchRemoteDetails(name);
    }
  },

  fetchRemoteDetails: async (name: string) => {
    const remoteTarget = name.endsWith(':') ? name : `${name}:`;
    try {
      const [feat, abt, sz] = await Promise.all([
        getFeatureFlags(remoteTarget).catch(() => null),
        getAbout(remoteTarget).catch(() => ({})),
        getSize(remoteTarget).catch(() => ({})),
      ]);

      set((state) => ({
        featuresMap: { ...state.featuresMap, [name]: feat },
        aboutMap: { ...state.aboutMap, [name]: abt },
        sizeMap: { ...state.sizeMap, [name]: sz },
      }));
    } catch (err) {
      console.error(`Lỗi nạp chi tiết remote ${name}:`, err);
    }
  },

  createRemote: async (name: string, provider: string, options: Record<string, string>) => {
    await apiCreateRemote(name, provider, options);
    await get().loadRemotes();
  },

  updateRemote: async (name: string, options: Record<string, string>) => {
    await apiUpdateRemote(name, options);
    await get().loadRemotes();
  },

  deleteRemote: async (name: string) => {
    await apiDeleteRemote(name);
    set((state) => {
      const next = { ...state.featuresMap };
      delete next[name];
      return {
        featuresMap: next,
        selectedRemote: state.selectedRemote === name ? null : state.selectedRemote,
      };
    });
    await get().loadRemotes();
  },

  checkIntegrity: async (src: string, dst: string) => {
    return await checkFilesIntegrity(src, dst);
  },
}));
