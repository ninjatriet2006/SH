/*
[INTEGRITY NOTES]
- Mục đích: Trang thêm nhanh nhiều provider — 1 endpoint + N API key.
- Trách nhiệm: Nhập endpoint và danh sách key (mỗi dòng 1 key), gọi bridge, hiện
  báo cáo thêm/trùng. Sau khi thêm xong nạp lại danh sách provider từ backend —
  backend là nguồn sự thật.
- Tương tác: `bridge/bulk_bridge.ts`, `store/useProviderStore.ts`.
*/

import { useState } from 'react';
import { Zap } from 'lucide-react';
import { bulkAddProviders } from '../../../bridge/bulk_bridge';
import type { BulkAddResult } from '../../../bridge/types';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';

export function BulkAddPage() {
    const { t } = useTranslation();
    const { fetchProviders } = useProviderStore();

    const [endpoint, setEndpoint] = useState('');
    const [keys, setKeys] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);
    const [result, setResult] = useState<BulkAddResult | null>(null);

    const keyCount = keys.split('\n').filter(l => l.trim().length > 0).length;

    const handleSubmit = async () => {
        setNotice(null);
        setResult(null);
        setIsSubmitting(true);
        try {
            const r = await bulkAddProviders(endpoint.trim(), keys);
            setResult(r);
            setKeys('');
            await fetchProviders();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsSubmitting(false);
        }
    };

    return (
        <div className="animate-fade-in" style={{ maxWidth: '720px' }}>
            <h1 style={{ marginBottom: '0.5rem' }}>{t('bulk.title')}</h1>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>
                {t('bulk.desc')}
            </p>

            {notice && (
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(239,68,68,0.12)', borderLeft: '4px solid var(--danger)', borderRadius: '4px', fontSize: '0.9rem' }}>
                    <span>{notice}</span>
                    <button className="btn" style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }} onClick={() => setNotice(null)}>✕</button>
                </div>
            )}

            {result && (
                <div style={{ padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(34,197,94,0.12)', borderLeft: '4px solid #22c55e', borderRadius: '4px', fontSize: '0.9rem' }}>
                    {t('bulk.result_added')}: {result.added}
                    {result.skipped_existing > 0 && ` · ${t('bulk.result_existing')}: ${result.skipped_existing}`}
                    {result.skipped_duplicate_input > 0 && ` · ${t('bulk.result_dup')}: ${result.skipped_duplicate_input}`}
                    {result.normalized_endpoint !== endpoint.trim() && result.normalized_endpoint && (
                        <div style={{ marginTop: '0.25rem', color: 'var(--text-secondary)' }}>
                            {t('bulk.result_normalized')}: {result.normalized_endpoint}
                        </div>
                    )}
                </div>
            )}

            <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem' }}>
                    {t('bulk.lbl_endpoint')}
                    <input
                        type="text"
                        className="input-field"
                        value={endpoint}
                        onChange={e => setEndpoint(e.target.value)}
                        placeholder="https://api.example.com/v1"
                    />
                </label>

                <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem' }}>
                    {t('bulk.lbl_keys')} ({keyCount})
                    <textarea
                        className="input-field"
                        style={{ minHeight: '180px', fontFamily: 'monospace', resize: 'vertical' }}
                        value={keys}
                        onChange={e => setKeys(e.target.value)}
                        placeholder={t('bulk.keys_placeholder')}
                    />
                </label>

                <div>
                    <button
                        className="btn btn-primary"
                        onClick={handleSubmit}
                        disabled={isSubmitting || !endpoint.trim() || keyCount === 0}
                    >
                        <Zap size={18} /> {isSubmitting ? t('common.loading') : t('bulk.submit')}
                    </button>
                </div>
            </div>
        </div>
    );
}
