/*
[INTEGRITY NOTES]
- Mục đích: Đối soát thanh toán — dán nội dung chuyển khoản từ sao kê ngân hàng
  để truy ra người chuyển và các giao dịch tương ứng.
- Trách nhiệm: Gọi `lookup_payment_ref` (chịu được chuỗi bẩn/gõ sai), hiển thị
  kết quả khớp chính xác hoặc suy ra theo dấu hiệu nhận dạng khách, và cho phép
  đánh dấu đã nhận tiền.
- Tương tác: `bridge/payment_bridge.ts` → backend `payment_api.rs`.
*/

import { useEffect, useState } from 'react';
import { Search, CheckCircle2, Clock, AlertTriangle } from 'lucide-react';
import type { PaymentRef, PaymentLookup } from '../../../bridge/types';
import { lookupPaymentRef, listPaymentRefs, settlePaymentRef } from '../../../bridge/payment_bridge';
import { useTranslation, formatDateTime, formatCurrency } from '../utils/i18n';

export function PaymentReconciliationPage() {
    const { t } = useTranslation();
    const [input, setInput] = useState('');
    const [result, setResult] = useState<PaymentLookup | null>(null);
    const [refs, setRefs] = useState<PaymentRef[]>([]);
    const [isSearching, setIsSearching] = useState(false);

    const loadRefs = async () => {
        try {
            setRefs(await listPaymentRefs());
        } catch (err) {
            console.error('Không tải được danh sách mã:', err);
        }
    };

    useEffect(() => { loadRefs(); }, []);

    const handleLookup = async () => {
        if (!input.trim()) return;
        setIsSearching(true);
        try {
            setResult(await lookupPaymentRef(input));
        } catch (err) {
            alert(`Tra cứu thất bại: ${err instanceof Error ? err.message : String(err)}`);
        } finally {
            setIsSearching(false);
        }
    };

    const handleSettle = async (code: string, settled: boolean) => {
        try {
            await settlePaymentRef(code, settled);
            // Nạp lại cả danh sách và kết quả đang xem để trạng thái khớp file.
            await loadRefs();
            if (input.trim()) setResult(await lookupPaymentRef(input));
        } catch (err) {
            alert(`Cập nhật thất bại: ${err instanceof Error ? err.message : String(err)}`);
        }
    };

    /// Thẻ thông tin một mã. Dùng cho cả khớp chính xác và danh sách suy ra.
    const RefCard = ({ r, highlight }: { r: PaymentRef; highlight?: boolean }) => (
        <div style={{
            border: `1px solid ${highlight ? 'var(--success)' : 'var(--border)'}`,
            borderRadius: '8px',
            padding: '1rem',
            background: 'var(--bg-panel)',
            marginBottom: '0.75rem'
        }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
                <span style={{ fontFamily: 'monospace', fontSize: '1.1rem', fontWeight: 700, letterSpacing: '1px' }}>
                    {r.code}
                </span>
                <span className={`badge ${r.settled_at ? 'badge-active' : 'badge-inactive'}`}>
                    {r.settled_at ? t('payment.settled') : t('payment.not_settled')}
                </span>
            </div>

            <div style={{ marginTop: '0.75rem', display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: '0.5rem', fontSize: '0.85rem' }}>
                <div>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('payment.customer')}: </span>
                    <strong>{r.username}</strong>
                </div>
                <div>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('payment.amount')}: </span>
                    <strong>{formatCurrency(r.amount)}</strong>
                </div>
                <div>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('payment.tx_count')}: </span>
                    <strong>{r.transaction_ids.length}</strong>
                </div>
                <div>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('payment.issued_at')}: </span>
                    <strong>{formatDateTime(r.created_at)}</strong>
                </div>
                <div>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('payment.token')}: </span>
                    <strong style={{ fontFamily: 'monospace' }}>{r.user_token}</strong>
                </div>
            </div>

            <div style={{ marginTop: '0.5rem', fontSize: '0.75rem', color: 'var(--text-secondary)', wordBreak: 'break-all' }}>
                {r.transaction_ids.join(', ')}
            </div>

            <button
                className={r.settled_at ? 'btn' : 'btn btn-primary'}
                style={{ marginTop: '0.75rem', padding: '0.3rem 0.75rem', fontSize: '0.8rem', background: r.settled_at ? 'rgba(255,255,255,0.08)' : undefined }}
                onClick={() => handleSettle(r.code, !r.settled_at)}
            >
                {r.settled_at ? t('payment.unmark_settled') : t('payment.mark_settled')}
            </button>
        </div>
    );

    return (
        <div className="animate-fade-in">
            <h1>{t('payment.title')}</h1>

            <div style={{ background: 'var(--bg-panel)', padding: '1.5rem', borderRadius: '12px', border: '1px solid var(--border)', marginTop: '1.5rem' }}>
                <label className="form-label">{t('payment.lookup_label')}</label>
                <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                    <input
                        type="text"
                        className="input-field"
                        style={{ flex: 1, minWidth: '240px' }}
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyDown={(e) => { if (e.key === 'Enter') handleLookup(); }}
                        placeholder={t('payment.lookup_placeholder')}
                    />
                    <button className="btn btn-primary" onClick={handleLookup} disabled={isSearching || !input.trim()}>
                        <Search size={18} /> {isSearching ? t('common.loading') : t('payment.lookup_btn')}
                    </button>
                </div>
            </div>

            {/* Kết quả tra cứu */}
            {result && (
                <div style={{ marginTop: '1.5rem' }}>
                    {result.exact ? (
                        <>
                            <h3 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--success)' }}>
                                <CheckCircle2 size={20} /> {t('payment.exact_match')}
                            </h3>
                            <RefCard r={result.exact} highlight />
                        </>
                    ) : result.same_user.length > 0 ? (
                        <>
                            {/* Đây là giá trị của dấu hiệu nhận dạng: mã gõ sai
                                vẫn truy ra được ai đã chuyển tiền. */}
                            <h3 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--warning)' }}>
                                <AlertTriangle size={20} /> {t('payment.no_exact')}
                            </h3>
                            <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
                                {t('payment.same_user_hint')}{' '}
                                <strong>{result.same_user[0].username}</strong>{' '}
                                (<span style={{ fontFamily: 'monospace' }}>{result.user_token}</span>)
                            </p>
                            {result.same_user.map(r => <RefCard key={r.code} r={r} />)}
                        </>
                    ) : (
                        <div style={{ padding: '1rem', background: 'rgba(239, 68, 68, 0.12)', borderLeft: '4px solid var(--danger)', borderRadius: '4px' }}>
                            {t('payment.not_found')}
                        </div>
                    )}
                </div>
            )}

            {/* Lịch sử mã đã phát hành */}
            <h3 style={{ marginTop: '2rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <Clock size={18} /> {t('payment.history')} ({refs.length})
            </h3>
            {refs.length === 0 ? (
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>{t('payment.no_refs')}</p>
            ) : (
                refs.map(r => <RefCard key={r.code} r={r} />)
            )}
        </div>
    );
}
