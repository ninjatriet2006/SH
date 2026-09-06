/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái bộ nhớ đệm (Store) cho danh sách Đăng ký dịch vụ (Subscription) của một người dùng.
- Trách nhiệm: Nạp và lưu trữ các Subscription, gán gói mới, hoặc thu hồi gói.
- Tương tác: Kết nối với bridge `subscription_bridge` và sử dụng trong component hiển thị chi tiết người dùng.
*/

import { create } from 'zustand';
import type { Subscription, AutoRenewReport } from '../../../bridge/types';
import { 
    listUserSubscriptions, 
    addSubscriptionToUser, 
    updateSubscriptionExpiry, 
    removeSubscriptionFromUser,
    listAllSubscriptions,
    setSubscriptionAutoRenew,
    processAutoRenewals
} from '../../../bridge/subscription_bridge';

interface SubscriptionState {
    // Mảng chứa các gói đăng ký của user hiện tại
    subscriptions: Subscription[];
    // Mảng chứa TOÀN BỘ gói đăng ký trong hệ thống
    allSubscriptions: Subscription[];
    // User ID đang được chọn để xem
    currentUserId: string | null;
    isLoading: boolean;
    // Báo cáo lần rà tự động gia hạn gần nhất (null = chưa rà lần nào)
    autoRenewReport: AutoRenewReport | null;
    
    // Nạp toàn bộ
    fetchAllSubscriptions: () => Promise<void>;
    // Nạp danh sách theo user_id
    fetchUserSubscriptions: (userId: string) => Promise<void>;
    // Gán gói mới
    addSubscription: (userId: string, packageId: string, customExpiry?: number, amount?: number, autoRenew?: boolean) => Promise<void>;
    // Cập nhật ngày hết hạn
    updateExpiry: (subId: string, newExpiry: number, amount?: number, autoRenew?: boolean) => Promise<void>;
    // Thu hồi gói
    removeSubscription: (subId: string) => Promise<void>;
    // Bật/tắt tự động gia hạn cho một gói
    toggleAutoRenew: (subId: string, autoRenew: boolean) => Promise<void>;
    // Chạy rà tự động gia hạn, trả về báo cáo để UI thông báo
    runAutoRenewals: () => Promise<AutoRenewReport>;
    // Xoá báo cáo sau khi người dùng đã đọc
    clearAutoRenewReport: () => void;
}

export const useSubscriptionStore = create<SubscriptionState>((set, get) => ({
    subscriptions: [],
    allSubscriptions: [],
    currentUserId: null,
    isLoading: false,
    autoRenewReport: null,

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

    addSubscription: async (userId, packageId, customExpiry, amount, autoRenew) => {
        set({ isLoading: true });
        try {
            const newSub = await addSubscriptionToUser(userId, packageId, customExpiry, amount, autoRenew);
            // Cập nhật state nếu đang xem đúng user đó
            if (get().currentUserId === userId) {
                set({ subscriptions: [...get().subscriptions, newSub] });
            }
            set({ allSubscriptions: [...get().allSubscriptions, newSub] });
        } catch (error) {
            console.error("Lỗi gán gói đăng ký:", error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    updateExpiry: async (subId, newExpiry, amount, autoRenew) => {
        set({ isLoading: true });
        try {
            const updatedSub = await updateSubscriptionExpiry(subId, newExpiry, amount, autoRenew);
            set({ subscriptions: get().subscriptions.map(s => s.id === subId ? updatedSub : s) });
            set({ allSubscriptions: get().allSubscriptions.map(s => s.id === subId ? updatedSub : s) });
        } catch (error) {
            console.error("Lỗi gia hạn gói:", error);
            throw error;
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
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    toggleAutoRenew: async (subId, autoRenew) => {
        // Không set isLoading: checkbox cần phản hồi tức thì, bật cờ loading
        // sẽ khiến cả bảng nhảy vào trạng thái "Đang tải".
        try {
            const updatedSub = await setSubscriptionAutoRenew(subId, autoRenew);
            set({ subscriptions: get().subscriptions.map(s => s.id === subId ? updatedSub : s) });
            set({ allSubscriptions: get().allSubscriptions.map(s => s.id === subId ? updatedSub : s) });
        } catch (error) {
            console.error("Lỗi bật/tắt tự động gia hạn:", error);
            throw error;
        }
    },

    runAutoRenewals: async () => {
        try {
            const report = await processAutoRenewals();
            set({ autoRenewReport: report });
            // Có gia hạn = số dư và hạn đã đổi → nạp lại để state khớp file.
            if (report.renewed > 0) {
                await get().fetchAllSubscriptions();
                const uid = get().currentUserId;
                if (uid) {
                    await get().fetchUserSubscriptions(uid);
                }
            }
            return report;
        } catch (error) {
            console.error("Lỗi chạy tự động gia hạn:", error);
            throw error;
        }
    },

    clearAutoRenewReport: () => set({ autoRenewReport: null })
}));
