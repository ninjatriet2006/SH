/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái bộ nhớ đệm (Store) cho danh sách Đăng ký dịch vụ (Subscription) của một người dùng.
- Trách nhiệm: Nạp và lưu trữ các Subscription, gán gói mới, hoặc thu hồi gói.
- Tương tác: Kết nối với bridge `subscription_bridge` và sử dụng trong component hiển thị chi tiết người dùng.
*/

import { create } from 'zustand';
import type { Subscription } from '../../../bridge/types';
import { 
    listUserSubscriptions, 
    addSubscriptionToUser, 
    updateSubscriptionExpiry, 
    removeSubscriptionFromUser,
    checkSubscriptionStatus,
    listAllSubscriptions
} from '../../../bridge/subscription_bridge';

interface SubscriptionState {
    // Mảng chứa các gói đăng ký của user hiện tại
    subscriptions: Subscription[];
    // Mảng chứa TOÀN BỘ gói đăng ký trong hệ thống
    allSubscriptions: Subscription[];
    // User ID đang được chọn để xem
    currentUserId: string | null;
    isLoading: boolean;
    
    // Nạp toàn bộ
    fetchAllSubscriptions: () => Promise<void>;
    // Nạp danh sách theo user_id
    fetchUserSubscriptions: (userId: string) => Promise<void>;
    // Gán gói mới
    addSubscription: (userId: string, packageId: string, customExpiry?: number, amount?: number) => Promise<void>;
    // Cập nhật ngày hết hạn
    updateExpiry: (subId: string, newExpiry: number, amount?: number) => Promise<void>;
    // Thu hồi gói
    removeSubscription: (subId: string) => Promise<void>;
    // Làm mới trạng thái của 1 gói
    refreshStatus: (subId: string) => Promise<void>;
}

export const useSubscriptionStore = create<SubscriptionState>((set, get) => ({
    subscriptions: [],
    allSubscriptions: [],
    currentUserId: null,
    isLoading: false,

    fetchAllSubscriptions: async () => {
        set({ isLoading: true });
        try {
            const data = await listAllSubscriptions();
            set({ allSubscriptions: data });
        } catch (error) {
            console.error("Lỗi lấy toàn bộ đăng ký:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    fetchUserSubscriptions: async (userId) => {
        set({ isLoading: true, currentUserId: userId });
        try {
            const data = await listUserSubscriptions(userId);
            set({ subscriptions: data });
        } catch (error) {
            console.error("Lỗi lấy danh sách đăng ký:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    addSubscription: async (userId, packageId, customExpiry, amount) => {
        set({ isLoading: true });
        try {
            const newSub = await addSubscriptionToUser(userId, packageId, customExpiry, amount);
            // Cập nhật state nếu đang xem đúng user đó
            if (get().currentUserId === userId) {
                set({ subscriptions: [...get().subscriptions, newSub] });
            }
            set({ allSubscriptions: [...get().allSubscriptions, newSub] });
        } catch (error) {
            console.error("Lỗi gán gói đăng ký:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    updateExpiry: async (subId, newExpiry, amount) => {
        set({ isLoading: true });
        try {
            const updatedSub = await updateSubscriptionExpiry(subId, newExpiry, amount);
            set({ subscriptions: get().subscriptions.map(s => s.id === subId ? updatedSub : s) });
            set({ allSubscriptions: get().allSubscriptions.map(s => s.id === subId ? updatedSub : s) });
        } catch (error) {
            console.error("Lỗi gia hạn gói:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    removeSubscription: async (subId) => {
        set({ isLoading: true });
        try {
            await removeSubscriptionFromUser(subId);
            set({ subscriptions: get().subscriptions.filter(s => s.id !== subId) });
            set({ allSubscriptions: get().allSubscriptions.filter(s => s.id !== subId) });
        } catch (error) {
            console.error("Lỗi xóa gói:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    refreshStatus: async (subId) => {
        try {
            // Gọi kiểm tra trạng thái
            const isActive = await checkSubscriptionStatus(subId);
            // Sửa đổi trạng thái trên giao diện ngay lập tức
            set({ 
                subscriptions: get().subscriptions.map(s => 
                    s.id === subId ? { ...s, is_active: isActive } : s
                ),
                allSubscriptions: get().allSubscriptions.map(s => 
                    s.id === subId ? { ...s, is_active: isActive } : s
                )
            });
        } catch (error) {
            console.error("Lỗi kiểm tra trạng thái:", error);
        }
    }
}));
