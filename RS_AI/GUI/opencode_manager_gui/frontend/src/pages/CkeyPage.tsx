/*
[INTEGRITY NOTES]
- Mục đích: Trang tích hợp CKey (ckey.vn) — xem tài khoản, thống kê, lịch sử
  dùng và import model vào cấu hình OpenCode.
- Trách nhiệm: Chọn provider CKey, gán account key (nhập mới hoặc dùng lại),
  hiện dashboard, lịch sử phân trang và danh sách import có tick chọn.
- Tương tác: `bridge/ckey_bridge.ts`, `store/useProviderStore.ts` (nạp lại sau
  import). Không giữ key thật trong state — danh sách key chỉ hiện bản đã che.

Phân biệt hai loại khoá: account key (quản lý, lưu ở ckey.json) và AI key
(ck-..., gọi LLM qua https://api.xah.io/v1). Trang này KHÔNG bao giờ hiện AI
key thật, chỉ hiện prefix/masked.
*/

import { useCallback, useEffect, useState } from 'react';
import { RefreshCw, Download, KeyRound, History, Trash2 } from 'lucide-react';
import type {
    CkeyAccountOption, CkeyDashboard, CkeyImportItem, CkeyProviderView, CkeyUsageView,
} from '../../../bridge/types';
import {
    listCkeyProviders, listCkeyAccounts, setCkeyAccountKey, fetchCkeyDashboard,
    fetchCkeyUsage, listCkeyImportItems, importCkeyModels, deleteCkeyAccountKey,
} from '../../../bridge/ckey_bridge';
import { useProviderStore } from '../store/useProviderStore';
import { useTranslation } from '../utils/i18n';
import { ConfirmModal } from '../components/ConfirmModal';

const USAGE_LIMIT = 20;

export function CkeyPage() {
    const { t } = useTranslation();
    const { fetchProviders } = useProviderStore();

    const [providers, setProviders] = useState<CkeyProviderView[]>([]);
    const [selected, setSelected] = useState('');
    const [accounts, setAccounts] = useState<CkeyAccountOption[]>([]);
    const [newKey, setNewKey] = useState('');
    const [copyFrom, setCopyFrom] = useState('');
    const [dashboard, setDashboard] = useState<CkeyDashboard | null>(null);
    const [usage, setUsage] = useState<CkeyUsageView | null>(null);
    const [usagePage, setUsagePage] = useState(1);
    const [importItems, setImportItems] = useState<CkeyImportItem[]>([]);
    const [checked, setChecked] = useState<Set<string>>(new Set());
    const [isLoading, setIsLoading] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);
    const [confirmRemoveKey, setConfirmRemoveKey] = useState(false);

    const selectedProvider = providers.find(p => p.provider_id === selected) ?? null;

    const reloadProviders = useCallback(async () => {
        const list = await listCkeyProviders();
        setProviders(list);
        // Giữ lựa chọn cũ nếu còn, nếu không chọn provider đầu tiên.
        setSelected(prev => (list.some(p => p.provider_id === prev) ? prev : (list[0]?.provider_id ?? '')));
        setAccounts(await listCkeyAccounts());
    }, []);

    useEffect(() => {
        reloadProviders().catch(err => setNotice(String(err)));
    }, [reloadProviders]);

    const loadDashboard = useCallback(async (providerId: string) => {
        if (!providerId) return;
        setIsLoading(true);
        setNotice(null);
        try {
            setDashboard(await fetchCkeyDashboard(providerId));
            setUsagePage(1);
            setUsage(await fetchCkeyUsage(providerId, 1, USAGE_LIMIT));
            const items = await listCkeyImportItems(providerId);
            setImportItems(items);
            setChecked(new Set(items.filter(i => i.in_config).map(i => i.id)));
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsLoading(false);
        }
    }, []);

    // Tài khoản đã có key → tự tải dashboard khi đổi provider.
    useEffect(() => {
        if (selected && selectedProvider?.has_account_key) {
            loadDashboard(selected).catch(err => setNotice(String(err)));
        } else {
            setDashboard(null);
            setUsage(null);
            setImportItems([]);
        }
    }, [selected, selectedProvider?.has_account_key, loadDashboard]);

    const handleSetKey = async () => {
        if (!selected) return;
        try {
            await setCkeyAccountKey({
                providerId: selected,
                accountKey: newKey.trim() || undefined,
                copyFromProviderId: !newKey.trim() && copyFrom ? copyFrom : undefined,
            });
            setNewKey('');
            await reloadProviders();
            await loadDashboard(selected);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleRemoveKey = async () => {
        setConfirmRemoveKey(false);
        if (!selected) return;
        try {
            await deleteCkeyAccountKey(selected);
            setDashboard(null);
            setUsage(null);
            setImportItems([]);
            await reloadProviders();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    const handleUsagePage = async (page: number) => {
        if (!selected || page < 1) return;
        try {
            setUsage(await fetchCkeyUsage(selected, page, USAGE_LIMIT));
            setUsagePage(page);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
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

    const handleImport = async () => {
        if (!selected) return;
        try {
            const r = await importCkeyModels(selected, [...checked]);
            setNotice(`${t('ckey.import_done')}: +${r.added} / -${r.removed}`);
            await fetchProviders();
            await loadDashboard(selected);
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        }
    };

    return (
        <>
        <div className="animate-fade-in" style={{ maxWidth: '900px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem', flexWrap: 'wrap', gap: '1rem' }}>
                <h1><KeyRound size={22} style={{ verticalAlign: '-3px' }} /> {t('ckey.title')}</h1>
                <button
                    className="btn"
                    style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }}
                    onClick={() => selected && loadDashboard(selected).catch(err => setNotice(String(err)))}
                    disabled={!selected || isLoading}
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

            {providers.length === 0 ? (
                <div className="glass-panel"><p style={{ color: 'var(--text-secondary)' }}>{t('ckey.no_providers')}</p></div>
            ) : (
                <>
                    <div className="glass-panel" style={{ marginBottom: '1rem', display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'end' }}>
                        <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', minWidth: '220px' }}>
                            {t('ckey.lbl_provider')}
                            <select className="input-field" value={selected} onChange={e => setSelected(e.target.value)}>
                                {providers.map(p => (
                                    <option key={p.provider_id} value={p.provider_id}>
                                        {p.provider_id}{p.has_account_key ? '' : ` (${t('ckey.no_account_key')})`}
                                    </option>
                                ))}
                            </select>
                        </label>

                        {selectedProvider && selectedProvider.has_account_key && (
                            <button
                                className="btn"
                                style={{ color: 'var(--danger)', border: '1px solid var(--danger)' }}
                                onClick={() => setConfirmRemoveKey(true)}
                                title={t('ckey.remove_key')}
                            >
                                <Trash2 size={16} /> {t('ckey.remove_key')}
                            </button>
                        )}

                        {selectedProvider && !selectedProvider.has_account_key && (
                            <>
                                <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', flex: 1, minWidth: '220px' }}>
                                    {t('ckey.lbl_new_key')}
                                    <input
                                        type="password"
                                        className="input-field"
                                        value={newKey}
                                        onChange={e => setNewKey(e.target.value)}
                                        placeholder={t('ckey.new_key_placeholder')}
                                    />
                                </label>
                                {accounts.length > 0 && !newKey.trim() && (
                                    <label style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', fontSize: '0.9rem', minWidth: '200px' }}>
                                        {t('ckey.lbl_copy_from')}
                                        <select className="input-field" value={copyFrom} onChange={e => setCopyFrom(e.target.value)}>
                                            <option value="">—</option>
                                            {accounts.filter(a => a.provider_id !== selected).map(a => (
                                                <option key={a.provider_id} value={a.provider_id}>
                                                    {a.provider_id} ({a.key_masked})
                                                </option>
                                            ))}
                                        </select>
                                    </label>
                                )}
                                <button className="btn btn-primary" onClick={handleSetKey}>
                                    {t('common.save')}
                                </button>
                            </>
                        )}
                    </div>

                    {isLoading && <p>{t('common.loading')}</p>}

                    {dashboard && (
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
                                    <div>{t('ckey.requests')}: {dashboard.stats.requests} ({t('ckey.success')}: {dashboard.stats.success_requests})</div>
                                    <div>{t('ckey.tokens')}: {dashboard.stats.total_tokens} (in {dashboard.stats.prompt_tokens} / out {dashboard.stats.completion_tokens})</div>
                                    <div>{t('ckey.charged')}: {dashboard.stats.charged_vnd_text}</div>
                                </div>
                            )}

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

                            <div className="glass-panel" style={{ marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
                                    <h3>Models ({checked.size}/{importItems.length})</h3>
                                    <button className="btn btn-primary" onClick={handleImport}>
                                        <Download size={16} /> {t('ckey.apply_import')}
                                    </button>
                                </div>
                                <div style={{ maxHeight: '320px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                                    {importItems.map(item => (
                                        <label key={item.id} style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', fontSize: '0.85rem', padding: '0.3rem 0.5rem', background: 'rgba(255,255,255,0.03)', borderRadius: '4px', cursor: 'pointer' }}>
                                            <input type="checkbox" checked={checked.has(item.id)} onChange={() => toggleImport(item.id)} />
                                            <span style={{ fontFamily: 'monospace' }}>{item.id}</span>
                                            {item.stale && <span className="badge badge-inactive" style={{ fontSize: '0.65rem' }}>{t('models.stale_badge')}</span>}
                                            {!item.stale && item.display_name !== item.id && (
                                                <span style={{ color: 'var(--text-secondary)' }}>{item.display_name}</span>
                                            )}
                                        </label>
                                    ))}
                                </div>
                            </div>

                            {usage && (
                                <div className="glass-panel">
                                    <h3 style={{ marginBottom: '0.5rem' }}><History size={16} style={{ verticalAlign: '-2px' }} /> {t('ckey.usage_title')}</h3>
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
                                        <button className="btn" disabled={usagePage <= 1} onClick={() => handleUsagePage(usagePage - 1)}>←</button>
                                        <span>{usagePage} / {usage.total_pages}</span>
                                        <button className="btn" disabled={usagePage >= usage.total_pages} onClick={() => handleUsagePage(usagePage + 1)}>→</button>
                                    </div>
                                </div>
                            )}
                        </>
                    )}
                </>
            )}
        </div>

        <ConfirmModal
            isOpen={confirmRemoveKey}
            title={t('ckey.remove_key_confirm_title')}
            message={`${t('ckey.remove_key_confirm_msg')}\n\n${selected}`}
            onConfirm={handleRemoveKey}
            onCancel={() => setConfirmRemoveKey(false)}
            confirmText={t('common.delete')}
            cancelText={t('common.cancel')}
            isDanger
        />
        </>
    );
}
