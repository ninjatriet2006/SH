import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import {
    addAccount,
    disableAccount,
    enableAccount,
    listAccounts,
    removeAccount,
    updateAccountRouting,
} from '../../../bridge/accounts_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccountInfo } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

export function AccountsPage() {
    const { t } = useTranslation();
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Edit modal state
    const [editingAccount, setEditingAccount] = useState<AccountInfo | null>(null);
    const [editProxyUrl, setEditProxyUrl] = useState('');
    const [editUserAgent, setEditUserAgent] = useState('');
    const [editHeadersJson, setEditHeadersJson] = useState('');

    const refresh = async () => {
        try { setError(null); setAccounts(await listAccounts()); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setLoading(false); }
    };
    useEffect(() => { void refresh(); }, []);

    const chooseAuthFile = async () => {
        const path = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (typeof path !== 'string') return;
        try { setBusy(true); setError(null); await addAccount(path); await refresh(); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const action = async (fn: () => Promise<void>) => {
        try { setBusy(true); setError(null); await fn(); await refresh(); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const openEditModal = (account: AccountInfo) => {
        setEditingAccount(account);
        setEditProxyUrl(account.proxy_url || '');
        setEditUserAgent(account.fingerprint_profile?.user_agent || '');
        setEditHeadersJson(
            account.fingerprint_profile?.headers
                ? JSON.stringify(account.fingerprint_profile.headers, null, 2)
                : ''
        );
    };

    const saveRouting = async () => {
        if (!editingAccount) return;
        try {
            setBusy(true);
            setError(null);
            let parsedHeaders: Record<string, string> | null = null;
            if (editHeadersJson.trim()) {
                parsedHeaders = JSON.parse(editHeadersJson.trim());
            }
            await updateAccountRouting(
                editingAccount.uid,
                editProxyUrl.trim() || null,
                editUserAgent.trim() || null,
                parsedHeaders
            );
            setEditingAccount(null);
            await refresh();
        } catch (e: any) {
            setError(e.message || ipcErrorMessage(e));
        } finally {
            setBusy(false);
        }
    };

    return (
        <div>
            <h1>{t('accounts.title')}</h1>
            <div className="card">
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '1rem' }}>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('accounts.count')}: {accounts.length}</span>
                    <span>
                        <button className="btn" onClick={() => void refresh()} disabled={busy}>{t('common.refresh')}</button>{' '}
                        <button className="btn btn-primary" onClick={() => void chooseAuthFile()} disabled={busy}>{t('accounts.add')}</button>
                    </span>
                </div>
                <table>
                    <thead>
                        {loading && <tr><td colSpan={7}>{t('common.loading')}</td></tr>}
                        {!loading && accounts.length === 0 && <tr>
                            <th>{t('accounts.col_uid')}</th>
                            <th>{t('accounts.col_nickname')}</th>
                            <th>Proxy Tunnel</th>
                            <th>{t('accounts.col_credits')}</th>
                            <th>{t('accounts.col_status')}</th>
                            <th>{t('accounts.col_inflight')}</th>
                            <th>{t('accounts.col_actions')}</th>
                        </tr>
                        }
                    </thead>
                    <tbody>
                        {accounts.length === 0 && (
                            <tr>
                                <td colSpan={7} style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>
                                    {t('accounts.none')}
                                </td>
                            </tr>
                        )}
                        {accounts.map((account) => <tr key={account.uid}>
                            <td>{account.uid}</td>
                            <td>{account.nickname}</td>
                            <td>
                                {account.proxy_url ? (
                                    <span style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}>{account.proxy_url}</span>
                                ) : (
                                    <span style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>Default (Direct)</span>
                                )}
                            </td>
                            <td>{account.credits}</td>
                            <td><span className={`badge ${account.disabled || !account.healthy ? 'badge-danger' : 'badge-success'}`}>{account.disabled ? 'disabled' : account.healthy ? 'healthy' : 'unhealthy'}</span></td>
                            <td>{account.in_flight}</td>
                            <td>
                                <button className="btn" onClick={() => openEditModal(account)} disabled={busy} style={{ marginRight: '4px' }}>
                                    Routing &amp; FP
                                </button>
                                <button className="btn" onClick={() => void action(() => account.disabled ? enableAccount(account.uid) : disableAccount(account.uid))} disabled={busy} style={{ marginRight: '4px' }}>
                                    {account.disabled ? 'Enable' : 'Disable'}
                                </button>
                                <button className="btn btn-danger" onClick={() => void action(() => removeAccount(account.uid))} disabled={busy}>
                                    {t('common.delete')}
                                </button>
                            </td>
                        </tr>)}
                    </tbody>
                </table>
                {error && <p className="error-message">{error}</p>}
            </div>

            {/* Modal: Edit Account Routing & Fingerprint */}
            {editingAccount && (
                <div style={{
                    position: 'fixed',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    backgroundColor: 'rgba(0, 0, 0, 0.6)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 1000,
                }}>
                    <div className="card" style={{ width: '500px', maxWidth: '90%' }}>
                        <h2>Configure Account Tunnel &amp; Fingerprint</h2>
                        <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                            Target Account: <strong>{editingAccount.nickname || editingAccount.uid}</strong>
                        </p>

                        <div style={{ marginTop: '1rem' }}>
                            <label style={{ display: 'block', marginBottom: '0.25rem', fontWeight: 600 }}>
                                Dedicated Proxy / VPN URL:
                            </label>
                            <input
                                type="text"
                                style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid var(--border-color)', background: 'var(--bg-card)' }}
                                placeholder="socks5://127.0.0.1:1080 or http://user:pass@host:port"
                                value={editProxyUrl}
                                onChange={(e) => setEditProxyUrl(e.target.value)}
                            />
                            <small style={{ color: 'var(--text-secondary)' }}>Leave blank to bypass proxy (direct connection)</small>
                        </div>

                        <div style={{ marginTop: '1rem' }}>
                            <label style={{ display: 'block', marginBottom: '0.25rem', fontWeight: 600 }}>
                                Custom User-Agent (Fingerprint):
                            </label>
                            <input
                                type="text"
                                style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid var(--border-color)', background: 'var(--bg-card)' }}
                                placeholder="Mozilla/5.0 (Windows NT 10.0; Win64; x64)..."
                                value={editUserAgent}
                                onChange={(e) => setEditUserAgent(e.target.value)}
                            />
                        </div>

                        <div style={{ marginTop: '1rem' }}>
                            <label style={{ display: 'block', marginBottom: '0.25rem', fontWeight: 600 }}>
                                Spoof Headers (JSON object):
                            </label>
                            <textarea
                                rows={4}
                                style={{ width: '100%', padding: '0.5rem', borderRadius: '4px', border: '1px solid var(--border-color)', background: 'var(--bg-card)', fontFamily: 'monospace', fontSize: '0.8rem' }}
                                placeholder='{"sec-ch-ua-platform": "\"Windows\""}'
                                value={editHeadersJson}
                                onChange={(e) => setEditHeadersJson(e.target.value)}
                            />
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '1.5rem' }}>
                            <button className="btn" onClick={() => setEditingAccount(null)} disabled={busy}>
                                {t('common.cancel')}
                            </button>
                            <button className="btn btn-primary" onClick={() => void saveRouting()} disabled={busy}>
                                {t('common.save')}
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
