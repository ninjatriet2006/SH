/*
[INTEGRITY NOTES]
- Mục đích: Form popup nạp/trừ số dư của một khách hàng.
- Trách nhiệm: Nhận số tiền, xác định chiều (nạp/trừ), hiển thị trước số dư sau
  điều chỉnh rồi gọi `onSave`. KHÔNG tự tính số dư mới vào state — backend là
  nguồn sự thật, frontend chỉ hiển thị dự kiến.
- Tương tác: Dùng trong `UserManagementPage`, gọi `useUserStore.changeBalance`.
*/

import React, { useState, useEffect } from 'react';
import { X, Wallet } from 'lucide-react';
import type { User } from '../../../bridge/types';
import { useTranslation, formatCurrency } from '../utils/i18n';

interface BalanceModalProps {
    isOpen: boolean;
    user: User | null;
    onClose: () => void;
    onSave: (delta: number) => Promise<void>;
}

export function BalanceModal({ isOpen, user, onClose, onSave }: BalanceModalProps) {
    const { t } = useTranslation();
    // Số tiền luôn nhập DƯƠNG; chiều nạp/trừ do `direction` quyết định. Tách như
    // vậy để không phải nhập dấu trừ (dễ sai) và thấy rõ đang làm gì.
    const [amount, setAmount] = useState<number | undefined>(undefined);
    const [direction, setDirection] = useState<'deposit' | 'withdraw'>('deposit');
    const [isSaving, setIsSaving] = useState(false);

    // Reset form mỗi lần mở để không mang số tiền của lần trước.
    useEffect(() => {
        if (isOpen) {
            setAmount(undefined);
            setDirection('deposit');
            setIsSaving(false);
        }
    }, [isOpen, user?.id]);

    if (!isOpen || !user) return null;

    const delta = direction === 'deposit' ? (amount ?? 0) : -(amount ?? 0);
    const preview = user.balance + delta;

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (amount === undefined || !Number.isFinite(amount) || amount <= 0) {
            alert('Số tiền phải là số dương!');
            return;
        }
        // Số nguyên: VNĐ không có phần thập phân, và backend nhận i64.
        if (!Number.isInteger(amount)) {
            alert('Số tiền phải là số nguyên (VNĐ)!');
            return;
        }
        setIsSaving(true);
        try {
            await onSave(delta);
            onClose();
        } catch (err) {
            alert(`Điều chỉnh số dư thất bại: ${err instanceof Error ? err.message : String(err)}`);
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '400px' }}>
                <button
                    onClick={onClose}
                    style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}
                >
                    <X size={20} />
                </button>

                <h3 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Wallet size={20} /> {t('balance_modal.title')}
                </h3>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', margin: '0.25rem 0 1rem' }}>
                    {user.username}
                </p>

                <form onSubmit={handleSubmit}>
                    <div className="form-group" style={{ marginBottom: '1rem' }}>
                        <label className="form-label">{t('balance_modal.current')}</label>
                        <div style={{ fontWeight: 600, color: user.balance < 0 ? 'var(--danger)' : 'var(--success)' }}>
                            {formatCurrency(user.balance)}
                            {user.balance < 0 && ` (${t('users.balance_debt')})`}
                        </div>
                    </div>

                    {/* Chọn chiều điều chỉnh */}
                    <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
                        <button
                            type="button"
                            className={direction === 'deposit' ? 'btn btn-primary' : 'btn'}
                            style={{ flex: 1, background: direction === 'deposit' ? undefined : 'rgba(255,255,255,0.08)' }}
                            onClick={() => setDirection('deposit')}
                        >
                            + {t('balance_modal.deposit')}
                        </button>
                        <button
                            type="button"
                            className={direction === 'withdraw' ? 'btn btn-danger' : 'btn'}
                            style={{ flex: 1, background: direction === 'withdraw' ? undefined : 'rgba(255,255,255,0.08)' }}
                            onClick={() => setDirection('withdraw')}
                        >
                            − {t('balance_modal.withdraw')}
                        </button>
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('balance_modal.lbl_amount')}</label>
                        <input
                            type="number"
                            className="input-field"
                            value={amount ?? ''}
                            onChange={(e) => setAmount(e.target.value === '' ? undefined : Number(e.target.value))}
                            placeholder="VD: 500000"
                            min="1"
                            step="1"
                            autoFocus
                        />
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            {t('balance_modal.hint')}
                        </small>
                    </div>

                    {amount !== undefined && amount > 0 && (
                        <div style={{ marginTop: '1rem', padding: '0.75rem', background: 'rgba(255,255,255,0.05)', borderRadius: '8px' }}>
                            <span style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                                {t('balance_modal.preview')}{' '}
                            </span>
                            <strong style={{ color: preview < 0 ? 'var(--danger)' : 'var(--success)' }}>
                                {formatCurrency(preview)}
                            </strong>
                        </div>
                    )}

                    <div className="modal-actions">
                        <button type="button" className="btn btn-danger" onClick={onClose}>
                            {t('common.cancel')}
                        </button>
                        <button type="submit" className="btn btn-primary" disabled={isSaving}>
                            {isSaving ? t('common.loading') : t('common.confirm')}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
