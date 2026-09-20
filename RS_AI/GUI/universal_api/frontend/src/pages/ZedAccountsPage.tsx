import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import {
    listZedAccounts,
    importZedAccount,
    setZedEnabled,
    removeZedAccount,
    testZedAccount,
    refreshZedModels,
} from '../../../bridge/zed_bridge';
import type { ZedAccount, ZedModel } from '../../../bridge/zed_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import { useTranslation } from '../utils/i18n';

// Zed/Account — song song CodeBuddy/Accounts: quản trị credential trong vault.
export function ZedAccountsPage() {
    const { t } = useTranslation();
    const [accounts, setAccounts] = useState<ZedAccount[]>([]);
    const [models, setModels] = useState<ZedModel[]>([]);
    const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);
    const [working, setWorking] = useState(false);
    const [loading, setLoading] = useState(true);

    const reload = async () => {
        try {
            setAccounts(await listZedAccounts());
        } catch (e) {
            setMsg({ text: ipcErrorMessage(e), ok: false });
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void reload();
    }, []);

    const run = async (label: string, fn: () => Promise<string | null>) => {
        try {
            setWorking(true);
            setMsg(null);
            const out = await fn();
            if (out) setMsg({ text: out, ok: true });
            await reload();
        } catch (e) {
            setMsg({ text: `${label}: ${ipcErrorMessage(e)}`, ok: false });
        } finally {
            setWorking(false);
        }
    };

    const doImport = () =>
        void run('import', async () => {
            const path = await open({ multiple: false, filters: [{ name: 'JSON', extensions: ['json'] }] });
            if (typeof path !== 'string') return null;
            const r = await importZedAccount(path);
            return `${r.id} — ${r.label}${r.plan ? ` · ${r.plan}` : ''}${r.period ? ` · ${r.period}` : ''}`;
        });

    const doTest = (id: string) =>
        void run('test', async () => {
            const r = await testZedAccount(id);
            if (!r.ok) throw new Error(r.message);
            return `${r.id} — ${r.login}${r.plan ? ` · ${r.plan}` : ''} · ${r.quota}`;
        });

    const doToggle = (id: string, enabled: boolean) =>
        void run('toggle', async () => {
            await setZedEnabled(id, enabled);
            return null;
        });

    const doRemove = (id: string) =>
        void run('remove', async () => {
            await removeZedAccount(id);
            return null;
        });

    const doRefreshModels = () =>
        void run('models', async () => {
            const list = await refreshZedModels();
            setModels(list);
            return `${list.length} models`;
        });

    if (loading) return <div>{t('common.loading')}</div>;

    return (
        <div>
            <h1>{t('sidebar.zed')} — {t('providers.account_section')}</h1>
            <div className="card">
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('providers.zed_account_note')}</p>
                <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.5rem', flexWrap: 'wrap' }}>
                    <button className="btn btn-primary" disabled={working} onClick={doImport}>
                        {t('providers.zed_import')}
                    </button>
                    <button className="btn" disabled={working} onClick={doRefreshModels}>
                        {t('providers.refresh_models')} ({models.length})
                    </button>
                </div>
                {accounts.length === 0 ? (
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('providers.zed_no_accounts')}</p>
                ) : (
                    <table>
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>{t('accounts.col_nickname')}</th>
                                <th>{t('accounts.col_status')}</th>
                                <th>{t('accounts.col_actions')}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {accounts.map((a) => (
                                <tr key={a.id}>
                                    <td><code style={{ fontSize: '0.8rem' }}>{a.id}</code></td>
                                    <td>{a.label || '—'}{a.org_id ? <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}> · {a.org_id}</span> : null}</td>
                                    <td>
                                        <span className={`badge ${a.enabled ? 'badge-success' : 'badge-warning'}`}>
                                            {a.enabled ? 'ON' : 'OFF'}
                                        </span>
                                    </td>
                                    <td>
                                        <div style={{ display: 'flex', gap: '0.4rem', flexWrap: 'wrap' }}>
                                            <button className="btn" disabled={working} onClick={() => doTest(a.id)}>
                                                {t('providers.zed_test')}
                                            </button>
                                            <button className="btn" disabled={working} onClick={() => doToggle(a.id, !a.enabled)}>
                                                {a.enabled ? t('accounts.disable') : t('accounts.enable')}
                                            </button>
                                            <button className="btn btn-danger" disabled={working} onClick={() => doRemove(a.id)}>
                                                {t('common.delete')}
                                            </button>
                                        </div>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                )}
                {models.length > 0 && (
                    <ul style={{ maxHeight: 200, overflow: 'auto', fontSize: '0.85rem', marginTop: '0.5rem' }}>
                        {models.map((m) => (
                            <li key={m.id}>
                                <code>{m.id}</code>{' '}
                                <span className="badge badge-info" style={{ fontSize: '0.7rem' }}>{m.provider}</span>{' '}
                                {m.max_output_tokens ? <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>max {m.max_output_tokens}</span> : null}
                            </li>
                        ))}
                    </ul>
                )}
            </div>

            {msg && <p className={msg.ok ? 'success-message' : 'error-message'}>{msg.text}</p>}
        </div>
    );
}
