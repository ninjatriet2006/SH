/*
[INTEGRITY NOTES]
- Mục đích: Chọn model cho một provider sau khi quét từ API, kèm khả năng
  (capability) của từng model.
- Trách nhiệm: Hiển thị danh sách model, tick/bỏ tick, tìm kiếm, chỉnh
  `tool_call`/`reasoning`/`interleaved` cho model reasoning, và LƯU danh sách
  cuối cùng.
- Tương tác: `store/useProviderStore.ts` (`scanModels`, `applyModels`).

Điểm quan trọng:
  - Model "stale" (còn trong config nhưng provider không còn hỗ trợ) được tách
    thành khối riêng có cảnh báo — bỏ tick là xoá khỏi config.
  - Capability chỉ được gửi cho model người dùng đã CHỈNH (dirty); model chưa
    đụng thì backend giữ nguyên entry cũ → round-trip không mất trường
    (case `hy3` cần `tool_call`/`reasoning`/`interleaved.reasoning_content`).
*/

import { useEffect, useMemo, useState } from 'react';
import { X, AlertTriangle, Search, Settings2 } from 'lucide-react';
import type { ProviderView, ScannedModel, ModelCaps } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

interface ModelsModalProps {
    isOpen: boolean;
    provider: ProviderView | null;
    onClose: () => void;
    onScan: (providerId: string) => Promise<ScannedModel[]>;
    onApply: (providerId: string, selected: string[], caps?: Record<string, ModelCaps>) => Promise<void>;
}

/** Trạng thái capability trên UI cho một model. */
interface CapsState {
    tool_call: boolean;
    reasoning: boolean;
    interleaved: string;
    /** `true` khi người dùng đã chỉnh — chỉ model dirty mới được gửi xuống. */
    dirty: boolean;
}

export function ModelsModal({ isOpen, provider, onClose, onScan, onApply }: ModelsModalProps) {
    const { t } = useTranslation();
    const [models, setModels] = useState<ScannedModel[]>([]);
    const [checked, setChecked] = useState<Set<string>>(new Set());
    const [caps, setCaps] = useState<Record<string, CapsState>>({});
    /** Model đang mở mục capability. */
    const [expanded, setExpanded] = useState<Set<string>>(new Set());
    const [query, setQuery] = useState('');
    const [isScanning, setIsScanning] = useState(false);
    const [isSaving, setIsSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!isOpen || !provider) return;
        let cancelled = false;
        setIsScanning(true);
        setError(null);
        setQuery('');
        setModels([]);
        setExpanded(new Set());

        (async () => {
            try {
                const list = await onScan(provider.id);
                if (cancelled) return;
                setModels(list);
                // Mặc định giữ đúng những gì đang có trong config: model mới chưa
                // tick (người dùng chủ động thêm), model stale vẫn tick để không
                // xoá ngoài ý muốn — họ phải tự bỏ tick.
                setChecked(new Set(list.filter(m => m.in_config).map(m => m.id)));
                // Prefill capability từ config hiện có (chưa dirty).
                const next: Record<string, CapsState> = {};
                for (const m of list) {
                    next[m.id] = {
                        tool_call: m.caps?.tool_call ?? false,
                        reasoning: m.caps?.reasoning ?? false,
                        interleaved: m.caps?.interleaved ?? '',
                        dirty: false,
                    };
                }
                setCaps(next);
            } catch (err) {
                if (cancelled) return;
                setError(err instanceof Error ? err.message : String(err));
            } finally {
                if (!cancelled) setIsScanning(false);
            }
        })();

        return () => { cancelled = true; };
    }, [isOpen, provider, onScan]);

    const filtered = useMemo(() => {
        const q = query.trim().toLowerCase();
        if (!q) return models;
        return models.filter(m => m.id.toLowerCase().includes(q));
    }, [models, query]);

    const fresh = filtered.filter(m => !m.stale);
    const stale = filtered.filter(m => m.stale);

    if (!isOpen || !provider) return null;

    const toggle = (id: string) => {
        const next = new Set(checked);
        if (next.has(id)) next.delete(id); else next.add(id);
        setChecked(next);
    };

    const toggleExpanded = (id: string) => {
        const next = new Set(expanded);
        if (next.has(id)) next.delete(id); else next.add(id);
        setExpanded(next);
    };

    const toggleAllVisible = (on: boolean) => {
        const next = new Set(checked);
        for (const m of filtered) {
            if (on) next.add(m.id); else next.delete(m.id);
        }
        setChecked(next);
    };

    const updateCaps = (id: string, patch: Partial<Omit<CapsState, 'dirty'>>) => {
        setCaps(prev => ({
            ...prev,
            [id]: { ...prev[id], ...patch, dirty: true },
        }));
    };

    const handleApply = async () => {
        setIsSaving(true);
        try {
            // Chỉ gửi capability của model người dùng đã chỉnh — model còn lại
            // backend giữ nguyên entry (không ghi đè field chưa đặt).
            const dirtyCaps: Record<string, ModelCaps> = {};
            for (const [id, c] of Object.entries(caps)) {
                if (!c.dirty || !checked.has(id)) continue;
                dirtyCaps[id] = {
                    tool_call: c.tool_call,
                    reasoning: c.reasoning,
                    interleaved: c.interleaved.trim() || null,
                };
            }
            await onApply(provider.id, [...checked], Object.keys(dirtyCaps).length > 0 ? dirtyCaps : undefined);
            onClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : String(err));
        } finally {
            setIsSaving(false);
        }
    };

    const capsEditor = (m: ScannedModel) => {
        const c = caps[m.id];
        if (!c) return null;
        return (
            <div
                style={{
                    display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'center',
                    padding: '0.35rem 0.5rem 0.35rem 2.2rem', fontSize: '0.8rem',
                    color: 'var(--text-secondary)',
                }}
            >
                <label style={{ display: 'flex', gap: '0.35rem', alignItems: 'center', cursor: 'pointer' }}>
                    <input
                        type="checkbox"
                        checked={c.tool_call}
                        onChange={e => updateCaps(m.id, { tool_call: e.target.checked })}
                        style={{ width: '13px', height: '13px', cursor: 'pointer' }}
                    />
                    {t('models.cap_tool_call')}
                </label>
                <label style={{ display: 'flex', gap: '0.35rem', alignItems: 'center', cursor: 'pointer' }}>
                    <input
                        type="checkbox"
                        checked={c.reasoning}
                        onChange={e => updateCaps(m.id, { reasoning: e.target.checked })}
                        style={{ width: '13px', height: '13px', cursor: 'pointer' }}
                    />
                    {t('models.cap_reasoning')}
                </label>
                <label style={{ display: 'flex', gap: '0.35rem', alignItems: 'center' }}>
                    {t('models.cap_interleaved')}
                    <input
                        type="text"
                        className="input-field"
                        style={{ padding: '0.2rem 0.4rem', fontSize: '0.78rem', width: '170px' }}
                        value={c.interleaved}
                        onChange={e => updateCaps(m.id, { interleaved: e.target.value })}
                        placeholder="reasoning_content"
                    />
                </label>
            </div>
        );
    };

    const row = (m: ScannedModel) => {
        const c = caps[m.id];
        const hasCaps = c && (c.tool_call || c.reasoning || c.interleaved.trim());
        return (
            <div key={m.id}>
                <div
                    style={{
                        display: 'flex', alignItems: 'center', gap: '0.5rem',
                        padding: '0.35rem 0.5rem', borderRadius: '4px', cursor: 'pointer',
                        background: checked.has(m.id) ? 'rgba(99,102,241,0.12)' : 'transparent',
                    }}
                >
                    <input
                        type="checkbox"
                        checked={checked.has(m.id)}
                        onChange={() => toggle(m.id)}
                        style={{ width: '15px', height: '15px', cursor: 'pointer' }}
                    />
                    <span style={{ fontFamily: 'monospace', fontSize: '0.85rem', flex: 1 }}>{m.id}</span>
                    {hasCaps && (
                        <span className="badge badge-active" style={{ fontSize: '0.6rem' }}>
                            {t('models.cap_badge')}
                        </span>
                    )}
                    {m.stale && (
                        <span className="badge badge-inactive" style={{ color: 'var(--danger)', borderColor: 'var(--danger)' }}>
                            {t('models.stale_badge')}
                        </span>
                    )}
                    <button
                        type="button"
                        className="btn"
                        style={{ padding: '0.15rem 0.3rem', background: 'transparent', color: 'var(--text-secondary)' }}
                        onClick={() => toggleExpanded(m.id)}
                        title={t('models.cap_title')}
                    >
                        <Settings2 size={14} />
                    </button>
                </div>
                {expanded.has(m.id) && capsEditor(m)}
            </div>
        );
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '620px' }}>
                <button
                    onClick={onClose}
                    style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}
                >
                    <X size={20} />
                </button>

                <h3>{t('models.title')} {provider.name}</h3>

                {isScanning ? (
                    <p style={{ color: 'var(--text-secondary)' }}>{t('models.scanning')}</p>
                ) : error ? (
                    <div style={{ padding: '0.75rem', background: 'rgba(239,68,68,0.12)', borderLeft: '3px solid var(--danger)', borderRadius: '4px' }}>
                        {error}
                    </div>
                ) : models.length === 0 ? (
                    <p style={{ color: 'var(--text-secondary)' }}>{t('models.none')}</p>
                ) : (
                    <>
                        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginBottom: '0.75rem', flexWrap: 'wrap' }}>
                            <div style={{ position: 'relative', flex: 1, minWidth: '180px' }}>
                                <Search size={14} style={{ position: 'absolute', left: '8px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)' }} />
                                <input
                                    type="text"
                                    className="input-field"
                                    style={{ paddingLeft: '28px' }}
                                    value={query}
                                    onChange={e => setQuery(e.target.value)}
                                    placeholder={t('models.search')}
                                />
                            </div>
                            <button type="button" className="btn" style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem', background: 'rgba(255,255,255,0.08)' }} onClick={() => toggleAllVisible(true)}>
                                {t('common.select_all')}
                            </button>
                            <button type="button" className="btn" style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem', background: 'rgba(255,255,255,0.08)' }} onClick={() => toggleAllVisible(false)}>
                                {t('common.deselect_all')}
                            </button>
                        </div>

                        <div style={{ maxHeight: '46vh', overflowY: 'auto', paddingRight: '4px' }}>
                            {fresh.map(row)}

                            {stale.length > 0 && (
                                <>
                                    <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'flex-start', margin: '1rem 0 0.5rem', padding: '0.6rem 0.75rem', background: 'rgba(239,68,68,0.12)', borderLeft: '3px solid var(--danger)', borderRadius: '4px', fontSize: '0.8rem' }}>
                                        <AlertTriangle size={16} style={{ color: 'var(--danger)', flexShrink: 0, marginTop: '2px' }} />
                                        <span>{t('models.stale_warning')}</span>
                                    </div>
                                    {stale.map(row)}
                                </>
                            )}
                        </div>

                        <div style={{ marginTop: '0.75rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                            {checked.size} / {models.length} {t('models.selected_count')}
                        </div>
                    </>
                )}

                <div className="modal-actions">
                    <button type="button" className="btn" onClick={onClose}>{t('common.cancel')}</button>
                    <button
                        type="button"
                        className="btn btn-primary"
                        onClick={handleApply}
                        disabled={isScanning || isSaving || models.length === 0}
                    >
                        {isSaving ? t('common.loading') : t('models.apply')}
                    </button>
                </div>
            </div>
        </div>
    );
}
