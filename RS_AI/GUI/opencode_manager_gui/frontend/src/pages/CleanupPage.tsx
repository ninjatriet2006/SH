/*
[INTEGRITY NOTES]
- Mục đích: Dọn nhà cung cấp không hoạt động.
- Trách nhiệm: Kiểm tra toàn bộ provider, liệt kê những mục lỗi kèm lý do, cho
  chọn và xoá theo lô.
- Tương tác: `bridge/provider_bridge.ts` (`findBadProviders`),
  `store/useProviderStore.ts` (`removeMany`).

Mặc định KHÔNG tick sẵn mục nào: xoá provider là thao tác không hoàn tác, và lỗi
"offline" có thể chỉ do mất mạng tạm thời — tick sẵn dễ dẫn tới xoá oan.
*/

import { useState } from 'react';
import { Trash2, Search, ShieldAlert } from 'lucide-react';
import type { BadProvider } from '../../../bridge/types';
import { findBadProviders } from '../../../bridge/provider_bridge';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';
import { StatusBadge } from '../components/StatusBadge';
import { ConfirmModal } from '../components/ConfirmModal';

export function CleanupPage() {
    const { t } = useTranslation();
    const { removeMany } = useProviderStore();
    const [bad, setBad] = useState<BadProvider[] | null>(null);
    const [checked, setChecked] = useState<Set<string>>(new Set());
    const [isScanning, setIsScanning] = useState(false);
    const [confirmOpen, setConfirmOpen] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);

    const handleScan = async () => {
        setIsScanning(true);
        setNotice(null);
        try {
            const list = await findBadProviders();
            setBad(list);
            setChecked(new Set());
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsScanning(false);
        }
    };

    const toggle = (id: string) => {
        const next = new Set(checked);
        if (next.has(id)) next.delete(id); else next.add(id);
        setChecked(next);
    };

    const handleDelete = async () => {
        try {
            const n = await removeMany([...checked]);
            setNotice(`${t('cleanup.deleted')}: ${n}`);
            // Nạp lại danh sách lỗi để mục vừa xoá không còn nằm trên bảng.
            setBad(prev => (prev ?? []).filter(b => !checked.has(b.id)));
            setChecked(new Set());
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setConfirmOpen(false);
        }
    };

    return (
        <>
            <div className="animate-fade-in">
                <h1 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <ShieldAlert size={24} /> {t('cleanup.title')}
                </h1>
                <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>{t('cleanup.desc')}</p>

                {notice && (
                    <div style={{ display: 'flex', justifyContent: 'space-between', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(99,102,241,0.15)', borderLeft: '4px solid var(--primary)', borderRadius: '4px', fontSize: '0.9rem' }}>
                        <span>{notice}</span>
                        <button className="btn" style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }} onClick={() => setNotice(null)}>✕</button>
                    </div>
                )}

                <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem', flexWrap: 'wrap' }}>
                    <button className="btn btn-primary" onClick={handleScan} disabled={isScanning}>
                        <Search size={18} /> {isScanning ? t('cleanup.scanning') : t('cleanup.scan')}
                    </button>
                    {checked.size > 0 && (
                        <button
                            className="btn"
                            style={{ color: 'var(--danger)', border: '1px solid var(--danger)' }}
                            onClick={() => setConfirmOpen(true)}
                        >
                            <Trash2 size={18} /> {t('cleanup.delete_selected')} ({checked.size})
                        </button>
                    )}
                </div>

                {bad !== null && (
                    <div className="glass-panel">
                        {bad.length === 0 ? (
                            <p style={{ color: 'var(--success)' }}>✓ {t('cleanup.none')}</p>
                        ) : (
                            <>
                                <h3 style={{ marginTop: 0 }}>{t('cleanup.found')}: {bad.length}</h3>
                                <div className="table-container">
                                    <table>
                                        <thead>
                                            <tr>
                                                <th style={{ width: '36px' }} />
                                                <th>{t('providers.col_id')}</th>
                                                <th>{t('providers.col_name')}</th>
                                                <th>{t('providers.col_status')}</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {bad.map(b => (
                                                <tr key={b.id}>
                                                    <td>
                                                        <input
                                                            type="checkbox"
                                                            checked={checked.has(b.id)}
                                                            onChange={() => toggle(b.id)}
                                                            style={{ width: '15px', height: '15px', cursor: 'pointer' }}
                                                        />
                                                    </td>
                                                    <td style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>
                                                        {b.id}
                                                        <br />
                                                        <span className="badge badge-inactive" style={{ fontSize: '0.65rem' }}>
                                                            {b.is_builtin ? t('providers.builtin') : t('providers.custom')}
                                                        </span>
                                                    </td>
                                                    <td style={{ fontWeight: 600 }}>{b.name}</td>
                                                    <td>
                                                        <StatusBadge kind={b.kind} message={b.message} />
                                                        {b.message && (
                                                            <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '4px', maxWidth: '320px' }}>
                                                                {b.message}
                                                            </div>
                                                        )}
                                                    </td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            </>
                        )}
                    </div>
                )}
            </div>

            <ConfirmModal
                isOpen={confirmOpen}
                title={t('cleanup.delete_confirm_title')}
                message={`${t('cleanup.delete_confirm_msg')}\n\n${[...checked].join(', ')}`}
                onConfirm={handleDelete}
                onCancel={() => setConfirmOpen(false)}
                confirmText={t('common.delete')}
                cancelText={t('common.cancel')}
                isDanger
            />
        </>
    );
}
