import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { addAccount, disableAccount, enableAccount, listAccounts, removeAccount } from '../../../bridge/accounts_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccountInfo } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

export function AccountsPage() {
    const { t } = useTranslation();
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
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
                        {loading && <tr><td colSpan={6}>{t('common.loading')}</td></tr>}
                        {!loading && accounts.length === 0 && <tr>
                            <th>{t('accounts.col_uid')}</th>
                            <th>{t('accounts.col_nickname')}</th>
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
                                <td colSpan={6} style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>
                                    {t('accounts.none')}
                                </td>
                            </tr>
                        )}
                        {accounts.map((account) => <tr key={account.uid}>
                            <td>{account.uid}</td><td>{account.nickname}</td><td>{account.credits}</td>
                            <td><span className={`badge ${account.disabled || !account.healthy ? 'badge-danger' : 'badge-success'}`}>{account.disabled ? 'disabled' : account.healthy ? 'healthy' : 'unhealthy'}</span></td>
                            <td>{account.in_flight}</td>
                            <td><button className="btn btn-danger" onClick={() => void action(() => removeAccount(account.uid))} disabled={busy}>{t('common.delete')}</button>{' '}<button className="btn" onClick={() => void action(() => account.disabled ? enableAccount(account.uid) : disableAccount(account.uid))} disabled={busy}>{account.disabled ? 'Enable' : 'Disable'}</button></td>
                        </tr>)}
                    </tbody>
                </table>
                {error && <p className="error-message">{error}</p>}
            </div>
        </div>
    );
}
