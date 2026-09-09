/*
[INTEGRITY NOTES]
- Mục đích: Trang CKey (ckey.vn) — quản lý TÀI KHOẢN (profile) trực tiếp,
  dashboard/lịch sử/nạp tiền theo tài khoản đang chọn, và import model vào
  provider với AI key của tài khoản đó.
- Trách nhiệm:
  + Account section: THẢNG TỚI ckey.vn (không có khái niệm provider — account
    key là của tài khoản, không gắn provider nào).
  + Import section: chọn provider ĐÍCH (nơi ghi model vào opencode.json).
  + Bảng giá model: 4 cột In / Out / Cache / Mỗi-request + sắp xếp theo cột.
- Tương tác: `bridge/ckey_bridge.ts`, `store/useProviderStore.ts` (nạp lại sau
  import). Không giữ key thật trong state — danh sách key chỉ hiện bản đã che.

Cơ chế tự update: interval 60s gọi refresh với force = false — backend trả từ
cache TTL (models 5 phút, stats/keys 2 phút, profile 10 phút) nên chu kỳ gọi
mạng thật dài, tránh bị ckey.vn rate-limit/ban. Nút Refresh dùng force = true.
*/

import { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { RefreshCw, Download, KeyRound, History, Trash2, Wallet, Plus, Pencil } from 'lucide-react';
import type {
    CkeyDashboard, CkeyDepositView, CkeyImportItem, CkeyProfileView, CkeyUsageView,
} from '../../../bridge/types';
import {
    listCkeyProfiles, saveCkeyProfile, deleteCkeyProfile, setActiveCkeyProfile,
    fetchCkeyDashboard, fetchCkeyUsage, fetchCkeyDeposit,
    listCkeyImportItems, importCkeyModels,
} from '../../../bridge/ckey_bridge';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';
import { ConfirmModal } from '../components/ConfirmModal';
import { openExternalUrl } from '../../../bridge/settings_bridge';

const USAGE_LIMIT = 20;
const DEPOSIT_LIMIT = 10;
/** Chu kỳ tự refresh (ms). force=false → backend dùng cache TTL. */
const AUTO_REFRESH_MS = 60_000;

type StatsPeriod = 0 | 7 | 30;
type SortKey = 'provider' | 'model' | 'in' | 'out' | 'cache' | 'perreq' | 'ctx';

export function CkeyPage() {
    const { t } = useTranslation();
    const fetchProviders = useProviderStore(state => state.fetchProviders);

    // ===== Tài khoản (profile) =====
    const [profiles, setProfiles] = useState<CkeyProfileView[]>([]);
    const [activeId, setActiveId] = useState('');
    const [showProfileForm, setShowProfileForm] = useState(false);
    const [editingProfile, setEditingProfile] = useState<CkeyProfileView | null>(null);
    const [formName, setFormName] = useState('');
    const [formKey, setFormKey] = useState('');
    const [toDeleteProfile, setToDeleteProfile] = useState<CkeyProfileView | null>(null);

    // ===== Dữ liệu tài khoản =====
    const [dashboard, setDashboard] = useState<CkeyDashboard | null>(null);
    const [statsPeriod, setStatsPeriod] = useState<StatsPeriod>(0);
    const [usage, setUsage] = useState<CkeyUsageView | null>(null);
    const [usagePage, setUsagePage] = useState(1);
    const [usageModel, setUsageModel] = useState('');
    const [deposit, setDeposit] = useState<CkeyDepositView | null>(null);
    const [depositAmount, setDepositAmount] = useState('50000');
    const [depositPage, setDepositPage] = useState(1);

    // ===== Import =====
    // Catalogue toàn cục; đích import do backend suy từ binding của tài khoản
    // đang xem — không có dropdown chọn provider nữa (dư thừa vì bảng giá
    // giống nhau mọi tài khoản, và đích thật sự chỉ cần biết lúc ghi config).
    const [importTarget, setImportTarget] = useState('');
    const [importItems, setImportItems] = useState<CkeyImportItem[]>([]);
    const [checked, setChecked] = useState<Set<string>>(new Set());
    const [sortKey, setSortKey] = useState<SortKey>('model');
    const [sortAsc, setSortAsc] = useState(true);

    const [isLoading, setIsLoading] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);
    const accountRequestRef = useRef(0);
    const foregroundRequestRef = useRef(0);
    const foregroundPendingRef = useRef(false);
    const usageRequestRef = useRef(0);
    const depositRequestRef = useRef(0);
    const importRequestRef = useRef(0);

    const reloadProfiles = useCallback(async () => {
        const list = await listCkeyProfiles();
        setProfiles(list);
        // Giữ active cũ nếu còn; không thì profile active từ backend (hoặc đầu).
        setActiveId(prev => (list.some(p => p.id === prev) ? prev : (list.find(p => p.is_active)?.id ?? list[0]?.id ?? '')));
    }, []);

    useEffect(() => {
        reloadProfiles().catch(err => setNotice(String(err)));
    }, [reloadProfiles]);

    // ===== Tải dữ liệu tài khoản (dashboard + usage) =====
    // KHÔNG đụng catalogue import ở đây: bảng giá là TOÀN CỤC (giống nhau mọi
    // tài khoản) và có effect riêng theo [importTarget] — gộp vào đây sẽ khiến
    // chọn provider đích nạp lại cả dashboard (force) và fetch items hai lần.
    const loadAccountData = useCallback(async (
        profileId: string,
        sinceDays: number,
        force: boolean,
        foreground = false,
    ) => {
        if (!profileId) return;
        if (!foreground && foregroundPendingRef.current) return;
        const request = ++accountRequestRef.current;
        const usageRequest = ++usageRequestRef.current;
        const foregroundRequest = foreground ? ++foregroundRequestRef.current : 0;
        if (foreground) {
            foregroundPendingRef.current = true;
            setIsLoading(true);
        }
        if (force) setNotice(null);
        try {
            const [nextDashboard, nextUsage] = await Promise.all([
                fetchCkeyDashboard(profileId, sinceDays || null, force),
                fetchCkeyUsage(profileId, 1, USAGE_LIMIT, null, force),
            ]);
            if (request !== accountRequestRef.current) return;
            startTransition(() => {
                setDashboard(nextDashboard);
                if (usageRequest === usageRequestRef.current) {
                    setUsagePage(1);
                    setUsage(nextUsage);
                }
            });
        } catch (err) {
            if (request !== accountRequestRef.current) return;
            // Lỗi tải (vd sai key) → xoá dashboard cũ để UI không hiện dữ liệu
            // của tài khoản trước như thể còn hợp lệ.
            setDashboard(null);
            setUsage(null);
            if (force) setNotice(err instanceof Error ? err.message : String(err));
            else console.error('Tự refresh CKey lỗi:', err);
        } finally {
            if (foreground && foregroundRequest === foregroundRequestRef.current) {
                foregroundPendingRef.current = false;
                setIsLoading(false);
            }
        }
    }, []);

    useEffect(() => {
        if (activeId) {
            // Vào trang/đổi kỳ dùng cache trước; chỉ nút Refresh mới bỏ TTL.
            loadAccountData(activeId, statsPeriod, false, true).catch(err => setNotice(String(err)));
        } else {
            accountRequestRef.current += 1;
            usageRequestRef.current += 1;
            depositRequestRef.current += 1;
            foregroundRequestRef.current += 1;
            foregroundPendingRef.current = false;
            setIsLoading(false);
            setDashboard(null);
            setUsage(null);
            setImportItems([]);
            setDeposit(null);
        }
    }, [activeId, statsPeriod, loadAccountData]);

    // ===== Tự refresh nền (qua cache — tránh gọi mạng dồn dập) =====
    useEffect(() => {
        if (!activeId) return;
        const timer = setInterval(() => {
            loadAccountData(activeId, statsPeriod, false);
        }, AUTO_REFRESH_MS);
        return () => clearInterval(timer);
    }, [activeId, statsPeriod, loadAccountData]);

    // ===== Quản lý profile =====
    const openNewProfileForm = () => {
        setEditingProfile(null);
        setFormName('');
        setFormKey('');
        setShowProfileForm(true);
    };

    const openEditProfileForm = (p: CkeyProfileView) => {
        setEditingProfile(p);
        setFormName(p.name);
        setFormKey('');
        setShowProfileForm(true);
    };

    const handleSaveProfile = async () => {
        if (!formKey.trim()) {
            setNotice(t('ckey.profile_key_required'));
            return;
        }
        try {
            const id = await saveCkeyProfile({
                profileId: editingProfile?.id ?? null,
                name: formName.trim(),
                key: formKey.trim(),
            });
            setShowProfileForm(false);
            setFormKey('');
            await reloadProfiles();
            // Profile mới / sửa xong → xem luôn tài khoản đó.
            await setActiveCkeyProfile(id);
            setActiveId(id);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleSwitchProfile = async (id: string) => {
        if (id === activeId) return;
        try {
            await setActiveCkeyProfile(id);
            setActiveId(id);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleDeleteProfile = async () => {
        const p = toDeleteProfile;
        setToDeleteProfile(null);
        if (!p) return;
        try {
            const newActive = await deleteCkeyProfile(p.id);
            await reloadProfiles();
            if (newActive) setActiveId(newActive);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    // ===== Usage / deposit =====
    const loadUsage = useCallback(async (page: number, model: string, force = false) => {
        if (!activeId) return;
        const request = ++usageRequestRef.current;
        try {
            const nextUsage = await fetchCkeyUsage(activeId, page, USAGE_LIMIT, model.trim() || null, force);
            if (request !== usageRequestRef.current) return;
            setUsage(nextUsage);
            setUsagePage(page);
        } catch (err) {
            if (request === usageRequestRef.current) setNotice(err instanceof Error ? err.message : String(err));
        }
    }, [activeId]);

    const loadDeposit = useCallback(async (page: number, force = false) => {
        if (!activeId) return;
        const request = ++depositRequestRef.current;
        const amount = parseInt(depositAmount.replace(/\D/g, ''), 10);
        // Backend ép tối thiểu 1.000₫ — báo ở đây để không tưởng đang nạp số nhỏ hơn.
        if (!Number.isFinite(amount) || amount < 1_000) {
            setNotice(t('ckey.deposit_bad_amount'));
            return;
        }
        try {
            const nextDeposit = await fetchCkeyDeposit(activeId, amount, page, DEPOSIT_LIMIT, force);
            if (request !== depositRequestRef.current) return;
            setDeposit(nextDeposit);
            setDepositPage(page);
        } catch (err) {
            if (request === depositRequestRef.current) setNotice(err instanceof Error ? err.message : String(err));
        }
    }, [activeId, depositAmount, t]);

    // ===== Import =====
    // Catalogue model + giá là TOÀN CỤC (giống nhau mọi tài khoản), nhưng đích
    // import (đối chiếu in_config) phụ thuộc BINDING của tài khoản đang xem —
    // nên đổi tài khoản là nạp lại danh sách (catalogue lấy từ cache, rẻ).
    useEffect(() => {
        if (activeId) {
            const request = ++importRequestRef.current;
            listCkeyImportItems()
                .then(list => {
                    if (request !== importRequestRef.current) return;
                    startTransition(() => {
                        setImportTarget(list.target_provider);
                        setImportItems(list.items);
                        setChecked(new Set(list.items.filter(i => i.in_config && !i.stale).map(i => i.id)));
                    });
                })
                .catch(err => {
                    if (request === importRequestRef.current) setNotice(String(err));
                });
        } else {
            importRequestRef.current += 1;
            setImportTarget('');
            setImportItems([]);
            setChecked(new Set());
        }
    }, [activeId]);
    const sortedItems = useMemo(() => {
        const val = (i: CkeyImportItem): number | string => {
            switch (sortKey) {
                case 'provider': return i.provider.toLowerCase();
                case 'in': return i.input_price;
                case 'out': return i.output_price;
                case 'cache': return i.cache_read_price;
                case 'perreq': return i.price_per_request || i.min_charge_per_request;
                case 'ctx': return i.context_limit;
                default: return i.model.toLowerCase();
            }
        };
        const arr = [...importItems];
        arr.sort((a, b) => {
            const va = val(a);
            const vb = val(b);
            const cmp = typeof va === 'string' ? va.localeCompare(vb as string) : (va as number) - (vb as number);
            return sortAsc ? cmp : -cmp;
        });
        return arr;
    }, [importItems, sortKey, sortAsc]);

    const toggleSort = (key: SortKey) => {
        if (key === sortKey) setSortAsc(a => !a);
        else {
            setSortKey(key);
            setSortAsc(true);
        }
    };

    const toggleImport = (id: string) => {
        setChecked(prev => {
            const next = new Set(prev);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            return next;
        });
    };

    const toggleAllImport = () => {
        setChecked(prev => (prev.size === sortedItems.length ? new Set() : new Set(sortedItems.map(i => i.id))));
    };

    const handleImport = async () => {
        if (!activeId) return;
        // `selected` là danh sách CUỐI CÙNG — backend sẽ XOÁ mọi model không có
        // trong đó. Chọn 0 model mà bấm áp dụng = xoá sạch provider đích, nên
        // phải hỏi lại; danh sách chưa tải được thì khoá nút luôn.
        if (checked.size === 0 && !window.confirm(t('ckey.import_none_selected_confirm'))) {
            return;
        }
        const request = ++importRequestRef.current;
        try {
            const r = await importCkeyModels(activeId, [...checked]);
            setNotice(`${t('ckey.import_done')}: +${r.added} / -${r.removed}`);
            await fetchProviders();
            const list = await listCkeyImportItems();
            if (request !== importRequestRef.current) return;
            startTransition(() => {
                setImportTarget(list.target_provider);
                setImportItems(list.items);
                setChecked(new Set(list.items.filter(i => i.in_config && !i.stale).map(i => i.id)));
            });
        } catch (err) {
            if (request === importRequestRef.current) setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const openQr = async (url: string) => {
        // Webview của Tauri không điều hướng được trang ngoài — mở bằng trình
        // duyệt hệ thống qua command backend (chỉ chấp nhận https).
        if (!url) return;
        try {
            await openExternalUrl(url);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const fmtNum = (n: number) => n.toLocaleString('vi-VN');
    const fmtPrice = (n: number) => (n > 0 ? fmtNum(n) : '—');

    const sortArrow = (key: SortKey) => (sortKey === key ? (sortAsc ? ' ↑' : ' ↓') : '');

    return (
        <>
        <div className="animate-fade-in" style={{ maxWidth: '1080px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', flexWrap: 'wrap', gap: '1rem' }}>
                <h1><KeyRound size={22} style={{ verticalAlign: '-3px' }} /> {t('ckey.title')}</h1>
                <button
                    className="btn"
                    style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }}
                    onClick={() => activeId && loadAccountData(activeId, statsPeriod, true, true)}
                    disabled={!activeId || isLoading}
                    title={t('ckey.force_refresh_hint')}
                >
                    <RefreshCw size={18} /> {t('common.refresh')}
                </button>
            </div>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>{t('ckey.desc')}</p>

            {notice && (
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(99,102,241,0.15)', borderLeft: '4px solid var(--primary)', borderRadius: '4px', fontSize: '0.9rem' }}>
                    <span>{notice}</span>
                    <button className="btn" style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }} onClick={() => setNotice(null)}>✕</button>
                </div>
            )}

            {/* ===== TÀI KHOẢN CKEY (profile) — không có khái niệm provider ===== */}
            <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                <h3 style={{ marginTop: 0, marginBottom: '0.5rem' }}>{t('ckey.profiles_title')}</h3>
                {profiles.length === 0 && !showProfileForm ? (
                    <p style={{ color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>{t('ckey.no_profiles')}</p>
                ) : (
                    <div style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap', marginBottom: showProfileForm ? '1rem' : 0 }}>
                        {profiles.map(p => (
                            <div
                                key={p.id}
                                style={{
                                    display: 'flex', gap: '0.5rem', alignItems: 'center',
                                    padding: '0.35rem 0.75rem', borderRadius: '4px', cursor: 'pointer',
                                    border: `1px solid ${p.id === activeId ? 'var(--primary)' : 'var(--border)'}`,
                                    background: p.id === activeId ? 'rgba(99,102,241,0.15)' : 'transparent',
                                }}
                                onClick={() => handleSwitchProfile(p.id)}
                                title={p.key_masked}
                            >
                                <span style={{ fontSize: '0.9rem', fontWeight: p.id === activeId ? 600 : 400 }}>{p.name}</span>
                                <span style={{ fontFamily: 'monospace', fontSize: '0.75rem', color: 'var(--text-secondary)' }}>{p.key_masked}</span>
                                {p.id === activeId && <span className="badge badge-active" style={{ fontSize: '0.6rem' }}>{t('ckey.profile_active_badge')}</span>}
                                <button
                                    className="btn"
                                    style={{ padding: '0.1rem 0.3rem', background: 'transparent' }}
                                    onClick={e => { e.stopPropagation(); openEditProfileForm(p); }}
                                    title={t('ckey.edit_profile')}
                                >
                                    <Pencil size={13} />
                                </button>
                                <button
                                    className="btn"
                                    style={{ padding: '0.1rem 0.3rem', background: 'transparent', color: 'var(--danger)' }}
                                    onClick={e => { e.stopPropagation(); setToDeleteProfile(p); }}
                                    title={t('ckey.delete_profile')}
                                >
                                    <Trash2 size={13} />
                                </button>
                            </div>
                        ))}
                    </div>
                )}

                {showProfileForm ? (
                    <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'end' }}>
                        <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', minWidth: '180px' }}>
                            {t('ckey.lbl_profile_name')}
                            <input
                                type="text"
                                className="input-field"
                                value={formName}
                                onChange={e => setFormName(e.target.value)}
                                placeholder={t('ckey.profile_name_placeholder')}
                            />
                        </label>
                        <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', flex: 1, minWidth: '260px' }}>
                            {t('ckey.lbl_profile_key')}
                            <input
                                type="password"
                                className="input-field"
                                value={formKey}
                                onChange={e => setFormKey(e.target.value)}
                                placeholder={editingProfile ? t('ckey.change_key_placeholder') : t('ckey.new_key_placeholder')}
                            />
                        </label>
                        <button className="btn btn-primary" onClick={handleSaveProfile}>
                            {t('common.save')}
                        </button>
                        <button className="btn" onClick={() => { setShowProfileForm(false); setFormKey(''); }}>
                            {t('common.cancel')}
                        </button>
                    </div>
                ) : (
                    <button className="btn" style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }} onClick={openNewProfileForm}>
                        <Plus size={16} /> {t('ckey.add_profile')}
                    </button>
                )}
                <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.5rem' }}>
                    {t('ckey.key_hint')}
                </small>
            </div>

            {isLoading && <p>{t('common.loading')}</p>}

            {activeId && dashboard && (
                <>
                    {dashboard.errors.length > 0 && (
                        <div style={{ padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(239,68,68,0.12)', borderLeft: '4px solid var(--danger)', borderRadius: '4px', fontSize: '0.9rem' }}>
                            {dashboard.errors.map((e, i) => <div key={i}>{e}</div>)}
                        </div>
                    )}

                    {dashboard.profile && (
                        <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                            <h3 style={{ marginBottom: '0.5rem' }}>{dashboard.profile.username}</h3>
                            <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                                {dashboard.profile.email && <div>{dashboard.profile.email}</div>}
                                <div>{t('ckey.balance')}: {dashboard.profile.balance}</div>
                            </div>
                        </div>
                    )}

                    {dashboard.stats && (
                        <div className="glass-panel" style={{ marginBottom: '1rem', fontSize: '0.9rem' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem', marginBottom: '0.5rem' }}>
                                <strong>{t('ckey.stats_title')}</strong>
                                <select
                                    className="input-field"
                                    aria-label={t('ckey.stats_title')}
                                    value={String(statsPeriod)}
                                    onChange={event => setStatsPeriod(Number(event.target.value) as StatsPeriod)}
                                    style={{ width: 'auto' }}
                                >
                                    <option value="0">{t('ckey.period_all')}</option>
                                    <option value="7">{t('ckey.period_7d')}</option>
                                    <option value="30">{t('ckey.period_30d')}</option>
                                </select>
                            </div>
                            <div>{t('ckey.requests')}: {dashboard.stats.requests} ({t('ckey.success')}: {dashboard.stats.success_requests})</div>
                            <div>{t('ckey.tokens')}: {dashboard.stats.total_tokens} (in {dashboard.stats.prompt_tokens} / out {dashboard.stats.completion_tokens})</div>
                            <div>{t('ckey.charged')}: {dashboard.stats.charged_vnd_text}</div>
                            <small style={{ color: 'var(--text-secondary)' }}>{t('ckey.auto_refresh_hint')}</small>
                        </div>
                    )}

                    {/* ===== NẠP TIỀN ===== */}
                    <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                        <h3 style={{ marginTop: 0, marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                            <Wallet size={16} /> {t('ckey.deposit_title')}
                        </h3>
                        <div style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap', alignItems: 'end', marginBottom: '0.75rem' }}>
                            <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', minWidth: '160px' }}>
                                {t('ckey.deposit_amount')}
                                <input
                                    type="text"
                                    className="input-field"
                                    value={depositAmount}
                                    onChange={e => setDepositAmount(e.target.value)}
                                    placeholder="50000"
                                    inputMode="numeric"
                                />
                            </label>
                            <button className="btn btn-primary" onClick={() => loadDeposit(1, true)}>
                                <Wallet size={16} /> {t('ckey.deposit_get_info')}
                            </button>
                        </div>

                        {deposit && (
                            <>
                                {deposit.errors.length > 0 && (
                                    <div style={{ padding: '0.6rem 0.75rem', marginBottom: '0.75rem', background: 'rgba(239,68,68,0.12)', borderLeft: '4px solid var(--danger)', borderRadius: '4px', fontSize: '0.85rem' }}>
                                        {deposit.errors.map((e, i) => <div key={i}>{e}</div>)}
                                    </div>
                                )}
                                {deposit.info && (
                                    <div style={{ marginBottom: '0.75rem', fontSize: '0.9rem' }}>
                                        <div>
                                            <strong>{t('ckey.deposit_content')}:</strong>{' '}
                                            <code style={{ fontFamily: 'monospace', color: 'var(--primary)' }}>{deposit.info.transfer_content}</code>
                                        </div>
                                        <div style={{ color: 'var(--text-secondary)' }}>
                                            {t('ckey.deposit_amount_col')}: {fmtNum(deposit.info.amount_vnd)}₫
                                            {deposit.info.expires_at && ` · ${t('ckey.deposit_expires')}: ${deposit.info.expires_at}`}
                                        </div>
                                        {deposit.info.qr_url && (
                                            <button className="btn" style={{ marginTop: '0.4rem', padding: '0.25rem 0.6rem', fontSize: '0.8rem' }} onClick={() => openQr(deposit.info!.qr_url)}>
                                                {t('ckey.deposit_open_qr')}
                                            </button>
                                        )}
                                        {deposit.info.banks.length > 0 && (
                                            <div className="table-container" style={{ marginTop: '0.5rem' }}>
                                                <table>
                                                    <thead><tr><th>{t('ckey.deposit_bank')}</th><th>{t('ckey.deposit_owner')}</th><th>{t('ckey.deposit_account')}</th><th>QR</th></tr></thead>
                                                    <tbody>
                                                        {deposit.info.banks.map((b, i) => (
                                                            <tr key={i}>
                                                                <td style={{ fontSize: '0.85rem' }}>{b.bank_name}</td>
                                                                <td style={{ fontSize: '0.85rem' }}>{b.account_owner}</td>
                                                                <td style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>{b.account_number}</td>
                                                                <td>
                                                                    {b.qr_url && (
                                                                        <button className="btn" style={{ padding: '0.15rem 0.4rem', fontSize: '0.75rem' }} onClick={() => openQr(b.qr_url)}>QR</button>
                                                                    )}
                                                                </td>
                                                            </tr>
                                                        ))}
                                                    </tbody>
                                                </table>
                                            </div>
                                        )}
                                    </div>
                                )}
                                {deposit.history.length > 0 && (
                                    <div>
                                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <strong style={{ fontSize: '0.9rem' }}>{t('ckey.deposit_history_title')}</strong>
                                            <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', fontSize: '0.85rem' }}>
                                                <button className="btn" disabled={depositPage <= 1} onClick={() => loadDeposit(depositPage - 1)}>←</button>
                                                <span>{depositPage} / {deposit.history_total_pages}</span>
                                                <button className="btn" disabled={depositPage >= deposit.history_total_pages} onClick={() => loadDeposit(depositPage + 1)}>→</button>
                                            </div>
                                        </div>
                                        <div className="table-container">
                                            <table>
                                                <thead><tr><th>#</th><th>{t('ckey.deposit_amount_col')}</th><th>{t('ckey.col_time')}</th></tr></thead>
                                                <tbody>
                                                    {deposit.history.map(h => (
                                                        <tr key={h.id}>
                                                            <td>{h.id}</td>
                                                            <td>{h.amount_text}</td>
                                                            <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>{h.time_text}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                )}
                            </>
                        )}
                    </div>

                    <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                        <h3 style={{ marginBottom: '0.5rem' }}>AI keys ({dashboard.keys.length})</h3>
                        <div className="table-container">
                            <table>
                                <thead><tr><th>ID</th><th>{t('ckey.col_name')}</th><th>{t('ckey.col_prefix')}</th><th>{t('ckey.col_key')}</th><th>{t('ckey.col_active')}</th></tr></thead>
                                <tbody>
                                    {dashboard.keys.map(k => (
                                        <tr key={k.id}>
                                            <td>{k.id}</td>
                                            <td>{k.key_name}</td>
                                            <td style={{ fontFamily: 'monospace' }}>{k.key_prefix}</td>
                                            <td style={{ fontFamily: 'monospace', fontSize: '0.8rem' }}>{k.key_masked}</td>
                                            <td>{k.is_active ? '✓' : '—'}</td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    </div>

                    {usage && (
                        <div className="glass-panel">
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', flexWrap: 'wrap', gap: '0.5rem' }}>
                                <h3 style={{ marginBottom: 0 }}><History size={16} style={{ verticalAlign: '-2px' }} /> {t('ckey.usage_title')}</h3>
                                <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                                    <input
                                        type="text"
                                        className="input-field"
                                        style={{ padding: '0.25rem 0.5rem', fontSize: '0.8rem', width: '180px' }}
                                        value={usageModel}
                                        onChange={e => setUsageModel(e.target.value)}
                                        onKeyDown={e => { if (e.key === 'Enter') loadUsage(1, usageModel, true); }}
                                        placeholder={t('ckey.usage_filter_model')}
                                    />
                                    <button className="btn" style={{ padding: '0.25rem 0.6rem', fontSize: '0.8rem' }} onClick={() => loadUsage(1, usageModel, true)}>
                                        {t('ckey.apply_filter')}
                                    </button>
                                </div>
                            </div>
                            <div className="table-container">
                                <table>
                                    <thead><tr><th>{t('ckey.col_model')}</th><th>HTTP</th><th>Tokens</th><th>VND</th><th>{t('ckey.col_time')}</th></tr></thead>
                                    <tbody>
                                        {usage.items.map(item => (
                                            <tr key={item.request_id}>
                                                <td style={{ fontSize: '0.8rem' }}>{item.model_name}</td>
                                                <td>{item.http_status}</td>
                                                <td>{item.total_tokens}</td>
                                                <td>{item.charged_vnd}</td>
                                                <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>{item.created_at_text}</td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                            <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', marginTop: '0.5rem', fontSize: '0.85rem' }}>
                                <button className="btn" disabled={usagePage <= 1} onClick={() => loadUsage(usagePage - 1, usageModel, true)}>←</button>
                                <span>{usagePage} / {usage.total_pages}</span>
                                <button className="btn" disabled={usagePage >= usage.total_pages} onClick={() => loadUsage(usagePage + 1, usageModel, true)}>→</button>
                            </div>
                        </div>
                    )}
                </>
            )}

            {/* ===== IMPORT: catalogue TOÀN CỤC — bảng giá giống nhau mọi tài
                 khoản nên KHÔNG cần chọn provider để xem. Provider ĐÍCH do
                 backend suy từ binding của tài khoản đang xem (import cũng là
                 lúc duy nhất gắn binding). ===== */}
            {activeId && (
                <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.75rem', marginBottom: '0.5rem' }}>
                        <h3 style={{ marginBottom: 0 }}>
                            {t('ckey.import_title')} ({checked.size}/{sortedItems.length})
                            {importTarget && (
                                <span style={{ marginLeft: '0.5rem', fontSize: '0.75rem', fontWeight: 400, color: 'var(--text-secondary)' }}>
                                    → <code style={{ fontFamily: 'monospace' }}>{importTarget}</code>
                                </span>
                            )}
                        </h3>
                        <button
                            className="btn btn-primary"
                            onClick={handleImport}
                            disabled={sortedItems.length === 0}
                            title={checked.size === 0 ? t('ckey.import_none_selected_confirm') : undefined}
                        >
                            <Download size={16} /> {t('ckey.apply_import')}
                        </button>
                    </div>

                    {sortedItems.length > 0 && (
                        <div className="table-container">
                            <table>
                                <thead>
                                    <tr>
                                        <th style={{ width: '34px' }}>
                                            <input
                                                type="checkbox"
                                                checked={checked.size === sortedItems.length}
                                                onChange={toggleAllImport}
                                                style={{ width: '13px', height: '13px', cursor: 'pointer' }}
                                                title={t('common.select_all')}
                                            />
                                        </th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('provider')}>{t('ckey.col_provider')}{sortArrow('provider')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('model')}>{t('ckey.col_model')}{sortArrow('model')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('in')}>{t('ckey.col_in')}{sortArrow('in')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('out')}>{t('ckey.col_out')}{sortArrow('out')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('cache')}>{t('ckey.col_cache')}{sortArrow('cache')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('perreq')}>{t('ckey.col_perreq')}{sortArrow('perreq')}</th>
                                        <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('ctx')}>{t('ckey.col_ctx')}{sortArrow('ctx')}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {sortedItems.map(item => (
                                        <tr
                                            key={item.id}
                                            style={{ cursor: 'pointer', background: checked.has(item.id) ? 'rgba(99,102,241,0.08)' : undefined }}
                                            onClick={() => toggleImport(item.id)}
                                        >
                                            <td>
                                                <input
                                                    type="checkbox"
                                                    checked={checked.has(item.id)}
                                                    onChange={() => toggleImport(item.id)}
                                                    onClick={e => e.stopPropagation()}
                                                    style={{ width: '13px', height: '13px', cursor: 'pointer' }}
                                                />
                                            </td>
                                            <td style={{ fontSize: '0.82rem', color: 'var(--text-secondary)' }}>
                                                {item.provider || '—'}
                                            </td>
                                            <td>
                                                <div style={{ fontFamily: 'monospace', fontSize: '0.82rem' }}>{item.model}</div>
                                                {item.stale ? (
                                                    <span className="badge badge-inactive" style={{ fontSize: '0.6rem', color: 'var(--danger)', borderColor: 'var(--danger)' }}>{t('models.stale_badge')}</span>
                                                ) : item.display_name !== item.id && (
                                                    <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{item.display_name}</span>
                                                )}
                                            </td>
                                            <td style={{ fontSize: '0.82rem', textAlign: 'right' }}>{fmtPrice(item.input_price)}</td>
                                            <td style={{ fontSize: '0.82rem', textAlign: 'right' }}>{fmtPrice(item.output_price)}</td>
                                            <td style={{ fontSize: '0.82rem', textAlign: 'right' }}>
                                                {item.cache_enabled
                                                    ? `${fmtPrice(item.cache_read_price)}/${fmtPrice(item.cache_write_price)}`
                                                    : '—'}
                                            </td>
                                            <td style={{ fontSize: '0.82rem', textAlign: 'right' }}>
                                                {item.price_per_request > 0
                                                    ? fmtNum(item.price_per_request)
                                                    : (item.min_charge_per_request > 0 ? `≥${fmtNum(item.min_charge_per_request)}` : '—')}
                                            </td>
                                            <td style={{ fontSize: '0.78rem', textAlign: 'right', color: 'var(--text-secondary)' }}>
                                                {item.context_limit > 0 || item.output_limit > 0
                                                    ? `${fmtNum(item.context_limit)}/${fmtNum(item.output_limit)}`
                                                    : '—'}
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </div>
                    )}
                    <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '0.4rem' }}>
                        {t('ckey.price_unit_hint')}
                    </small>
                </div>
            )}
        </div>

        <ConfirmModal
            isOpen={toDeleteProfile !== null}
            title={t('ckey.delete_profile_confirm_title')}
            message={`${t('ckey.delete_profile_confirm_msg')}\n\n${toDeleteProfile?.name ?? ''}`}
            onConfirm={handleDeleteProfile}
            onCancel={() => setToDeleteProfile(null)}
            confirmText={t('common.delete')}
            cancelText={t('common.cancel')}
            isDanger
        />
        </>
    );
}
