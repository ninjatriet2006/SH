/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Backend (Rust) cho chức năng Quản lý Đăng ký gói (Subscription).
- Trách nhiệm: Giao tiếp với lệnh Tauri, truyền đúng tham số (user_id, package_id, timestamp...) và trả về kiểu dữ liệu Subscription.
- Tương tác: Dùng interface `Subscription` từ `types.ts`.
*/

// Nhúng lệnh gọi API từ Tauri
// Nhúng kiểu dữ liệu Subscription
import { invokeCommand, type Subscription, type AutoRenewReport, type Empty } from './types';

// Hàm gọi API gán gói dịch vụ cho người dùng
export async function addSubscriptionToUser(
    user_id: string, 
    package_id: string, 
    custom_expiration_date?: number,
    amount?: number,
    auto_renew?: boolean
): Promise<Subscription> {
    try {
        // Gọi lệnh "add_subscription_to_user"
        const result = await invokeCommand<Subscription, { user_id: string; package_id: string; custom_expiration_date: string | null; amount: number | null; auto_renew: boolean | null }>('add_subscription_to_user', {
            user_id: user_id,
            package_id: package_id,
            custom_expiration_date: custom_expiration_date == null ? null : String(custom_expiration_date),
            amount: amount ?? null,
            auto_renew: auto_renew ?? null
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API cập nhật ngày hết hạn của một đăng ký.
// `auto_renew` bỏ trống = giữ nguyên lựa chọn hiện tại (quy ước như update_user).
export async function updateSubscriptionExpiry(
    subscription_id: string, 
    new_expiration_date: number,
    amount?: number,
    auto_renew?: boolean
): Promise<Subscription> {
    try {
        // Gọi lệnh "update_subscription_expiry"
        const result = await invokeCommand<Subscription, { subscription_id: string; new_expiration_date: string; amount: number | null; auto_renew: boolean | null }>('update_subscription_expiry', {
            subscription_id: subscription_id,
            new_expiration_date: String(new_expiration_date),
            amount: amount ?? null,
            auto_renew: auto_renew ?? null
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
        await invokeCommand<void, { subscription_id: string }>('remove_subscription_from_user', {
            subscription_id: subscription_id
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách đăng ký của một người dùng
export async function listUserSubscriptions(userId: string): Promise<Subscription[]> {
    try {
        // Gọi "list_user_subscriptions"
        const result = await invokeCommand<Subscription[], { user_id: string }>('list_user_subscriptions', {
            user_id: userId
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
        const result = await invokeCommand<boolean, { subscription_id: string }>('check_subscription_status', {
            subscription_id: subscription_id
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách toàn bộ đăng ký trong hệ thống
export async function listAllSubscriptions(): Promise<Subscription[]> {
    try {
        const result = await invokeCommand<Subscription[], Empty>('list_all_subscriptions', {});
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

/// Bật/tắt tự động gia hạn cho một đăng ký.
export async function setSubscriptionAutoRenew(
    subscription_id: string,
    auto_renew: boolean
): Promise<Subscription> {
    try {
        return await invokeCommand<Subscription, { subscription_id: string; auto_renew: boolean }>('set_subscription_auto_renew', {
            subscription_id: subscription_id,
            auto_renew: auto_renew
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

/// Chạy rà tự động gia hạn và lấy báo cáo (đã gia hạn bao nhiêu, trừ bao nhiêu
/// tiền, gói nào bị bỏ qua vì thiếu số dư).
export async function processAutoRenewals(): Promise<AutoRenewReport> {
    try {
        return await invokeCommand<AutoRenewReport, Empty>('process_auto_renewals', {});
    } catch (error) {
        throw new Error(String(error));
    }
}
