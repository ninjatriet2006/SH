/*
[INTEGRITY NOTES]
- Mục đích: Trang chính — danh sách nhà cung cấp AI với trạng thái kết nối.
- Trách nhiệm: Liệt kê, tìm kiếm, thêm/sửa/xoá, kiểm tra kết nối (một hoặc tất
  cả), quét model. Danh sách tách hai nhóm "Custom provider" / "Built-in
  Provider"; trong mỗi nhóm, provider được đánh sao lên trước và đổi thứ tự
  được bằng mũi tên; cột riêng ghim model ưa thích của từng provider. Mọi cột
  đều có handle kéo đổi độ rộng (lưu localStorage, nhấn đúp để đặt lại).
- Tương tác: `store/useProviderStore.ts`, `store/usePreferencesStore.ts`,
  `hooks/useColumnWidths.ts`.

  Key trong `auth.json` (provider built-in) đã được backend TỰ ĐỘNG gộp vào
  danh sách này mỗi lần nạp — nên không cần trang/nút "quản lý auth key" riêng;
  xoá provider ở đây cũng xoá key tương ứng trong auth.json.
*/

import { useEffect, useMemo, useRef, useState, useDeferredValue } from 'react';
import type { ReactNode } from 'react';
import {
    Plus, Edit, Trash2, RefreshCw, Boxes, Search, Star, ChevronUp, ChevronDown, AlertTriangle, Radar,
} from 'lucide-react';
import type { ProviderView, SaveResult } from '../../../bridge/types';
import { useProviderStore } from '../store/useProviderStore';
import { usePreferencesStore } from '../store/usePreferencesStore';
import { useColumnWidths } from '../hooks/useColumnWidths';
import type { ResizableColumn } from '../hooks/useColumnWidths';
import { useTranslation } from '../utils/i18n';
import { StatusBadge } from '../components/StatusBadge';
import { ProviderModal } from '../components/ProviderModal';
import { ModelsModal } from '../components/ModelsModal';
import { ConfirmModal } from '../components/ConfirmModal';

/** Định nghĩa cột + độ rộng mặc định/hạn dưới cho bảng provider. */
const PROVIDER_COLUMNS: ResizableColumn[] = [
    { key: 'favorite', defaultWidth: 78, minWidth: 78 },
    { key: 'tracking', defaultWidth: 88, minWidth: 78 },
    { key: 'id', defaultWidth: 130, minWidth: 90 },
    { key: 'name', defaultWidth: 125, minWidth: 90 },
    { key: 'url', defaultWidth: 195, minWidth: 110 },
    { key: 'key', defaultWidth: 115, minWidth: 80 },
    { key: 'models', defaultWidth: 95, minWidth: 70 },
    { key: 'pinned', defaultWidth: 160, minWidth: 100 },
    { key: 'status', defaultWidth: 105, minWidth: 80 },
    { key: 'actions', defaultWidth: 125, minWidth: 110 },
];

export function ProvidersPage() {
    const { t } = useTranslation();
    const {
        providers, presets, statuses, checking, isLoading, isTestingAll,
        fetchProviders, fetchPresets, save, remove, testAll,
        scanModels, applyModels,
    } = useProviderStore();
    const {
        favorites, pinnedModels, trackedProviders, isLoaded: preferencesLoaded,
        fetchPreferences, toggleFavorite, reorderFavorites, pinModel, toggleTracking, untrack,
    } = usePreferencesStore();
    const { widthOf, startResize, resetResize } = useColumnWidths('providers', PROVIDER_COLUMNS);

    const [query, setQuery] = useState('');
    const deferredQuery = useDeferredValue(query);
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
    // Endpoint trùng built-in, chờ người dùng xác nhận "vẫn lưu".
    const [pendingConflict, setPendingConflict] = useState<{
        info: { id: string; name: string };
        args: { presetId: string; name: string; baseUrl: string; apiKey: string; npm: string; customId?: string | null };
    } | null>(null);
    const startupTrackingStarted = useRef(false);

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
        void Promise.all([
            fetchProviders().catch(err => setNotice(String(err))),
            fetchPresets(),
            fetchPreferences(),
        ]);
    }, [fetchProviders, fetchPresets, fetchPreferences]);

    // Chỉ kiểm tra provider người dùng chọn, tuần tự để không tạo burst request.
    useEffect(() => {
        if (startupTrackingStarted.current || !preferencesLoaded || providers.length === 0) return;
        startupTrackingStarted.current = true;
        const tracked = trackedProviders.filter(id => providers.some(p => p.id === id));
        let cancelled = false;
        void (async () => {
            for (const providerId of tracked) {
                if (cancelled) return;
                try {
                    const models = await scanModels(providerId);
                    if (models.some(model => !model.stale && model.from_api)) await untrack(providerId);
                } catch {
                    // Provider lỗi vẫn được giữ để lần khởi động sau thử lại.
                }
                await new Promise(resolve => window.setTimeout(resolve, 350));
            }
        })();
        return () => { cancelled = true; };
    }, [preferencesLoaded, providers, trackedProviders, scanModels, untrack]);

    const filtered = useMemo(() => {
        const q = deferredQuery.trim().toLowerCase();
        if (!q) return providers;
        return providers.filter(p =>
            p.id.toLowerCase().includes(q)
            || p.name.toLowerCase().includes(q)
            || p.base_url.toLowerCase().includes(q)
        );
    }, [providers, deferredQuery]);

    // Trong mỗi nhóm: provider có sao lên TRƯỚC (theo đúng thứ tự người dùng
    // sắp bằng mũi tên), phần còn lại giữ nguyên thứ tự gốc của danh sách.
    const withFavoriteRank = useMemo(() => (list: ProviderView[]): ProviderView[] => {
        const rank = new Map(favorites.map((id, i) => [id, i]));
        return list
            .map((p, i) => ({ p, i }))
            .sort((a, b) => {
                const fa = rank.get(a.p.id);
                const fb = rank.get(b.p.id);
                if (fa !== undefined && fb !== undefined) return fa - fb;
                if (fa !== undefined) return -1;
                if (fb !== undefined) return 1;
                return a.i - b.i;
            })
            .map(x => x.p);
    }, [favorites]);

    const customGroup = useMemo(() => withFavoriteRank(filtered.filter(p => !p.is_builtin)), [filtered, withFavoriteRank]);
    const builtinGroup = useMemo(() => withFavoriteRank(filtered.filter(p => p.is_builtin)), [filtered, withFavoriteRank]);

    // Tổng độ rộng hiện tại của các cột — làm minWidth cho bảng (xem chú thích ở bảng).
    const totalWidth = useMemo(
        () => PROVIDER_COLUMNS.reduce((sum, c) => sum + widthOf(c.key), 0),
        [widthOf],
    );

    const handleToggleFavorite = async (providerId: string) => {
        try {
            await toggleFavorite(providerId);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    /** Đổi chỗ provider đang sao với đứa kế trên/dưới trong NHÓM SAO. */
    const moveFavorite = async (providerId: string, dir: -1 | 1) => {
        const idx = favorites.indexOf(providerId);
        const swap = idx + dir;
        if (idx === -1 || swap < 0 || swap >= favorites.length) return;
        const next = [...favorites];
        [next[idx], next[swap]] = [next[swap], next[idx]];
        try {
            await reorderFavorites(next);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handlePinModel = async (providerId: string, modelId: string) => {
        try {
            await pinModel(providerId, modelId);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleToggleTracking = async (providerId: string) => {
        try {
            await toggleTracking(providerId);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

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
        // Endpoint trùng preset built-in và CHƯA lưu → hỏi "vẫn lưu?".
        if (!result.saved_id && result.builtin_conflict) {
            setPendingConflict({ info: result.builtin_conflict, args });
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

    /** Người dùng bấm "Vẫn lưu" trong hộp thoại xung đột endpoint built-in. */
    const confirmConflictSave = async () => {
        if (!pendingConflict) return;
        try {
            const result = await save({
                providerId: editing?.id,
                ...pendingConflict.args,
                acknowledgeBuiltinConflict: true,
            });
            showSaveNotice(result);
            setPendingConflict(null);
            setProviderModalOpen(false);
            setEditing(null);
        } catch (err) {
            setPendingConflict(null);
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

    /** Dòng phân nhóm trong bảng — chiếm trọn độ rộng, chỉ là nhãn. */
    const GroupRow = ({ label, count }: { label: string; count: number }) => (
        <tr>
            <td
                colSpan={10}
                style={{
                    background: 'rgba(99,102,241,0.08)',
                    color: 'var(--primary)',
                    fontWeight: 700,
                    fontSize: '0.78rem',
                    letterSpacing: '0.04em',
                    textTransform: 'uppercase',
                    padding: '0.45rem 0.75rem',
                }}
            >
                {label} · {count}
            </td>
        </tr>
    );

    /** Ô tiêu đề có handle kéo đổi độ rộng gắn mép phải (xem `.col-resizer` CSS). */
    const Th = ({ colKey, children }: { colKey: string; children: ReactNode }) => (
        <th>
            {children}
            <span
                className="col-resizer"
                onMouseDown={e => startResize(colKey, e)}
                onDoubleClick={() => resetResize(colKey)}
                title={t('providers.col_resize_hint')}
            />
        </th>
    );

    const renderRow = (p: ProviderView) => {
        const favIdx = favorites.indexOf(p.id);
        const isFav = favIdx !== -1;
        const pinned = pinnedModels[p.id] ?? '';
        return (
            <tr key={p.id}>
                <td style={{ whiteSpace: 'nowrap' }}>
                    <div style={{ display: 'flex', gap: '2px', alignItems: 'center' }}>
                        <button
                            className="btn"
                            style={{
                                padding: '0.2rem',
                                background: 'transparent',
                                color: isFav ? 'var(--primary)' : 'var(--text-secondary)',
                            }}
                            onClick={() => handleToggleFavorite(p.id)}
                            title={isFav ? t('providers.unfavorite') : t('providers.favorite')}
                        >
                            <Star size={15} fill={isFav ? 'currentColor' : 'none'} />
                        </button>
                        {/* Đổi thứ tự chỉ áp dụng trong nhóm sao. */}
                        {isFav && (
                            <>
                                <button
                                    className="btn"
                                    style={{ padding: '0.1rem', background: 'transparent', color: 'var(--text-secondary)' }}
                                    disabled={favIdx === 0}
                                    onClick={() => moveFavorite(p.id, -1)}
                                    title={t('providers.move_up')}
                                >
                                    <ChevronUp size={13} />
                                </button>
                                <button
                                    className="btn"
                                    style={{ padding: '0.1rem', background: 'transparent', color: 'var(--text-secondary)' }}
                                    disabled={favIdx === favorites.length - 1}
                                    onClick={() => moveFavorite(p.id, 1)}
                                    title={t('providers.move_down')}
                                >
                                    <ChevronDown size={13} />
                                </button>
                            </>
                        )}
                    </div>
                </td>
                <td style={{ textAlign: 'center' }}>
                    <button className="btn" style={{ padding: '0.25rem', background: 'transparent', color: trackedProviders.includes(p.id) ? 'var(--primary)' : 'var(--text-secondary)' }} onClick={() => handleToggleTracking(p.id)} title={trackedProviders.includes(p.id) ? t('providers.untrack') : t('providers.track')} aria-label={trackedProviders.includes(p.id) ? t('providers.untrack') : t('providers.track')}>
                        <Radar size={15} />
                    </button>
                </td>
                <td style={{ fontSize: '0.85rem' }}>
                    <div style={{ fontFamily: 'monospace' }}>{p.id}</div>
                    <span className="badge badge-inactive" style={{ fontSize: '0.65rem' }}>
                        {p.is_builtin ? t('providers.builtin') : t('providers.custom')}
                    </span>
                </td>
                <td style={{ fontWeight: 600 }}>{p.name}</td>
                <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', wordBreak: 'break-all' }}>
                    {p.endpoint_conflict_preset && (
                        <span
                            title={`${t('providers.endpoint_conflict')}: ${p.endpoint_conflict_preset}`}
                            style={{ color: 'var(--danger)', marginRight: '4px', cursor: 'help' }}
                        >
                            <AlertTriangle size={12} style={{ verticalAlign: '-2px' }} />
                        </span>
                    )}
                    {p.base_url}
                </td>
                <td style={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>
                    {p.has_api_key
                        ? p.api_key_masked
                        : <span style={{ color: 'var(--danger)' }}>{t('providers.no_key')}</span>}
                </td>
                <td style={{ fontSize: '0.85rem' }}>
                    {p.model_count} {t('providers.model_count')}
                    {p.models_from_catalog && (
                        <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', marginTop: '2px' }}>
                            {t('providers.from_catalog')}
                        </div>
                    )}
                    {p.primary_model && (
                        <div style={{ fontSize: '0.7rem', color: 'var(--primary)', marginTop: '2px' }}>
                            ⭐ <span style={{ fontFamily: 'monospace' }}>{p.primary_model}</span>
                        </div>
                    )}
                </td>
                <td>
                    {p.models.length > 0 || pinned ? (
                        <select
                            className="input-field"
                            style={{
                                padding: '0.25rem 0.4rem', fontSize: '0.78rem',
                                width: '100%', fontFamily: 'monospace',
                            }}
                            value={pinned}
                            onChange={e => handlePinModel(p.id, e.target.value)}
                            title={t('providers.pinned_hint')}
                        >
                            <option value="">{t('providers.pinned_none')}</option>
                            {p.models.map(m => (
                                <option key={m} value={m}>{m}</option>
                            ))}
                            {/* Ghim model đã bị xoá khỏi config: vẫn hiện để người
                                dùng thấy và bỏ ghim, không tự biến mất. */}
                            {pinned && !p.models.includes(pinned) && (
                                <option value={pinned}>{pinned}</option>
                            )}
                        </select>
                    ) : (
                        <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>—</span>
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
        );
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
                            {/* Bố cục fixed + minWidth = tổng độ rộng: kéo một cột
                                rộng thêm thì bảng rộng theo và container cuộn ngang,
                                các cột khác không bị co lại. */}
                             <table style={{ tableLayout: 'fixed', width: totalWidth, minWidth: totalWidth }}>
                                <colgroup>
                                    {PROVIDER_COLUMNS.map(c => (
                                     <col key={c.key} style={{ width: widthOf(c.key), minWidth: widthOf(c.key) }} />
                                    ))}
                                </colgroup>
                                <thead>
                                    <tr>
                                        <Th colKey="favorite">{t('providers.col_favorite')}</Th>
                                        <Th colKey="tracking">{t('providers.col_tracking')}</Th>
                                        <Th colKey="id">{t('providers.col_id')}</Th>
                                        <Th colKey="name">{t('providers.col_name')}</Th>
                                        <Th colKey="url">{t('providers.col_url')}</Th>
                                        <Th colKey="key">{t('providers.col_key')}</Th>
                                        <Th colKey="models">{t('providers.col_models')}</Th>
                                        <Th colKey="pinned">{t('providers.col_pinned_model')}</Th>
                                        <Th colKey="status">{t('providers.col_status')}</Th>
                                        <Th colKey="actions">{t('providers.col_actions')}</Th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {customGroup.length > 0 && (
                                        <GroupRow label={t('providers.group_custom')} count={customGroup.length} />
                                    )}
                                    {customGroup.map(p => renderRow(p))}
                                    {builtinGroup.length > 0 && (
                                        <GroupRow label={t('providers.group_builtin')} count={builtinGroup.length} />
                                    )}
                                    {builtinGroup.map(p => renderRow(p))}
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

            <ConfirmModal
                isOpen={pendingConflict !== null}
                title={t('provider_modal.conflict_title')}
                message={`${t('provider_modal.conflict_msg')}\n\n${pendingConflict?.info.name ?? ''} (${pendingConflict?.info.id ?? ''})`}
                onConfirm={confirmConflictSave}
                onCancel={() => setPendingConflict(null)}
                confirmText={t('provider_modal.conflict_save')}
                cancelText={t('common.cancel')}
            />
        </>
    );
}
