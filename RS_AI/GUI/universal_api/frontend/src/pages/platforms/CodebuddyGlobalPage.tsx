import { useEffect, useRef, useState } from 'react';
import {
    Users,
    Plus,
    RefreshCw,
    Globe,
    CheckCircle2,
    AlertCircle,
    Trash2,
    ExternalLink,
} from 'lucide-react';
import {
    listAccounts,
    removeAccount,
    testAccount,
} from '../../../../bridge/accounts_bridge';
import { loginStart, loginPoll, openLoginUrl, loginCancel } from '../../../../bridge/login_bridge';
import type { AccountInfo } from '../../../../bridge/types';
import { useProfileStore } from '../../store/useProfileStore';

export function CodebuddyGlobalPage() {
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    // Login modal & polling state
    const [loginOpen, setLoginOpen] = useState(false);
    const [loginPhase, setLoginPhase] = useState<'idle' | 'waiting' | 'done' | 'error'>('idle');
    const [authUrl, setAuthUrl] = useState('');
    const [pollMsg, setPollMsg] = useState('');
    const pollTimer = useRef<number | null>(null);

    const { createProfile } = useProfileStore();

    const fetchGlobalAccounts = async () => {
        setLoading(true);
        try {
            const all = await listAccounts();
            // Lọc ra các tài khoản thuộc realm Global (codebuddy.ai)
            setAccounts(all.filter((a) => a.domain.includes('codebuddy.ai')));
        } catch (e: any) {
            console.error('Fetch error:', e);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchGlobalAccounts();
        return () => {
            if (pollTimer.current) clearInterval(pollTimer.current);
        };
    }, []);

    const showMsg = (text: string, ok: boolean) => {
        setActionMsg({ text, ok });
        setTimeout(() => setActionMsg(null), 4000);
    };

    const startGlobalLogin = async () => {
        try {
            setLoginPhase('waiting');
            setPollMsg('Đang khởi tạo kết nối Google/Email tới codebuddy.ai...');
            const res = await loginStart('intl');
            setAuthUrl(res.auth_url);
            setPollMsg('Vui lòng hoàn tất xác thực tài khoản Google/Email trên trình duyệt.');
            await openLoginUrl(res.auth_url);

            // Bắt đầu polling
            pollTimer.current = window.setInterval(async () => {
                try {
                    const pollRes = await loginPoll();
                    if (pollRes.status === 'done') {
                        clearInterval(pollTimer.current!);
                        setLoginPhase('done');
                        setPollMsg(`Đăng nhập thành công: ${pollRes.account?.nickname || ''} (${pollRes.account?.uid || ''})`);
                        fetchGlobalAccounts();
                        setTimeout(() => setLoginOpen(false), 2000);
                    }
                } catch (e: any) {
                    console.error('Poll tick failed:', e);
                }
            }, 2500);
        } catch (e: any) {
            setLoginPhase('error');
            setPollMsg(e.message || 'Lỗi khởi động đăng nhập');
        }
    };

    const cancelLogin = async () => {
        if (pollTimer.current) clearInterval(pollTimer.current);
        await loginCancel().catch(() => {});
        setLoginOpen(false);
        setLoginPhase('idle');
    };

    const handleDelete = async (uid: string) => {
        try {
            await removeAccount(uid);
            showMsg(`Đã xóa tài khoản ${uid}`, true);
            fetchGlobalAccounts();
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi xóa', false);
        }
    };

    const handleTest = async (uid: string) => {
        try {
            const res = await testAccount(uid);
            showMsg(`Kiểm tra ${uid}: ${res.ok ? 'Khỏe mạnh (OK)' : 'Lỗi kết nối'} - ${res.message}`, res.ok);
        } catch (e: any) {
            showMsg(`Lỗi test: ${e.message}`, false);
        }
    };

    return (
        <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
            {/* Header */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
                <div>
                    <h1 style={{ fontSize: '1.75rem', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <Globe color="var(--primary)" size={28} />
                        CodeBuddy Global (codebuddy.ai)
                    </h1>
                    <p style={{ color: 'var(--text-secondary)', marginTop: '0.25rem', fontSize: '0.9rem' }}>
                        Quản lý tài khoản CodeBuddy Quốc tế (<code style={{ color: 'var(--primary)' }}>www.codebuddy.ai</code>) với đăng nhập Google OAuth / Email và tracing B3 headers.
                    </p>
                </div>
                <div style={{ display: 'flex', gap: '0.75rem' }}>
                    <button className="btn" onClick={fetchGlobalAccounts} title="Làm mới">
                        <RefreshCw size={16} />
                    </button>
                    <button
                        className="btn btn-primary"
                        onClick={() => { setLoginOpen(true); startGlobalLogin(); }}
                        style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}
                    >
                        <Globe size={16} /> Đăng nhập Google / Email
                    </button>
                </div>
            </div>

            {/* Notification alert */}
            {actionMsg && (
                <div style={{
                    padding: '0.75rem 1rem',
                    borderRadius: '0.5rem',
                    marginBottom: '1rem',
                    background: actionMsg.ok ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                    border: `1px solid ${actionMsg.ok ? 'var(--success)' : 'var(--danger)'}`,
                    color: actionMsg.ok ? 'var(--success)' : 'var(--danger)',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem'
                }}>
                    {actionMsg.ok ? <CheckCircle2 size={18} /> : <AlertCircle size={18} />}
                    {actionMsg.text}
                </div>
            )}

            {/* Accounts list */}
            {loading ? (
                <div className="card" style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)' }}>Đang tải danh sách tài khoản...</div>
            ) : accounts.length === 0 ? (
                <div className="card" style={{ textAlign: 'center', padding: '3rem 1rem' }}>
                    <Users size={48} style={{ opacity: 0.3, marginBottom: '1rem' }} />
                    <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>Chưa có tài khoản CodeBuddy Global nào trong Vault.</p>
                    <button className="btn btn-primary" onClick={() => { setLoginOpen(true); startGlobalLogin(); }}>
                        <Globe size={16} /> Đăng nhập Quốc tế Ngay
                    </button>
                </div>
            ) : (
                <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '1rem' }}>
                    {accounts.map((a) => (
                        <div key={a.uid} className="card" style={{ padding: '1.25rem' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                <div>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                        <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>{a.nickname || a.uid}</h3>
                                        <span style={{
                                            fontSize: '0.75rem',
                                            padding: '0.2rem 0.6rem',
                                            borderRadius: '1rem',
                                            background: a.healthy ? 'rgba(16, 185, 129, 0.2)' : 'rgba(239, 68, 68, 0.2)',
                                            color: a.healthy ? 'var(--success)' : 'var(--danger)',
                                            fontWeight: 600
                                        }}>
                                            {a.healthy ? 'Khỏe mạnh' : 'Cần kiểm tra'}
                                        </span>
                                    </div>
                                    <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginTop: '0.4rem', display: 'flex', gap: '1rem' }}>
                                        <span>Email/UID: <code style={{ color: 'var(--text-primary)' }}>{a.uid}</code></span>
                                        <span>Domain: <strong>{a.domain}</strong></span>
                                        <span>Thành công: <strong>{a.success_count}</strong></span>
                                    </div>
                                </div>

                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                    <button className="btn" onClick={() => handleTest(a.uid)}>Kiểm tra</button>
                                    <button
                                        className="btn btn-primary"
                                        onClick={async () => {
                                            try {
                                                await createProfile(`CodeBuddy Global - ${a.nickname}`, 'codebuddy_global', a.uid);
                                                showMsg(`Đã tạo Profile mới và tiêm tài khoản ${a.nickname}!`, true);
                                            } catch (e: any) {
                                                showMsg(e.message, false);
                                            }
                                        }}
                                        title="Tạo Profile độc lập cho tài khoản này"
                                    >
                                        <Plus size={14} /> Tạo Profile
                                    </button>
                                    <button className="btn btn-danger" onClick={() => handleDelete(a.uid)}>
                                        <Trash2 size={14} />
                                    </button>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            )}

            {/* Modal Đăng nhập Google / Email */}
            {loginOpen && (
                <div style={{
                    position: 'fixed',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'rgba(0,0,0,0.7)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 1000
                }}>
                    <div className="card" style={{ width: '460px', padding: '1.75rem', background: '#1e293b', textAlign: 'center' }}>
                        <Globe size={40} color="var(--primary)" style={{ margin: '0 auto 1rem' }} />
                        <h2 style={{ fontSize: '1.25rem', marginBottom: '0.5rem' }}>Đăng nhập CodeBuddy Quốc tế</h2>
                        <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '1.25rem' }}>
                            {pollMsg}
                            {loginPhase === 'waiting' && <span style={{ display: 'block', marginTop: '0.25rem', color: 'var(--primary)' }}>⏳ Đang chờ xác thực...</span>}
                        </p>

                        {authUrl && (
                            <div style={{ marginBottom: '1.5rem' }}>
                                <button
                                    className="btn btn-primary"
                                    onClick={() => openLoginUrl(authUrl)}
                                    style={{ display: 'inline-flex', alignItems: 'center', gap: '0.5rem', width: '100%', justifyContent: 'center' }}
                                >
                                    <ExternalLink size={16} /> Mở Trang Xác Thực Google / Email
                                </button>
                            </div>
                        )}

                        <button className="btn" onClick={cancelLogin}>Hủy bỏ</button>
                    </div>
                </div>
            )}
        </div>
    );
}
