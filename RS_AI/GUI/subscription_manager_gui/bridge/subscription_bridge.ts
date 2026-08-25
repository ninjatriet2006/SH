/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Backend (Rust) cho chức năng Quản lý Đăng ký gói (Subscription).
- Trách nhiệm: Giao tiếp với lệnh Tauri, truyền đúng tham số (user_id, package_id, timestamp...) và trả về kiểu dữ liệu Subscription.
- Tương tác: Dùng interface `Subscription` từ `types.ts`.
*/

// Nhúng lệnh gọi API từ Tauri
import { invoke } from '@tauri-apps/api/core';
// Nhúng kiểu dữ liệu Subscription
import type { Subscription } from './types';

// Hàm gọi API gán gói dịch vụ cho người dùng
export async function addSubscriptionToUser(
    user_id: string, 
    package_id: string, 
    custom_expiration_date?: number,
    amount?: number
): Promise<Subscription> {
    try {
        // Gọi lệnh "add_subscription_to_user"
        const result = await invoke<Subscription>('add_subscription_to_user', {
            userId: user_id,
            packageId: package_id,
            customExpirationDate: custom_expiration_date || null,
            amount: amount || null
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API cập nhật ngày hết hạn của một đăng ký
export async function updateSubscriptionExpiry(
    subscription_id: string, 
    new_expiration_date: number,
    amount?: number
): Promise<Subscription> {
    try {
        // Gọi lệnh "update_subscription_expiry"
        const result = await invoke<Subscription>('update_subscription_expiry', {
            subscriptionId: subscription_id,
            newExpirationDate: new_expiration_date,
            amount: amount || null
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API xóa đăng ký của người dùng
export async function removeSubscriptionFromUser(subscription_id: string): Promise<void> {
    try {
        // Gọi "remove_subscription_from_user"
        await invoke<void>('remove_subscription_from_user', {
            subscriptionId: subscription_id
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách đăng ký của một người dùng
export async function listUserSubscriptions(userId: string): Promise<Subscription[]> {
    try {
        // Gọi "list_user_subscriptions"
        const result = await invoke<Subscription[]>('list_user_subscriptions', {
            userId: userId
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API kiểm tra trạng thái kích hoạt của gói
export async function checkSubscriptionStatus(subscription_id: string): Promise<boolean> {
    try {
        // Gọi "check_subscription_status"
        const result = await invoke<boolean>('check_subscription_status', {
            subscriptionId: subscription_id
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách toàn bộ đăng ký trong hệ thống
export async function listAllSubscriptions(): Promise<Subscription[]> {
    try {
        const result = await invoke<Subscription[]>('list_all_subscriptions');
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}
