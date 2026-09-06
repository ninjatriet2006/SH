/*
[INTEGRITY NOTES]
- Mục đích: Trạng thái danh sách provider, preset và kết quả kiểm tra kết nối.
- Trách nhiệm: Gọi bridge, giữ danh sách + map trạng thái. Sau MỌI thao tác ghi
  đều nạp lại từ backend — backend là nguồn sự thật, không tự đoán kết quả ở
  frontend rồi để state lệch với file trên đĩa.
- Tương tác: `pages/ProvidersPage.tsx`, bridge `provider_bridge.ts`.
*/

import { create } from 'zustand';
import type { ProviderView, PresetView, StatusView, SaveResult, ScannedModel, ModelCaps } from '../../../bridge/types';
import {
    listProviders, listPresets, saveProvider, deleteProvider, deleteProviders,
    testProvider, testAllProviders, scanProviderModels, setProviderModels,
} from '../../../bridge/provider_bridge';

interface ProviderState {
    providers: ProviderView[];
    presets: PresetView[];
    /** provider_id → trạng thái mới nhất. Thiếu = chưa kiểm tra. */
    statuses: Record<string, StatusView>;
    /** provider_id đang được kiểm tra (để hiện spinner từng dòng). */
    checking: Record<string, boolean>;
    isLoading: boolean;
    isTestingAll: boolean;

    fetchProviders: () => Promise<void>;
    fetchPresets: () => Promise<void>;
    save: (args: {
        providerId?: string;
        presetId: string;
        name: string;
        baseUrl: string;
        apiKey: string;
        forceOverwriteId?: string;
        npm?: string;
        customId?: string | null;
    }) => Promise<SaveResult>;
    remove: (providerId: string) => Promise<void>;
    removeMany: (providerIds: string[]) => Promise<number>;
    testOne: (providerId: string) => Promise<StatusView>;
    testAll: () => Promise<void>;
    scanModels: (providerId: string) => Promise<ScannedModel[]>;
    applyModels: (providerId: string, selected: string[], caps?: Record<string, ModelCaps>) => Promise<void>;
}

export const useProviderStore = create<ProviderState>((set, get) => ({
    providers: [],
    presets: [],
    statuses: {},
    checking: {},
    isLoading: false,
    isTestingAll: false,

    fetchProviders: async () => {
        set({ isLoading: true });
        try {
            set({ providers: await listProviders() });
        } catch (error) {
            console.error('Lỗi nạp danh sách provider:', error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    fetchPresets: async () => {
        try {
            set({ presets: await listPresets() });
        } catch (error) {
            console.error('Lỗi nạp danh sách preset:', error);
        }
    },

    save: async (args) => {
        const result = await saveProvider(args);
        // Chỉ nạp lại khi ĐÃ lưu thật. Trường hợp phát hiện trùng, backend chưa
        // ghi gì — nạp lại sẽ làm mất dữ liệu đang nhập trên form.
        if (result.saved_id) {
            await get().fetchProviders();
        }
        return result;
    },

    remove: async (providerId) => {
        await deleteProvider(providerId);
        // Dọn trạng thái cũ, nếu không dòng đã xoá vẫn để lại badge trong map.
        const { [providerId]: _removed, ...rest } = get().statuses;
        set({ statuses: rest });
        await get().fetchProviders();
    },

    removeMany: async (providerIds) => {
        const n = await deleteProviders(providerIds);
        const statuses = { ...get().statuses };
        for (const id of providerIds) delete statuses[id];
        set({ statuses });
        await get().fetchProviders();
        return n;
    },

    testOne: async (providerId) => {
        set({ checking: { ...get().checking, [providerId]: true } });
        try {
            const status = await testProvider(providerId);
            set({ statuses: { ...get().statuses, [providerId]: status } });
            return status;
        } finally {
            const { [providerId]: _done, ...rest } = get().checking;
            set({ checking: rest });
        }
    },

    testAll: async () => {
        set({ isTestingAll: true });
        try {
            const list = await testAllProviders();
            const statuses = { ...get().statuses };
            for (const s of list) statuses[s.provider_id] = s;
            set({ statuses });
        } finally {
            set({ isTestingAll: false });
        }
    },

    scanModels: async (providerId) => scanProviderModels(providerId),

    applyModels: async (providerId, selected, caps) => {
        await setProviderModels(providerId, selected, caps);
        await get().fetchProviders();
    },
}));
