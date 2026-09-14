/*
[INTEGRITY NOTES]
 - Mục đích: Trạng thái sở thích hiển thị provider — sao yêu thích (đưa lên đầu
   bảng, đổi thứ tự trong nhóm sao) và model ghim theo provider.
- Trách nhiệm: Gọi bridge, giữ danh sách sao + map model ghim. Backend là nguồn
  sự thật: mọi thao tác đều lấy preferences MỚI từ phản hồi lệnh ghi rồi đặt
  lại state, không tự đoán kết quả ở frontend.
- Tương tác: `pages/ProvidersPage.tsx`, bridge `settings_bridge.ts`,
  backend `api/settings.rs` (lưu tại `~/.config/opencode-manager/settings.json`).
*/

import { create } from 'zustand';
import {
    getProviderPreferences, toggleFavoriteProvider, reorderFavoriteProviders, setPinnedModel,
    toggleTrackedProvider, untrackProvider,
} from '../../../bridge/settings_bridge';

interface PreferencesState {
    /** Danh sách provider có sao — thứ tự mảng = thứ tự hiển thị nhóm sao. */
    favorites: string[];
    /** provider_id → model ghim. */
    pinnedModels: Record<string, string>;
    trackedProviders: string[];
    isLoaded: boolean;

    fetchPreferences: () => Promise<void>;
    /** Bật/tắt sao; NÉM lỗi để trang hiện báo cáo thay vì nuốt mất. */
    toggleFavorite: (providerId: string) => Promise<void>;
    /** Đổi thứ tự trong nhóm sao; NÉM lỗi nếu backend từ chối hoán vị lệch. */
    reorderFavorites: (providerIds: string[]) => Promise<void>;
    /** Ghim model cho provider; modelId rỗng = bỏ ghim. */
    pinModel: (providerId: string, modelId: string) => Promise<void>;
    toggleTracking: (providerId: string) => Promise<void>;
    untrack: (providerId: string) => Promise<void>;
}

function apply(set: (partial: Partial<PreferencesState>) => void, prefs: {
    favorite_providers: string[];
    pinned_models: Record<string, string>;
    tracked_providers: string[];
}) {
    set({ favorites: prefs.favorite_providers, pinnedModels: prefs.pinned_models, trackedProviders: prefs.tracked_providers, isLoaded: true });
}

export const usePreferencesStore = create<PreferencesState>((set) => ({
    favorites: [],
    pinnedModels: {},
    trackedProviders: [],
    isLoaded: false,

    fetchPreferences: async () => {
        apply(set, await getProviderPreferences());
    },

    toggleFavorite: async (providerId) => {
        apply(set, await toggleFavoriteProvider(providerId));
    },

    reorderFavorites: async (providerIds) => {
        apply(set, await reorderFavoriteProviders(providerIds));
    },

    pinModel: async (providerId, modelId) => {
        apply(set, await setPinnedModel(providerId, modelId));
    },
    toggleTracking: async (providerId) => {
        apply(set, await toggleTrackedProvider(providerId));
    },
    untrack: async (providerId) => {
        apply(set, await untrackProvider(providerId));
    },
}));
