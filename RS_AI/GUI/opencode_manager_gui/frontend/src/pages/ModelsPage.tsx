/*
[INTEGRITY NOTES]
- Mục đích: Tab "So sánh model" — ma trận toàn bộ model trong config: khả năng
  (tool/reasoning/vision), giới hạn (context/output), giá (USD/1M token) và
  chọn model chính (⭐) cho OpenCode.
- Trách nhiệm: Lọc (theo chữ/provider/khả năng), sắp xếp theo cột, đặt/bỏ
  model chính. Dữ liệu đã làm giàu từ cache models.dev ở backend.
- Tương tác: `bridge/models_bridge.ts`, `store/useProviderStore.ts` (làm mới
  badge model chính của trang Provider sau khi đổi).

Phân loại: "—" = CHƯA KHAI trong config và models.dev không có metadata —
OpenCode sẽ tự dùng mặc định của model. Giá chỉ có khi models.dev biết model.
*/

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { RefreshCw, Star, Search, Gavel, Trash2, Wrench } from 'lucide-react';
import type { ModelMatrixRow, ArbiterState, ArbiterVerdict, RecommendationView, RecommendedModel } from '../../../bridge/types';
import { listModelMatrix, setPrimaryModel, syncLimitsFromDev } from '../../../bridge/models_bridge';
import { getArbiterState, runArbiterEvaluation, clearArbiterHistory, recommendModels } from '../../../bridge/arbiter_bridge';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';
import { ConfirmModal } from '../components/ConfirmModal';
import { SearchableSelect, type SearchableOption } from '../components/SearchableSelect';
import { safetyFreedomView } from '../utils/safetyFreedom';
import { OpenCodeControls } from '../components/OpenCodeControls';

type SortKey = 'provider' | 'model' | 'tools' | 'reasoning' | 'vision' | 'ctx' | 'out' | 'in' | 'outusd';
type ArbiterSortKey = 'model' | 'coding' | 'reasoning' | 'tools' | 'vision' | 'safety' | 'overall';
type ViewMode = 'meta' | 'arbiter' | 'recommend';

/** Event `arbiter://progress` từ backend — log từng lô request của arbiter. */
interface ArbiterProgressEvent {
    stage: 'start' | 'batch' | 'scored' | 'warn' | 'error' | 'done';
    message: string;
    current: number;
    total: number;
    scored: number;
}
type TaskKey = 'coding' | 'agentic' | 'reasoning' | 'vision' | 'long_context' | 'value';

export function ModelsPage() {
    const { t } = useTranslation();
    const fetchProviders = useProviderStore(state => state.fetchProviders);

    const [rows, setRows] = useState<ModelMatrixRow[]>([]);
    const [query, setQuery] = useState('');
    const [providerFilter, setProviderFilter] = useState('');
    const [needTools, setNeedTools] = useState(false);
    const [needReasoning, setNeedReasoning] = useState(false);
    const [needVision, setNeedVision] = useState(false);
    const [sortKey, setSortKey] = useState<SortKey>('provider');
    const [sortAsc, setSortAsc] = useState(true);
    const [isLoading, setIsLoading] = useState(true);
    const [notice, setNotice] = useState<string | null>(null);

    // ===== ARBITER (trọng tài chấm điểm) =====
    const [view, setView] = useState<ViewMode>('meta');
    const [arbiterState, setArbiterState] = useState<ArbiterState | null>(null);
    const [arbiterLoaded, setArbiterLoaded] = useState(false);
    const [arbiterChoice, setArbiterChoice] = useState('');
    const [isEvaluating, setIsEvaluating] = useState(false);
    const [arbiterSort, setArbiterSort] = useState<ArbiterSortKey>('overall');
    const [arbiterAsc, setArbiterAsc] = useState(false);
    /** Xác nhận đồng bộ limit về models.dev (chỉ model đang lệch). */
    const [confirmSync, setConfirmSync] = useState(false);

    // ===== RECOMMEND (xếp hạng theo tác vụ) =====
    const [recTask, setRecTask] = useState<TaskKey>('coding');
    const [recView, setRecView] = useState<RecommendationView | null>(null);
    const [isRecommending, setIsRecommending] = useState(false);

    // ===== Tiến trình chấm của arbiter (event từ backend, LIVE) =====
    const [evalLog, setEvalLog] = useState<ArbiterProgressEvent[]>([]);
    const logRef = useRef<HTMLDivElement>(null);

    // ===== Tìm kiếm nhanh =====
    /** Lọc bảng điểm arbiter. */
    const [arbiterQuery, setArbiterQuery] = useState('');

    const reload = useCallback(async () => {
        setIsLoading(true);
        try {
            setRows(await listModelMatrix());
            setNotice(null);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsLoading(false);
        }
    }, []);

    const reloadArbiter = useCallback(async () => {
        try {
            const st = await getArbiterState();
            setArbiterState(st);
            // Mặc định: giữ lựa chọn cũ (còn hợp lệ) → arbiter lần trước → model
            // chính. Choice cũ/st.arbiter KHÔNG còn trong candidates (model bị
            // xoá, hoặc lịch sử vừa bị clear) phải bị bỏ — không thì Evaluate
            // dùng lựa chọn lệch với trạng thái thật.
            const keys = new Set(st.candidates.map(c => `${c.provider_id}/${c.model_id}`));
            const primary = st.candidates.find(c => c.is_primary);
            setArbiterChoice(prev => {
                if (prev && keys.has(prev)) return prev;
                if (st.arbiter && keys.has(st.arbiter)) return st.arbiter;
                if (primary) return `${primary.provider_id}/${primary.model_id}`;
                return '';
            });
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setArbiterLoaded(true);
        }
    }, []);

    useEffect(() => {
        reload();
        // Panel arbiter luôn hiển thị nên trạng thái của nó phải được nạp nền.
        reloadArbiter();
    }, [reload, reloadArbiter]);

    // Lắng nghe tiến trình chấm của arbiter (backend phát từng lô) — log hiện
    // LIVE vì lệnh giờ chạy async (không đóng băng main thread).
    useEffect(() => {
        let un: UnlistenFn | null = null;
        let alive = true;
        listen<ArbiterProgressEvent>('arbiter://progress', e => {
            setEvalLog(prev => [...prev.slice(-100), e.payload]);
        }).then(fn => {
            if (alive) un = fn;
            else fn();
        }).catch(err => console.error('Lắng nghe arbiter://progress lỗi:', err));
        return () => {
            alive = false;
            un?.();
        };
    }, []);

    // Tự cuộn log xuống dòng mới nhất.
    useEffect(() => {
        if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
    }, [evalLog]);

    const providerIds = useMemo(
        () => [...new Set(rows.map(r => r.provider_id))].sort(),
        [rows],
    );

    const primaryRow = useMemo(() => rows.find(r => r.is_primary) ?? null, [rows]);

    const stats = useMemo(() => ({
        total: rows.length,
        tools: rows.filter(r => r.tool_call === true).length,
        reasoning: rows.filter(r => r.reasoning === true).length,
        vision: rows.filter(r => r.vision === true).length,
        enriched: rows.filter(r => r.enriched).length,
        conflicts: rows.filter(r => r.limit_conflict).length,
    }), [rows]);

    const visible = useMemo(() => {
        const q = query.trim().toLowerCase();
        return rows.filter(r => {
            if (providerFilter && r.provider_id !== providerFilter) return false;
            if (needTools && r.tool_call !== true) return false;
            if (needReasoning && r.reasoning !== true) return false;
            if (needVision && r.vision !== true) return false;
            if (!q) return true;
            return r.provider_id.toLowerCase().includes(q)
                || r.provider_name.toLowerCase().includes(q)
                || r.model_id.toLowerCase().includes(q)
                || r.display_name.toLowerCase().includes(q);
        });
    }, [rows, query, providerFilter, needTools, needReasoning, needVision]);

    const sorted = useMemo(() => {
        const val = (r: ModelMatrixRow): number | string | null => {
            switch (sortKey) {
                case 'model': return r.model_id.toLowerCase();
                case 'tools': return r.tool_call === null ? null : (r.tool_call ? 1 : 0);
                case 'reasoning': return r.reasoning === null ? null : (r.reasoning ? 1 : 0);
                case 'vision': return r.vision === null ? null : (r.vision ? 1 : 0);
                case 'ctx': return r.context;
                case 'out': return r.output;
                case 'in': return r.price_input;
                case 'outusd': return r.price_output;
                default: return r.provider_id.toLowerCase();
            }
        };
        const arr = [...visible];
        arr.sort((a, b) => {
            const va = val(a);
            const vb = val(b);
            // null (chưa biết) luôn xếp cuối bất kể hướng sắp xếp.
            if (va === null && vb === null) return 0;
            if (va === null) return 1;
            if (vb === null) return -1;
            const cmp = typeof va === 'string'
                ? (va as string).localeCompare(vb as string)
                : (va as number) - (vb as number);
            return sortAsc ? cmp : -cmp;
        });
        return arr;
    }, [visible, sortKey, sortAsc]);

    const toggleSort = (key: SortKey) => {
        if (key === sortKey) setSortAsc(a => !a);
        else {
            setSortKey(key);
            setSortAsc(true);
        }
    };
    const sortArrow = (key: SortKey) => (sortKey === key ? (sortAsc ? ' ↑' : ' ↓') : '');

    const handleSetPrimary = async (row: ModelMatrixRow) => {
        try {
            await setPrimaryModel(row.provider_id, row.model_id);
            await reload();
            // Badge ⭐ ở trang Provider phải cập nhật theo.
            await fetchProviders();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleSyncLimits = async () => {
        setConfirmSync(false);
        try {
            const n = await syncLimitsFromDev();
            setNotice(`${t('models.sync_done')}: ${n}`);
            await reload();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleClearPrimary = async () => {
        try {
            await setPrimaryModel(null, null);
            await reload();
            await fetchProviders();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    // ===== ARBITER: chạy đánh giá / xoá lịch sử / sắp xếp điểm =====
    const handleEvaluate = async () => {
        const choice = arbiterChoice.split('/');
        // Tách "pid/mid" ở "/" ĐẦU (mid có thể chứa "/" — model CKey).
        if (choice.length < 2) {
            setNotice(t('models.arbiter_pick_first'));
            return;
        }
        const [pid, ...rest] = choice;
        const mid = rest.join('/');
        setIsEvaluating(true);
        setNotice(null);
        setEvalLog([]);
        try {
            await runArbiterEvaluation(pid, mid);
            await reloadArbiter();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsEvaluating(false);
        }
    };

    // Nạp ranking cho tác vụ (task đổi / sau khi judge chạy thêm).
    const reloadRecommend = useCallback(async (task: TaskKey) => {
        setIsRecommending(true);
        try {
            setRecView(await recommendModels(task));
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsRecommending(false);
        }
    }, []);

    useEffect(() => {
        if (view === 'recommend' && arbiterLoaded) reloadRecommend(recTask);
    }, [view, recTask, reloadRecommend, arbiterLoaded, arbiterState]);

    const handleClearArbiter = async () => {
        try {
            await clearArbiterHistory();
            await reloadArbiter();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const sortedVerdicts = useMemo(() => {
        if (!arbiterState) return [];
        const q = arbiterQuery.trim().toLowerCase();
        const filtered = q
            ? arbiterState.verdicts.filter(v => v.model.toLowerCase().includes(q))
            : arbiterState.verdicts;
        const arr = [...filtered];
        const val = (v: ArbiterVerdict): number | string => {
            switch (arbiterSort) {
                case 'model': return v.model.toLowerCase();
                case 'coding': return v.coding;
                case 'reasoning': return v.reasoning;
                case 'tools': return v.tool_use;
                case 'vision': return v.vision;
                case 'safety': return v.safety_freedom ?? -1;
                default: return v.overall;
            }
        };
        arr.sort((a, b) => {
            const va = val(a);
            const vb = val(b);
            const cmp = typeof va === 'string'
                ? (va as string).localeCompare(vb as string)
                : (va as number) - (vb as number);
            return arbiterAsc ? cmp : -cmp;
        });
        return arr;
    }, [arbiterState, arbiterSort, arbiterAsc, arbiterQuery]);

    /** Map "pid/mid" → verdict (tra ước lượng limit của arbiter cho tooltip). */
    const verdictMap = useMemo(() => {
        const m = new Map<string, ArbiterVerdict>();
        for (const v of arbiterState?.verdicts ?? []) m.set(v.model, v);
        return m;
    }, [arbiterState]);

    /** Tooltip limit: liệt kê MỌI nguồn đang có (đối chiếu chéo). */
    const limitTip = (r: ModelMatrixRow, field: 'context' | 'output'): string => {
        const parts: string[] = [];
        const dev = field === 'context' ? r.dev_context : r.dev_output;
        const heur = field === 'context' ? r.heur_context : r.heur_output;
        const av = verdictMap.get(`${r.provider_id}/${r.model_id}`);
        const arb = av ? (field === 'context' ? av.context : av.output) : null;
        if (dev != null) parts.push(`${t('models.dev_says')}: ${fmtNum(dev)}`);
        if (heur != null) parts.push(`${t('models.name_says')}: ${fmtNum(heur)}`);
        if (arb != null) parts.push(`${t('models.arbiter_says')}: ${fmtNum(arb)}`);
        return parts.join(' · ');
    };

    /** Ứng viên trọng tài (SearchableSelect tự lọc khi gõ — catalog builtin
     *  có thể dài hàng trăm model). */
    const arbiterCandidates: SearchableOption[] = useMemo(
        () =>
            (arbiterState?.candidates ?? []).map(c => ({
                value: `${c.provider_id}/${c.model_id}`,
                label: `${c.provider_id}/${c.model_id}`,
                hint: c.is_primary ? '⭐' : undefined,
            })),
        [arbiterState],
    );

    const toggleArbiterSort = (key: ArbiterSortKey) => {
        if (key === arbiterSort) setArbiterAsc(a => !a);
        else {
            setArbiterSort(key);
            setArbiterAsc(false); // điểm cao xuống dưới cùng → mặc định giảm dần
        }
    };
    const arbiterArrow = (key: ArbiterSortKey) => (arbiterSort === key ? (arbiterAsc ? ' ↑' : ' ↓') : '');

    /** Arbiter vs nguồn mạnh (config/dev): lệch quá 25% mới đánh dấu — ước
     * lượng AI xê xích chút đỉnh là bình thường, chỉ bất đồng lớn mới đáng ⚠. */
    const limitMismatch = (arb: number | null, known: number | null): boolean => {
        if (arb == null || known == null || known === 0) return false;
        const ratio = arb / known;
        return ratio < 0.75 || ratio > 1.34;
    };

    /** Giá trị "đã biết" của model (theo ma trận) để so với arbiter. */
    const knownOf = (modelKey: string): { ctx: number | null; out: number | null } => {
        const r = rows.find(x => `${x.provider_id}/${x.model_id}` === modelKey);
        return { ctx: r?.dev_context ?? r?.context ?? null, out: r?.dev_output ?? r?.output ?? null };
    };

    /** Nhãn điều chỉnh theo kind (front-end định dạng, backend chỉ gửi kind+delta). */
    const adjLabel = (a: RecommendedModel['adjustments'][number], m: RecommendedModel): string => {
        switch (a.kind) {
            case 'ctx': return `${t('models.adj_ctx')} (${fmtNum(m.context)})`;
            case 'output': return t('models.adj_output');
            case 'tools': return t('models.adj_tools');
            case 'coding_bonus': return t('models.adj_coding_bonus');
            case 'price': return t('models.adj_price');
            default: return a.kind;
        }
    };

    const tierStyle = (tier: string): React.CSSProperties => {
        switch (tier) {
            case 'S': return { color: 'var(--primary)', fontWeight: 700 };
            case 'A': return { color: 'var(--success)', fontWeight: 600 };
            case 'B': return { color: 'var(--text-secondary)' };
            default: return { color: 'var(--text-secondary)', opacity: 0.7 };
        }
    };

    const fmtNum = (n: number | null) => (n != null ? n.toLocaleString('vi-VN') : '—');
    // Giá 0 là model FREE (có thật trên models.dev) — phải hiện 0, chỉ null là "không biết".
    const fmtPrice = (n: number | null) =>
        (n != null ? (n < 1 ? n.toFixed(3) : n.toFixed(2)) : '—');
    const capCell = (v: boolean | null) =>
        v === null ? <span style={{ color: 'var(--text-secondary)' }}>—</span>
        : v ? <span style={{ color: 'var(--success)' }}>✓</span>
        : <span style={{ color: 'var(--danger)' }}>✗</span>;

    return (
        <div className="animate-fade-in">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', flexWrap: 'wrap', gap: '1rem' }}>
                <h1>{t('models.matrix_title')}</h1>
                <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                    {stats.conflicts > 0 && (
                        <button
                            className="btn"
                            style={{ color: 'var(--danger)', border: '1px solid var(--danger)' }}
                            onClick={() => setConfirmSync(true)}
                            title={t('models.sync_limits_hint')}
                        >
                            <Wrench size={18} /> {t('models.sync_limits')} ({stats.conflicts})
                        </button>
                    )}
                    <button
                        className="btn"
                        style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }}
                        onClick={reload}
                        disabled={isLoading}
                    >
                        <RefreshCw size={18} /> {t('common.refresh')}
                    </button>
                </div>
            </div>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>{t('models.matrix_desc')}</p>

            <OpenCodeControls onError={setNotice} />

            {notice && (
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(239,68,68,0.12)', borderLeft: '4px solid var(--danger)', borderRadius: '4px', fontSize: '0.9rem' }}>
                    <span>{notice}</span>
                    <button className="btn" style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }} onClick={() => setNotice(null)}>✕</button>
                </div>
            )}

            {/* Model chính hiện tại */}
            <div className="glass-panel" style={{ marginBottom: '1rem', padding: '0.75rem 1rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.75rem' }}>
                <div style={{ fontSize: '0.9rem' }}>
                    <Star size={15} style={{ verticalAlign: '-2px', color: primaryRow ? 'var(--primary)' : 'var(--text-secondary)' }} />{' '}
                    {t('models.primary_current')}:{' '}
                    {primaryRow ? (
                        <code style={{ fontFamily: 'monospace', color: 'var(--primary)' }}>
                            {primaryRow.provider_id}/{primaryRow.model_id}
                        </code>
                    ) : (
                        <span style={{ color: 'var(--text-secondary)' }}>{t('models.primary_none')}</span>
                    )}
                </div>
                {primaryRow && (
                    <button className="btn" style={{ color: 'var(--danger)', border: '1px solid var(--danger)' }} onClick={handleClearPrimary}>
                        {t('models.clear_primary')}
                    </button>
                )}
            </div>

            {/* Bộ lọc */}
            <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'end', marginBottom: '1rem' }}>
                <div style={{ position: 'relative', minWidth: '220px', flex: 1, maxWidth: '360px' }}>
                    <Search size={16} style={{ position: 'absolute', left: '10px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)' }} />
                    <input
                        type="text"
                        className="input-field"
                        style={{ paddingLeft: '32px' }}
                        value={query}
                        onChange={e => setQuery(e.target.value)}
                        placeholder={t('models.filter_placeholder')}
                    />
                </div>
                <label style={{ display: 'flex', flexDirection: 'column', gap: '0.3rem', fontSize: '0.85rem' }}>
                    {t('models.filter_provider')}
                    <SearchableSelect
                        options={providerIds.map(pid => ({ value: pid, label: pid }))}
                        value={providerFilter}
                        onChange={setProviderFilter}
                        placeholder={t('models.filter_all')}
                        ariaLabel={t('models.filter_provider')}
                        clearable
                        style={{ width: 'auto', minWidth: '160px' }}
                    />
                </label>
                <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', fontSize: '0.85rem', cursor: 'pointer' }}>
                    <input type="checkbox" checked={needTools} onChange={e => setNeedTools(e.target.checked)} style={{ width: '14px', height: '14px', cursor: 'pointer' }} />
                    {t('models.need_tools')}
                </label>
                <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', fontSize: '0.85rem', cursor: 'pointer' }}>
                    <input type="checkbox" checked={needReasoning} onChange={e => setNeedReasoning(e.target.checked)} style={{ width: '14px', height: '14px', cursor: 'pointer' }} />
                    {t('models.need_reasoning')}
                </label>
                <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', fontSize: '0.85rem', cursor: 'pointer' }}>
                    <input type="checkbox" checked={needVision} onChange={e => setNeedVision(e.target.checked)} style={{ width: '14px', height: '14px', cursor: 'pointer' }} />
                    {t('models.need_vision')}
                </label>
            </div>

            {/* Thống kê nhanh (phân loại tổng quan) */}
            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.75rem', display: 'flex', gap: '1rem', flexWrap: 'wrap' }}>
                <span>{t('models.stat_total')}: <strong>{stats.total}</strong></span>
                <span>🛠 {t('models.col_tools')}: <strong>{stats.tools}</strong></span>
                <span>💭 {t('models.col_reasoning')}: <strong>{stats.reasoning}</strong></span>
                <span>🖼 {t('models.col_vision')}: <strong>{stats.vision}</strong></span>
                <span title={t('models.enriched_hint')}>models.dev: {stats.enriched}</span>
                {stats.conflicts > 0 && (
                    <span style={{ color: 'var(--danger)' }}>⚠ {t('models.limit_conflicts')}: <strong>{stats.conflicts}</strong></span>
                )}
            </div>

            {/* ===== ARBITER: trọng tài chấm điểm bằng dữ liệu, đồng thuận 5 lần ===== */}
            <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.75rem' }}>
                    <h3 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                        <Gavel size={16} /> {t('models.arbiter_title')}
                    </h3>
                    <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', flexWrap: 'wrap' }}>
                        <SearchableSelect
                            options={arbiterCandidates}
                            value={arbiterChoice}
                            onChange={setArbiterChoice}
                            placeholder={`${t('models.arbiter_pick_placeholder')} (${arbiterCandidates.length})`}
                            ariaLabel={t('models.arbiter_pick_placeholder')}
                            style={{ minWidth: '280px', fontSize: '0.85rem' }}
                        />
                        <button className="btn btn-primary" onClick={handleEvaluate} disabled={isEvaluating || !arbiterChoice}>
                            <Gavel size={16} /> {isEvaluating ? t('models.arbiter_running') : t('models.arbiter_run')}
                        </button>
                        {arbiterState && arbiterState.run_count > 0 && (
                            <button className="btn" style={{ color: 'var(--danger)', border: '1px solid var(--danger)' }} onClick={handleClearArbiter} title={t('models.arbiter_clear')}>
                                <Trash2 size={16} />
                            </button>
                        )}
                    </div>
                </div>
                <div style={{ fontSize: '0.82rem', color: 'var(--text-secondary)', marginTop: '0.5rem', display: 'flex', gap: '1.25rem', flexWrap: 'wrap' }}>
                    <span>
                        {t('models.arbiter_runs')}: <strong>{arbiterState?.run_count ?? 0}/5</strong>
                        {arbiterState?.last_run && ` · ${t('models.arbiter_last')} ${arbiterState.last_run.at}`}
                    </span>
                    <span>{t('models.arbiter_count')}: <strong>{arbiterState?.verdicts.length ?? 0}</strong></span>
                </div>
                <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.4rem' }}>
                    {t('models.arbiter_hint')}
                </small>

                {/* Log tiến trình LIVE: từng lô request / cảnh báo / lỗi từng lô */}
                {(isEvaluating || evalLog.length > 0) && (
                    <div style={{ marginTop: '0.5rem', border: '1px solid var(--border)', borderRadius: '4px', padding: '0.5rem 0.75rem', background: 'rgba(0,0,0,0.25)' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.35rem', fontSize: '0.8rem' }}>
                            <strong>{isEvaluating ? `⏳ ${t('models.arbiter_running')}` : t('models.arbiter_log_title')}</strong>
                            {isEvaluating && evalLog.length > 0 && evalLog[evalLog.length - 1].total > 0 && (
                                <span style={{ color: 'var(--text-secondary)' }}>
                                    {t('models.arbiter_batch')}: {evalLog[evalLog.length - 1].current}/{evalLog[evalLog.length - 1].total}
                                    {' · '}{t('models.arbiter_scored')}: {evalLog[evalLog.length - 1].scored}
                                </span>
                            )}
                        </div>
                        <div
                            ref={logRef}
                            style={{ maxHeight: '130px', overflowY: 'auto', fontFamily: 'monospace', fontSize: '0.72rem', lineHeight: 1.5 }}
                        >
                            {evalLog.map((l, i) => (
                                <div
                                    key={i}
                                    style={{
                                        color: l.stage === 'error' ? 'var(--danger)' : l.stage === 'warn' ? '#f59e0b' : l.stage === 'done' ? 'var(--success)' : 'var(--text-secondary)',
                                    }}
                                >
                                    {l.stage === 'error' ? '✖' : l.stage === 'warn' ? '⚠' : l.stage === 'done' ? '✔' : '·'} {l.message}
                                </div>
                            ))}
                        </div>
                    </div>
                )}
            </div>

            {/* Chế độ xem: metadata khai báo / điểm arbiter */}
            <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.75rem' }}>
                <button
                    className={`btn ${view === 'meta' ? 'btn-primary' : ''}`}
                    style={view !== 'meta' ? { background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' } : undefined}
                    onClick={() => setView('meta')}
                >
                    {t('models.view_meta')}
                </button>
                <button
                    className={`btn ${view === 'arbiter' ? 'btn-primary' : ''}`}
                    style={view !== 'arbiter' ? { background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' } : undefined}
                    onClick={() => setView('arbiter')}
                >
                    {t('models.view_arbiter')} {arbiterState?.run_count ? `(${arbiterState.verdicts.length})` : ''}
                </button>
                <button
                    className={`btn ${view === 'recommend' ? 'btn-primary' : ''}`}
                    style={view !== 'recommend' ? { background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' } : undefined}
                    onClick={() => setView('recommend')}
                >
                    🏆 {t('models.view_recommend')}
                </button>
            </div>

            {view === 'recommend' ? (
                <div className="glass-panel">
                    {/* Bộ chọn tác vụ — "tôi cần gì hôm nay?" */}
                    <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', marginBottom: '0.75rem' }}>
                        {(['coding', 'agentic', 'reasoning', 'vision', 'long_context', 'value'] as TaskKey[]).map(task => (
                            <button
                                key={task}
                                className={`btn ${recTask === task ? 'btn-primary' : ''}`}
                                style={recTask !== task ? { background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' } : undefined}
                                onClick={() => setRecTask(task)}
                            >
                                {t(`models.task_${task}`)}
                            </button>
                        ))}
                    </div>

                    {isRecommending ? (
                        <p>{t('common.loading')}</p>
                    ) : !recView || recView.items.length === 0 ? (
                        <p style={{ color: 'var(--text-secondary)' }}>
                            {t('models.rec_none')}{' '}
                            {(!arbiterState || arbiterState.verdicts.length === 0) && t('models.rec_need_judge')}
                        </p>
                    ) : (
                        <>
                            <div className="table-container">
                                <table>
                                    <thead>
                                        <tr>
                                            <th>#</th>
                                            <th>{t('models.tier')}</th>
                                            <th>{t('models.col_model')}</th>
                                            <th style={{ textAlign: 'right' }}>{t('models.rec_score')}</th>
                                            <th style={{ textAlign: 'right' }}>{t('models.col_ctx')}</th>
                                            <th style={{ textAlign: 'right' }}>{t('models.col_in')}</th>
                                            <th>{t('models.rec_why')}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {recView.items.map((m, i) => (
                                            <tr key={`${m.provider_id}/${m.model_id}`}>
                                                <td style={{ color: 'var(--text-secondary)' }}>{i + 1}</td>
                                                <td style={tierStyle(m.tier)}>{m.tier}</td>
                                                <td>
                                                    <div style={{ fontFamily: 'monospace', fontSize: '0.82rem' }}>{m.model_id}</div>
                                                    <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{m.provider_id}</div>
                                                </td>
                                                <td style={{ textAlign: 'right', fontWeight: 700, color: m.tier === 'S' ? 'var(--primary)' : undefined }}>
                                                    {m.score}
                                                    <span style={{ fontSize: '0.65rem', color: 'var(--text-secondary)', fontWeight: 400 }}> /{m.base}</span>
                                                </td>
                                                <td style={{ fontSize: '0.8rem', textAlign: 'right' }}>{fmtNum(m.context)}</td>
                                                <td style={{ fontSize: '0.8rem', textAlign: 'right' }}>{fmtPrice(m.price_input)}</td>
                                                <td style={{ fontSize: '0.72rem', color: 'var(--text-secondary)', maxWidth: '300px' }}>
                                                    {m.adjustments.length === 0
                                                        ? <span title={t('models.rec_pure')}>{t('models.rec_pure')}</span>
                                                        : m.adjustments.map((a, j) => (
                                                            <span key={j} style={{ marginRight: '0.5rem', whiteSpace: 'nowrap' }}>
                                                                {adjLabel(a, m)} <span style={{ color: a.delta >= 0 ? 'var(--success)' : 'var(--danger)' }}>{a.delta >= 0 ? `+${a.delta}` : a.delta}</span>
                                                            </span>
                                                        ))}
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                            {recView.unevaluated > 0 && (
                                <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                                    {t('models.rec_unevaluated')}: {recView.unevaluated}
                                </small>
                            )}
                            <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.4rem' }}>
                                {t('models.rec_legend')}
                            </small>
                        </>
                    )}
                </div>
            ) : view === 'arbiter' ? (
                <div className="glass-panel">
                    <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: '0.5rem' }}>
                        <div style={{ position: 'relative', minWidth: '240px' }}>
                            <Search size={14} style={{ position: 'absolute', left: '8px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)' }} />
                            <input
                                type="text"
                                className="input-field"
                                style={{ paddingLeft: '28px', padding: '0.3rem 0.5rem', fontSize: '0.82rem' }}
                                value={arbiterQuery}
                                onChange={e => setArbiterQuery(e.target.value)}
                                placeholder={t('models.arbiter_search')}
                            />
                        </div>
                    </div>
                    {sortedVerdicts.length === 0 ? (
                        <p style={{ color: 'var(--text-secondary)' }}>
                            {arbiterQuery.trim() ? t('models.search_none') : t('models.arbiter_none')}
                        </p>
                    ) : (
                        <div className="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleArbiterSort('model')}>{t('models.col_model')}{arbiterArrow('model')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('coding')}>{t('models.arb_coding')}{arbiterArrow('coding')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('reasoning')}>{t('models.arb_reasoning')}{arbiterArrow('reasoning')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('tools')}>{t('models.arb_tools')}{arbiterArrow('tools')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('vision')}>{t('models.arb_vision')}{arbiterArrow('vision')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('safety')} title={t('models.arb_safety_hint')}>{t('models.arb_safety')}{arbiterArrow('safety')}</th>
                                        <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleArbiterSort('overall')}>{t('models.arb_overall')}{arbiterArrow('overall')}</th>
                                        <th style={{ textAlign: 'right' }} title={t('models.arb_limits_hint')}>{t('models.col_ctx')}</th>
                                        <th style={{ textAlign: 'right' }} title={t('models.arb_limits_hint')}>{t('models.col_out')}</th>
                                        <th>{t('models.arb_stability')}</th>
                                        <th>{t('models.arb_note')}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {sortedVerdicts.map(v => (
                                        <tr key={v.model} title={v.note}>
                                            <td style={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>{v.model}</td>
                                            <td style={{ textAlign: 'right' }}>{v.coding}</td>
                                            <td style={{ textAlign: 'right' }}>{v.reasoning}</td>
                                            <td style={{ textAlign: 'right' }}>{v.tool_use}</td>
                                            <td style={{ textAlign: 'right' }}>{v.vision}</td>
                                            <td style={{ textAlign: 'right' }}>
                                                {(() => {
                                                    const safety = safetyFreedomView(v.safety_freedom, v.safety_confidence, v.safety_evidence, t('models.arb_safety_unknown'), t('models.arb_confidence'));
                                                    return <span title={safety.title} style={!safety.known ? { color: 'var(--text-secondary)' } : undefined}>{safety.value}</span>;
                                                })()}
                                            </td>
                                            <td style={{ textAlign: 'right', fontWeight: 700, color: 'var(--primary)' }}>{v.overall}</td>
                                            <td style={{ textAlign: 'right', fontSize: '0.8rem' }}>
                                                {(() => {
                                                    const known = knownOf(v.model);
                                                    const bad = limitMismatch(v.context, known.ctx);
                                                    const tip = bad && known.ctx != null ? `${t('models.dev_says')}: ${fmtNum(known.ctx)}` : undefined;
                                                    return (
                                                        <span style={bad ? { color: 'var(--danger)' } : undefined} title={tip}>
                                                            {v.context != null ? fmtNum(v.context) : '—'}{bad ? ' ⚠' : ''}
                                                        </span>
                                                    );
                                                })()}
                                            </td>
                                            <td style={{ textAlign: 'right', fontSize: '0.8rem' }}>
                                                {(() => {
                                                    const known = knownOf(v.model);
                                                    const bad = limitMismatch(v.output, known.out);
                                                    const tip = bad && known.out != null ? `${t('models.dev_says')}: ${fmtNum(known.out)}` : undefined;
                                                    return (
                                                        <span style={bad ? { color: 'var(--danger)' } : undefined} title={tip}>
                                                            {v.output != null ? fmtNum(v.output) : '—'}{bad ? ' ⚠' : ''}
                                                        </span>
                                                    );
                                                })()}
                                            </td>
                                            <td>
                                                <span className={`badge ${v.stable ? 'badge-active' : 'badge-inactive'}`} style={{ fontSize: '0.65rem' }}>
                                                    {v.stable ? t('models.arb_stable') : t('models.arb_volatile')}
                                                </span>
                                                <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}> {v.runs}/5</span>
                                            </td>
                                            <td style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', maxWidth: '260px' }}>{v.note}</td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                    <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                        {t('models.arbiter_legend')} · {t('models.arb_formula')}: v{arbiterState?.overall_version ?? 2}
                        {arbiterState && ` (${arbiterState.overall_weights.coding}/${arbiterState.overall_weights.reasoning}/${arbiterState.overall_weights.tool_use}/${arbiterState.overall_weights.vision}/${arbiterState.overall_weights.safety_freedom})`}
                    </small>
                </div>
            ) : (
            <div className="glass-panel">
                {isLoading && rows.length === 0 ? (
                    <p>{t('common.loading')}</p>
                ) : sorted.length === 0 ? (
                    <p style={{ color: 'var(--text-secondary)' }}>{t('models.matrix_none')}</p>
                ) : (
                    <div className="table-container">
                        <table>
                            <thead>
                                <tr>
                                    <th style={{ width: '40px' }}>⭐</th>
                                    <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('provider')}>{t('models.col_provider')}{sortArrow('provider')}</th>
                                    <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('model')}>{t('models.col_model')}{sortArrow('model')}</th>
                                    <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('tools')}>{t('models.col_tools')}{sortArrow('tools')}</th>
                                    <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('reasoning')}>{t('models.col_reasoning')}{sortArrow('reasoning')}</th>
                                    <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('vision')}>{t('models.col_vision')}{sortArrow('vision')}</th>
                                    <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleSort('ctx')}>{t('models.col_ctx')}{sortArrow('ctx')}</th>
                                    <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleSort('out')}>{t('models.col_out')}{sortArrow('out')}</th>
                                    <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleSort('in')}>{t('models.col_in')}{sortArrow('in')}</th>
                                    <th style={{ cursor: 'pointer', textAlign: 'right' }} onClick={() => toggleSort('outusd')}>{t('models.col_outusd')}{sortArrow('outusd')}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {sorted.map(r => (
                                    <tr key={`${r.provider_id}/${r.model_id}`}>
                                        <td>
                                            <button
                                                className="btn"
                                                style={{
                                                    padding: '0.15rem 0.3rem', background: 'transparent',
                                                    color: r.is_primary ? 'var(--primary)' : 'var(--text-secondary)',
                                                }}
                                                onClick={() => handleSetPrimary(r)}
                                                title={t('models.set_primary')}
                                            >
                                                <Star size={14} fill={r.is_primary ? 'currentColor' : 'none'} />
                                            </button>
                                        </td>
                                        <td style={{ fontSize: '0.82rem' }}>
                                            <div style={{ fontFamily: 'monospace' }}>{r.provider_id}</div>
                                            <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{r.provider_name}</div>
                                        </td>
                                        <td>
                                            <div style={{ fontFamily: 'monospace', fontSize: '0.82rem' }}>{r.model_id}</div>
                                            {r.display_name !== r.model_id && (
                                                <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{r.display_name}</div>
                                            )}
                                        </td>
                                        <td style={{ textAlign: 'center' }}>{capCell(r.tool_call)}</td>
                                        <td style={{ textAlign: 'center' }}>{capCell(r.reasoning)}</td>
                                        <td style={{ textAlign: 'center' }}>{capCell(r.vision)}</td>
                                        <td
                                            style={{ fontSize: '0.8rem', textAlign: 'right' }}
                                            title={limitTip(r, 'context') || undefined}
                                        >
                                            {r.context_source === 'name' ? <span style={{ color: 'var(--text-secondary)' }}>≈</span> : null}
                                            {fmtNum(r.context)}
                                            {r.limit_conflict && r.dev_context != null && r.context !== r.dev_context && (
                                                <span style={{ color: 'var(--danger)' }}> ⚠</span>
                                            )}
                                        </td>
                                        <td
                                            style={{ fontSize: '0.8rem', textAlign: 'right' }}
                                            title={limitTip(r, 'output') || undefined}
                                        >
                                            {r.output_source === 'name' ? <span style={{ color: 'var(--text-secondary)' }}>≈</span> : null}
                                            {fmtNum(r.output)}
                                            {r.limit_conflict && r.dev_output != null && r.output !== r.dev_output && (
                                                <span style={{ color: 'var(--danger)' }}> ⚠</span>
                                            )}
                                        </td>
                                        <td style={{ fontSize: '0.8rem', textAlign: 'right' }}>{fmtPrice(r.price_input)}</td>
                                        <td style={{ fontSize: '0.8rem', textAlign: 'right' }} title={r.price_cache_read != null ? `${t('models.cache_read')}: ${r.price_cache_read}` : undefined}>
                                            {fmtPrice(r.price_output)}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
                <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                    {t('models.matrix_legend')}
                </small>
            </div>
            )}

            <ConfirmModal
                isOpen={confirmSync}
                title={t('models.sync_limits')}
                message={`${t('models.sync_limits_confirm')}\n\n⚠ ${stats.conflicts}`}
                onConfirm={handleSyncLimits}
                onCancel={() => setConfirmSync(false)}
                confirmText={t('models.sync_limits')}
                cancelText={t('common.cancel')}
            />
        </div>
    );
}
