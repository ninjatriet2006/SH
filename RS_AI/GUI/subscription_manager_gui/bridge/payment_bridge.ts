/*
[INTEGRITY NOTES]
- Mục đích: Bridge cho MÃ THANH TOÁN (nội dung chuyển khoản) — phát hành khi in
  hóa đơn và tra cứu ngược từ sao kê ngân hàng.
- Trách nhiệm: Gọi lệnh Tauri, ném lỗi ra cho UI xử lý (không nuốt lỗi vì phát
  hành mã thất bại nghĩa là hóa đơn in ra sẽ không truy vết được).
- Tương tác: `components/InvoiceModal.tsx`, backend `payment_api.rs`.
*/

import { invokeCommand, type PaymentRef, type PaymentLookup } from './types';

/// Phát hành mã mới cho nhóm giao dịch của CÙNG một khách và lưu lại.
export async function issuePaymentRef(user_id: string, transaction_ids: string[]): Promise<PaymentRef> {
    try {
        return await invokeCommand<PaymentRef, { user_id: string; transaction_ids: string[] }>('issue_payment_ref', { user_id, transaction_ids });
    } catch (error) {
        throw new Error(String(error));
    }
}

/// Tra cứu nội dung chuyển khoản → người chuyển + giao dịch.
/// Chịu được chuỗi bẩn từ sao kê và mã bị gõ sai một phần.
export async function lookupPaymentRef(input: string): Promise<PaymentLookup> {
    try {
        return await invokeCommand<PaymentLookup, { input: string }>('lookup_payment_ref', { input });
    } catch (error) {
        throw new Error(String(error));
    }
}

/// Danh sách mã đã phát hành, mới nhất trước. Bỏ trống `user_id` = lấy tất cả.
export async function listPaymentRefs(user_id?: string): Promise<PaymentRef[]> {
    try {
        return await invokeCommand<PaymentRef[], { user_id: string | null }>('list_payment_refs', { user_id: user_id ?? null });
    } catch (error) {
        throw new Error(String(error));
    }
}

/// Đánh dấu đã nhận được tiền cho một mã (đối soát xong).
export async function settlePaymentRef(code: string, settled: boolean): Promise<PaymentRef> {
    try {
        return await invokeCommand<PaymentRef, { code: string; settled: boolean }>('settle_payment_ref', { code, settled });
    } catch (error) {
        throw new Error(String(error));
    }
}
