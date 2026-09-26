/*
[INTEGRITY NOTES]
- Mục đích: Quản lý danh sách và trạng thái các dịch vụ Mount Systemd của rclone.
- Trách nhiệm: Nạp danh sách service, kiểm tra FUSE, kích hoạt các lệnh start/stop/enable/disable/restart.
- Tương tác: Dùng `mount_bridge.ts`.
*/

import { create } from 'zustand';
import {
  checkFuseInstalled,
  createMountService,
  deleteMountService,
  getMountServiceConfig,
  listMountServices,
  manageMountService,
} from '../../../bridge/mount_bridge';
import type { MountConfig, SystemdServiceInfo } from '../../../bridge/types';

interface MountsStore {
  mounts: SystemdServiceInfo[];
  fuseInstalled: boolean;
  isLoading: boolean;

  loadMounts: () => Promise<void>;
  createMount: (config: MountConfig, confirmed?: boolean) => Promise<string>;
  deleteMount: (serviceName: string, isUser: boolean, confirmed?: boolean) => Promise<string>;
  manageMount: (
    serviceName: string,
    isUser: boolean,
    action: 'start' | 'stop' | 'enable' | 'disable' | 'restart',
    confirmed?: boolean,
  ) => Promise<string>;
  getServiceConfig: (serviceName: string, isUser: boolean) => Promise<MountConfig | null>;
}

export const useMountsStore = create<MountsStore>((set, get) => ({
  mounts: [],
  fuseInstalled: true,
  isLoading: false,

  loadMounts: async () => {
    set({ isLoading: true });
    try {
      const [services, isFuse] = await Promise.all([
        listMountServices().catch(() => []),
        checkFuseInstalled().catch(() => false),
      ]);
      set({ mounts: services, fuseInstalled: isFuse });
    } catch (err) {
      console.error('Lỗi loadMounts:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  createMount: async (config: MountConfig, confirmed = false) => {
    const res = await createMountService(config, confirmed);
    await get().loadMounts();
    return res;
  },

  deleteMount: async (serviceName: string, isUser: boolean, confirmed = false) => {
    const res = await deleteMountService(serviceName, isUser, confirmed);
    await get().loadMounts();
    return res;
  },

  manageMount: async (
    serviceName: string,
    isUser: boolean,
    action: 'start' | 'stop' | 'enable' | 'disable' | 'restart',
    confirmed = false,
  ) => {
    const res = await manageMountService(serviceName, isUser, action, confirmed);
    await get().loadMounts();
    return res;
  },

  getServiceConfig: async (serviceName: string, isUser: boolean) => {
    return await getMountServiceConfig(serviceName, isUser);
  },
}));
