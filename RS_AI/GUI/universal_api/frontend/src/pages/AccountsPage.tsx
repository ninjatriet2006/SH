import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { open } from '@tauri-apps/plugin-dialog';
import {
    addAccount,
    disableAccount,
    enableAccount,
    listAccounts,
    probeAccount,
    removeAccount,
    testAccount,
} from '../../../bridge/accounts_bridge';
import { getServingAccount } from '../../../bridge/gateway_bridge';
import { loginCancel, loginPoll, loginStart, openLoginUrl } from '../../../bridge/login_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';
import type { AccountInfo, ServingInfo } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

type LoginPhase = 'idle' | 'starting' | 'waiting' | 'done' | 'error';

const POLL_INTERVAL_MS = 3000;
const POLL_TIMEOUT_MS = 5 * 60 * 1000;

export function AccountsPage() {
    const { t } = useTranslation();
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [testMsg, setTestMsg] = useState<{ uid: string; msg: string; ok: boolean } | null>(null);
    const [probeMsg, setProbeMsg] = useState<string | null>(null);
    const [probeOk, setProbeOk] = useState(true);
    const [serving, setServing] = useState<ServingInfo | null>(null);

    // Browser device-login state (port Go login.sh: url → browser → poll)
    const [loginOpen, setLoginOpen] = useState(false);
    const [loginRealm, setLoginRealm] = useState('cn');
    const [loginPhase, setLoginPhase] = useState<LoginPhase>('idle');
    const [loginUrl, setLoginUrl] = useState('');
    const [loginMsg, setLoginMsg] = useState<string | null>(null);
    const pollTimer = useRef<number | null>(null);
    const pollDeadline = useRef(0);
    // Chống race poll chồng nhau: backend poll treo tới 30s trong khi tick 3s —
    // poll cũ về trễ với "no pending login" sẽ đè lên kết quả done. Guard kép:
    // (1) không tick khi đang có poll bay, (2) generation — response cũ bị lờ.
    const pollGen = useRef(0);
    const pollFlying = useRef(false);

    const stopPolling = () => {
        if (pollTimer.current !== null) {
            window.clearInterval(pollTimer.current);
            pollTimer.current = null;
        }
    };

    useEffect(() => stopPolling, []);

    const closeLogin = async () => {
        pollGen.current++; // vô hiệu mọi response đang bay
        stopPolling();
        try {
            if (loginPhase === 'waiting' || loginPhase === 'starting') await loginCancel();
        } catch { /* best-effort */ }
        setLoginOpen(false);
        setLoginPhase('idle');
        setLoginUrl('');
        setLoginMsg(null);
    };

    const beginLogin = async () => {
        const gen = ++pollGen.current;
        try {
            setLoginPhase('starting');
            setLoginMsg(null);
            const r = await loginStart(loginRealm);
            if (gen !== pollGen.current) return; // user đã hủy/retry giữa chừng
            setLoginUrl(r.auth_url);
            setLoginPhase('waiting');
            pollDeadline.current = Date.now() + POLL_TIMEOUT_MS;
            stopPolling();
            pollTimer.current = window.setInterval(() => void pollLogin(gen), POLL_INTERVAL_MS);
        } catch (e) {
            if (gen !== pollGen.current) return;
            setLoginPhase('error');
            setLoginMsg(ipcErrorMessage(e));
        }
    };

    const pollLogin = async (gen: number) => {
        if (gen !== pollGen.current || pollFlying.current) return;
        if (Date.now() > pollDeadline.current) {
            pollGen.current++;
            stopPolling();
            setLoginPhase('error');
            setLoginMsg('Login timed out (5 min). Please try again.');
            try { await loginCancel(); } catch { /* ignore */ }
            return;
        }
        pollFlying.current = true;
        try {
            const r = await loginPoll();
            if (gen !== pollGen.current) return;
            if (r.status === 'done' && r.account) {
                pollGen.current++; // khóa: poll trễ không được đè kết quả done
                stopPolling();
                setLoginPhase('done');
                setLoginMsg(`${r.account.uid} — ${r.account.checkin}`);
                await refresh();
            }
        } catch (e) {
            if (gen !== pollGen.current) return;
            // Lỗi mạng thoáng qua thì chờ tick sau; chỉ dừng khi backend nói
            // hết phiên (cancel/timeout) hoặc lỗi thật khác pending.
            const msg = ipcErrorMessage(e);
            if (!/pending login/i.test(msg)) {
                stopPolling();
                setLoginPhase('error');
                setLoginMsg(msg);
            }
            // "no pending login" ở đây = poll trễ sau done/cancel → lờ đi.
        } finally {
            pollFlying.current = false;
        }
    };

    const openBrowser = async () => {
        try {
            await openLoginUrl(loginUrl);
        } catch (e) {
            setLoginMsg(ipcErrorMessage(e));
        }
    };

    const refresh = async () => {
        try { setError(null); setAccounts(await listAccounts()); }
        catch (e) { setError(ipcErrorMessage(e)); }
        finally { setLoading(false); }
        try { setServing(await getServingAccount()); }
        catch { setServing(null); }
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

    const runTest = async (uid: string) => {
        try {
            setBusy(true); setError(null); setTestMsg(null);
            const r = await testAccount(uid);
            setTestMsg({ uid: r.uid, msg: r.message, ok: r.ok });
            await refresh();
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    const runProbe = async (uid: string) => {
        try {
            setBusy(true); setError(null); setProbeMsg(null);
            const r = await probeAccount(uid);
            const lines = [
                `quota: ${r.quota_ok ? 'OK' : 'FAIL'} (${r.quota_message})`,
                `models: ${r.models.ok ? 'OK' : 'FAIL'} (${r.models.message})`,
                `chat: HTTP ${r.chat.http_status} ${r.chat.ok ? 'OK' : 'FAIL'} — ${r.chat.message}`,
                `chat preview: ${r.chat.preview || '(empty)'}`,
            ];
            setProbeOk(r.quota_ok && r.models.ok && r.chat.ok);
            setProbeMsg(`${r.uid}\n${lines.join('\n')}`);
            await refresh();
        } catch (e) { setError(ipcErrorMessage(e)); }
        finally { setBusy(false); }
    };

    return (
        <div>
            <h1>{t('accounts.title')} <span className="badge badge-success">CodeBuddy</span></h1>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                {t('accounts.origin_note')} <Link to="/providers">{t('sidebar.providers')}</Link>
                {serving?.gateway_running && serving.sticky_sessions > 0 && (
                    <span style={{ marginLeft: '0.5rem' }}>· {t('accounts.sticky')}: {serving.sticky_sessions}</span>
                )}
            </p>
            <div className="card">
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '1rem' }}>
                    <span style={{ color: 'var(--text-secondary)' }}>{t('accounts.count')}: {accounts.length}</span>
                    <span>
                        <button className="btn" onClick={() => void refresh()} disabled={busy}>{t('common.refresh')}</button>{' '}
                        <button className="btn btn-primary" onClick={() => { setLoginOpen(true); }} disabled={busy}>{t('accounts.login_browser')}</button>{' '}
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
                            <td>{account.uid}</td>
                            <td>{account.nickname}</td>
                            <td>{account.credits}</td>
                            <td><span className={`badge ${account.disabled || !account.healthy ? 'badge-danger' : 'badge-success'}`} title={account.disabled ? (account.disabled_reason || 'disabled') : (account.cooling ? `${account.cool_kind ?? 'cooling'} (${account.cool_remaining_sec ?? 0}s)` : 'healthy')}>{account.disabled ? 'disabled' : account.healthy ? 'healthy' : 'unhealthy'}</span>
                                {serving?.gateway_running && serving.serving_uid === account.uid && (
                                    <span className="badge badge-info" style={{ marginLeft: '4px' }} title={t('accounts.serving_hint')}>● {t('accounts.serving')}</span>
                                )}
                            </td>
                            <td>{account.in_flight}</td>
                            <td>
                                <button className="btn" onClick={() => void runTest(account.uid)} disabled={busy} style={{ marginRight: '4px' }} title={t('accounts.test_hint')}>
                                    {t('accounts.test')}
                                </button>
                                <button className="btn" onClick={() => void runProbe(account.uid)} disabled={busy} style={{ marginRight: '4px' }} title={t('accounts.probe_hint')}>
                                    {t('accounts.probe')}
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
                {testMsg && <p className={testMsg.ok ? 'success-message' : 'error-message'}>{testMsg.uid}: {testMsg.msg}</p>}
                {probeMsg && <pre className={probeOk ? 'success-message' : 'error-message'} style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{probeMsg}</pre>}
            </div>

            {/* Modal: Browser device-login (port Go login.sh) */}
            {loginOpen && (
                <div style={{
                    position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
                    backgroundColor: 'rgba(0, 0, 0, 0.6)', display: 'flex',
                    alignItems: 'center', justifyContent: 'center', zIndex: 1000,
                }}>
                    <div className="card" style={{ width: '560px', maxWidth: '92%' }}>
                        <h2>{t('accounts.login_title')}</h2>
                        {loginPhase === 'idle' && (
                            <div style={{ display: 'grid', gap: '0.75rem', marginTop: '1rem' }}>
                                <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('accounts.login_hint')}</p>
                                <label style={{ display: 'flex', gap: '1rem', fontSize: '0.9rem' }}>
                                    <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', cursor: 'pointer' }}>
                                        <input type="radio" checked={loginRealm === 'cn'} onChange={() => setLoginRealm('cn')} />
                                        CN — codebuddy.cn
                                    </label>
                                    <label style={{ display: 'flex', gap: '0.4rem', alignItems: 'center', cursor: 'pointer' }}>
                                        <input type="radio" checked={loginRealm === 'intl'} onChange={() => setLoginRealm('intl')} />
                                        Intl — codebuddy.ai (Google)
                                    </label>
                                </label>
                                <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
                                    <button className="btn" onClick={() => void closeLogin()}>{t('common.cancel')}</button>
                                    <button className="btn btn-primary" onClick={() => void beginLogin()}>{t('accounts.login_start')}</button>
                                </div>
                            </div>
                        )}
                        {loginPhase === 'starting' && <p className="status-message">{t('common.loading')}</p>}
                        {(loginPhase === 'waiting' || loginPhase === 'done' || loginPhase === 'error') && (
                            <div style={{ display: 'grid', gap: '0.75rem', marginTop: '1rem' }}>
                                {loginPhase === 'waiting' && (
                                    <>
                                        <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{t('accounts.login_waiting')}</p>
                                        <input className="input" style={{ width: '100%' }} readOnly value={loginUrl} onFocus={(e) => e.target.select()} />
                                        <div style={{ display: 'flex', gap: '0.5rem' }}>
                                            <button className="btn btn-primary" onClick={() => void openBrowser()}>{t('accounts.login_open_browser')}</button>
                                            <button className="btn" onClick={() => void pollLogin(pollGen.current)}>{t('accounts.login_poll_now')}</button>
                                        </div>
                                        <p className="status-message">{t('accounts.login_polling')}</p>
                                    </>
                                )}
                                {loginMsg && <p className={loginPhase === 'error' ? 'error-message' : 'success-message'}>{loginMsg}</p>}
                                <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
                                    {loginPhase !== 'waiting' && (
                                        <button className="btn" onClick={() => { setLoginPhase('idle'); setLoginUrl(''); setLoginMsg(null); }}>
                                            {t('accounts.login_retry')}
                                        </button>
                                    )}
                                    <button className="btn btn-primary" onClick={() => void closeLogin()}>{t('common.close')}</button>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}
