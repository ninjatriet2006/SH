/*
[INTEGRITY NOTES]
- Mục đích: Trang chính — danh sách nhà cung cấp AI với trạng thái kết nối.
- Trách nhiệm: Liệt kê, tìm kiếm, thêm/sửa/xoá, kiểm tra kết nối (một hoặc tất
  cả), quét model.
- Tương tác: `store/useProviderStore.ts`.

  Key trong `auth.json` (provider built-in) đã được backend TỰ ĐỘNG gộp vào
  danh sách này mỗi lần nạp — nên không cần trang/nút "quản lý auth key" riêng;
  xoá provider ở đây cũng xoá key tương ứng trong auth.json.
*/

import { useEffect, useMemo, useState } from 'react';
import {
    Plus, Edit, Trash2, RefreshCw, Plug, Boxes, Search,
} from 'lucide-react';
import type { ProviderView, SaveResult } from '../../../bridge/types';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';
import { StatusBadge } from '../components/StatusBadge';
import { ProviderModal } from '../components/ProviderModal';
import { ModelsModal } from '../components/ModelsModal';
import { ConfirmModal } from '../components/ConfirmModal';

export function ProvidersPage() {
    const { t } = useTranslation();
    const {
        providers, presets, statuses, checking, isLoading, isTestingAll,
        fetchProviders, fetchPresets, save, remove, testOne, testAll,
        scanModels, applyModels,
    } = useProviderStore();

    const [query, setQuery] = useState('');
    const [editing, setEditing] = useState<ProviderView | null>(null);
    const [isProviderModalOpen, setProviderModalOpen] = useState(false);
    const [modelsTarget, setModelsTarget] = useState<ProviderView | null>(null);
    const [toDelete, setToDelete] = useState<ProviderView | null>(null);
    const [notice, setNotice] = useState<string | null>(null);
    // Thông tin provider trùng, chờ người dùng xác nhận gộp.
    const [pendingDup, setPendingDup] = useState<{
        info: { id: string; name: string };
        args: { presetId: string; name: string; baseUrl: string; apiKey: string; npm: string; customId?: string | null };
    } | null>(null);

    const showSaveNotice = (result: SaveResult) => {
        const notices = [];
        if (result.normalized_base_url) {
            notices.push(`${t('provider_modal.url_fixed')} ${result.normalized_base_url}`);
        }
        if (result.detected_npm) {
            notices.push(`${t('provider_modal.auto_selected')} ${result.detected_npm}`);
        }
        if (notices.length > 0) setNotice(notices.join(' · '));
    };

    useEffect(() => {
        fetchProviders().catch(err => setNotice(String(err)));
        fetchPresets();
    }, [fetchProviders, fetchPresets]);

    const filtered = useMemo(() => {
        const q = query.trim().toLowerCase();
        if (!q) return providers;
        return providers.filter(p =>
            p.id.toLowerCase().includes(q)
            || p.name.toLowerCase().includes(q)
            || p.base_url.toLowerCase().includes(q)
        );
    }, [providers, query]);

    const handleSave = async (args: {
        presetId: string;
        name: string;
        baseUrl: string;
        apiKey: string;
        npm: string;
        forceOverwriteId?: string;
        customId?: string | null;
    }): Promise<boolean> => {
        const result: SaveResult = await save({
            providerId: editing?.id,
            ...args,
        });

        showSaveNotice(result);

        // Backend phát hiện trùng và CHƯA lưu → hỏi người dùng.
        if (!result.saved_id && result.duplicate_of) {
            setPendingDup({ info: result.duplicate_of, args });
            return false;
        }
        return true;
    };

    const confirmMerge = async () => {
        if (!pendingDup) return;
        try {
            const result = await save({
                providerId: editing?.id,
                ...pendingDup.args,
                forceOverwriteId: pendingDup.info.id,
            });
            showSaveNotice(result);
            setPendingDup(null);
            setProviderModalOpen(false);
            setEditing(null);
        } catch (err) {
            // Đóng hộp thoại gộp để lỗi hiện trên trang nhìn thấy được (nó đang
            // che cả notice lẫn form); form provider còn mở để người dùng sửa —
            // vd bỏ trống ô ID — rồi lưu lại.
            setPendingDup(null);
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleDelete = async () => {
        if (!toDelete) return;
        try {
            await remove(toDelete.id);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setToDelete(null);
        }
    };

    return (
        <>
            <div className="animate-fade-in">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', flexWrap: 'wrap', gap: '1rem' }}>
                    <h1>{t('providers.title')}</h1>
                    <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                        <button
                            className="btn"
                            style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }}
                            onClick={() => testAll().catch(err => setNotice(String(err)))}
                            disabled={isTestingAll || providers.length === 0}
                        >
                            <RefreshCw size={18} /> {isTestingAll ? t('cleanup.scanning') : t('providers.test_all')}
                        </button>
                        <button
                            className="btn btn-primary"
                            onClick={() => { setEditing(null); setProviderModalOpen(true); }}
                        >
                            <Plus size={18} /> {t('providers.add')}
                        </button>
                    </div>
                </div>

                {notice && (
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(99,102,241,0.15)', borderLeft: '4px solid var(--primary)', borderRadius: '4px', fontSize: '0.9rem' }}>
                        <span>{notice}</span>
                        <button className="btn" style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }} onClick={() => setNotice(null)}>✕</button>
                    </div>
                )}

                <div style={{ marginBottom: '1.5rem', position: 'relative', maxWidth: '420px' }}>
                    <Search size={16} style={{ position: 'absolute', left: '10px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)' }} />
                    <input
                        type="text"
                        className="input-field"
                        style={{ paddingLeft: '32px' }}
                        value={query}
                        onChange={e => setQuery(e.target.value)}
                        placeholder={t('common.search_placeholder')}
                    />
                </div>

                <div className="glass-panel">
                    {isLoading && providers.length === 0 ? (
                        <p>{t('common.loading')}</p>
                    ) : filtered.length === 0 ? (
                        <p style={{ color: 'var(--text-secondary)' }}>{t('providers.none')}</p>
                    ) : (
                        <div className="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th>{t('providers.col_id')}</th>
                                        <th>{t('providers.col_name')}</th>
                                        <th>{t('providers.col_url')}</th>
                                        <th>{t('providers.col_key')}</th>
                                        <th>{t('providers.col_models')}</th>
                                        <th>{t('providers.col_status')}</th>
                                        <th>{t('providers.col_actions')}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {filtered.map(p => (
                                        <tr key={p.id}>
                                            <td style={{ fontSize: '0.85rem' }}>
                                                <div style={{ fontFamily: 'monospace' }}>{p.id}</div>
                                                <span className="badge badge-inactive" style={{ fontSize: '0.65rem' }}>
                                                    {p.is_builtin ? t('providers.builtin') : t('providers.custom')}
                                                </span>
                                            </td>
                                            <td style={{ fontWeight: 600 }}>{p.name}</td>
                                            <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', wordBreak: 'break-all', maxWidth: '220px' }}>
                                                {p.base_url}
                                            </td>
                                            <td style={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                                                {p.has_api_key
                                                    ? p.api_key_masked
                                                    : <span style={{ color: 'var(--danger)' }}>{t('providers.no_key')}</span>}
                                            </td>
                                            <td style={{ fontSize: '0.85rem' }}>
                                                {p.model_count} {t('providers.model_count')}
                                                {p.primary_model && (
                                                    <div style={{ fontSize: '0.7rem', color: 'var(--primary)', marginTop: '2px' }}>
                                                        ⭐ <span style={{ fontFamily: 'monospace' }}>{p.primary_model}</span>
                                                    </div>
                                                )}
                                            </td>
                                            <td>
                                                <StatusBadge
                                                    kind={statuses[p.id]?.kind}
                                                    message={statuses[p.id]?.message}
                                                    isChecking={checking[p.id]}
                                                />
                                            </td>
                                            <td>
                                                <div style={{ display: 'flex', gap: '0.4rem' }}>
                                                    <button
                                                        className="btn"
                                                        style={{ padding: '0.35rem', background: 'rgba(255,255,255,0.08)' }}
                                                        onClick={() => testOne(p.id).catch(err => setNotice(String(err)))}
                                                        title={t('providers.test_one')}
                                                    >
                                                        <Plug size={15} />
                                                    </button>
                                                    <button
                                                        className="btn"
                                                        style={{ padding: '0.35rem', background: 'rgba(255,255,255,0.08)' }}
                                                        onClick={() => setModelsTarget(p)}
                                                        title={t('providers.scan_models')}
                                                    >
                                                        <Boxes size={15} />
                                                    </button>
                                                    <button
                                                        className="btn btn-primary"
                                                        style={{ padding: '0.35rem' }}
                                                        onClick={() => { setEditing(p); setProviderModalOpen(true); }}
                                                        title={t('common.edit')}
                                                    >
                                                        <Edit size={15} />
                                                    </button>
                                                    <button
                                                        className="btn"
                                                        style={{ padding: '0.35rem', color: 'var(--danger)', border: '1px solid var(--danger)' }}
                                                        onClick={() => setToDelete(p)}
                                                        title={t('common.delete')}
                                                    >
                                                        <Trash2 size={15} />
                                                    </button>
                                                </div>
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                </div>
            </div>

            <ProviderModal
                isOpen={isProviderModalOpen}
                provider={editing}
                presets={presets}
                onClose={() => { setProviderModalOpen(false); setEditing(null); }}
                onSave={handleSave}
            />

            <ModelsModal
                isOpen={modelsTarget !== null}
                /* Lấy ProviderView MỚI NHẤT từ store (modelsTarget chỉ là snapshot
                   cũ — model chính/đếm model đổi ở chỗ khác sẽ không phản ánh). */
                provider={modelsTarget ? (providers.find(p => p.id === modelsTarget.id) ?? modelsTarget) : null}
                onClose={() => setModelsTarget(null)}
                onScan={scanModels}
                onApply={applyModels}
                onPrimaryChanged={() => fetchProviders().catch(() => {})}
            />

            <ConfirmModal
                isOpen={toDelete !== null}
                title={t('providers.delete_confirm_title')}
                message={`${t('providers.delete_confirm_msg')}\n\n${toDelete?.name ?? ''} (${toDelete?.id ?? ''})`}
                onConfirm={handleDelete}
                onCancel={() => setToDelete(null)}
                confirmText={t('common.delete')}
                cancelText={t('common.cancel')}
                isDanger
            />

            <ConfirmModal
                isOpen={pendingDup !== null}
                title={t('provider_modal.dup_title')}
                message={`${t('provider_modal.dup_msg')}\n\n${t('provider_modal.dup_merge')}: ${pendingDup?.info.name ?? ''} (${pendingDup?.info.id ?? ''})`}
                onConfirm={confirmMerge}
                onCancel={() => setPendingDup(null)}
                confirmText={t('provider_modal.dup_merge')}
                cancelText={t('common.cancel')}
            />
        </>
    );
}
