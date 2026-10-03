import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Users,
    RefreshCw,
    Play,
    Square,
    Plus,
    Rocket,
    Clock,
    Tag,
    Eye,
    EyeOff,
    Sparkles,
    Terminal,
    ExternalLink,
} from 'lucide-react';
import {
    listAccounts,
    getAntigravityOverview,
    getProvidersOverview,
    type AntigravityOverview,
    type ProviderStat
} from '../../../bridge/accounts_bridge';
import type { AccountInfo } from '../../../bridge/types';
import { useProfileStore } from '../store/useProfileStore';

// Assets
import antigravityIcon from '../assets/icons/antigravity-menu.png';
import codebuddyIcon from '../assets/icons/codebuddy.png';
import zedIcon from '../assets/icons/zed.png';
import copilotIcon from '../assets/icons/github-copilot.svg';
import cursorIcon from '../assets/icons/cursor-menu.png';
import windsurfIcon from '../assets/icons/windsurf.svg';
import traeIcon from '../assets/icons/trae.png';
import claudeIcon from '../assets/icons/claude.png';
import codexIcon from '../assets/icons/codex.svg';
import kiroIcon from '../assets/icons/kiro-menu.png';
import qoderIcon from '../assets/icons/qoder.png';
import zcodeIcon from '../assets/icons/zcode.png';

export function DashboardPage() {
    const navigate = useNavigate();
    const { profiles, runningInstances, fetchProfiles, launchInstance, stopInstance } = useProfileStore();
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [providerStats, setProviderStats] = useState<ProviderStat[]>([]);
    const [antigravityData, setAntigravityData] = useState<AntigravityOverview | null>(null);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [privacyMode, setPrivacyMode] = useState<boolean>(true);
    const [launchingId, setLaunchingId] = useState<string | null>(null);
    const [selectedProvider, setSelectedProvider] = useState<string>('antigravity');

    const loadData = async () => {
        try {
            setRefreshing(true);
            const [accs, agOverview, provStats] = await Promise.all([
                listAccounts().catch(() => []),
                getAntigravityOverview().catch(() => null),
                getProvidersOverview().catch(() => []),
                fetchProfiles().catch(() => []),
            ]);
            setAccounts(accs);
            if (agOverview) {
                setAntigravityData(agOverview);
            }
            if (provStats && provStats.length > 0) {
                setProviderStats(provStats);
            }
        } catch (e) {
            console.error('Failed to load dashboard data:', e);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadData();
    }, []);

    const maskEmail = (val: string) => {
        if (!privacyMode) return val;
        if (val.includes('@')) {
            const parts = val.split('@');
            const name = parts[0];
            const visible = name.slice(0, Math.min(8, name.length));
            return `${visible}...@${parts[1]}`;
        }
        return val.length > 8 ? `${val.slice(0, 8)}...` : val;
    };

    const handleQuickLaunch = async (profileId: string) => {
        try {
            setLaunchingId(profileId);
            await launchInstance(profileId);
        } catch (e) {
            console.error('Launch failed:', e);
        } finally {
            setLaunchingId(null);
        }
    };

    const handleQuickStop = async (profileId: string) => {
        try {
            await stopInstance(profileId);
        } catch (e) {
            console.error('Stop failed:', e);
        }
    };

    // Default 15 providers matching Cockpit layout
    const fallbackStats: ProviderStat[] = [
        { id: 'total', name: 'Total accounts', count: 6, badge: null },
        { id: 'relay', name: 'Relay', count: 1, badge: null },
        { id: 'claude', name: 'Claude', count: 0, badge: null },
        { id: 'codex', name: 'Codex', count: 0, badge: '+1' },
        { id: 'antigravity', name: 'Antigravity', count: 4, badge: '+1' },
        { id: 'zed', name: 'Zed', count: 0, badge: null },
        { id: 'github_copilot', name: 'GitHub Copilot', count: 1, badge: null },
        { id: 'windsurf', name: 'Windsurf', count: 0, badge: null },
        { id: 'kiro', name: 'Kiro', count: 0, badge: null },
        { id: 'cursor', name: 'Cursor', count: 0, badge: null },
        { id: 'grok', name: 'Grok CLI', count: 0, badge: null },
        { id: 'codebuddy', name: 'CodeBuddy', count: 1, badge: '+2' },
        { id: 'qoder', name: 'Qoder', count: 0, badge: null },
        { id: 'zcode', name: 'ZCode', count: 0, badge: null },
        { id: 'trae', name: 'Trae', count: 0, badge: '+3' },
    ];

    const displayStats = providerStats.length > 0 ? providerStats : fallbackStats;

    const getProviderIcon = (id: string) => {
        switch (id) {
            case 'total':
                return <Users size={18} color="#38bdf8" />;
            case 'relay':
                return <span style={{ fontWeight: 800, fontSize: '0.82rem', color: '#f97316' }}>AK</span>;
            case 'claude':
                return <img src={claudeIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'codex':
                return <img src={codexIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'antigravity':
                return <img src={antigravityIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'zed':
                return <img src={zedIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'github_copilot':
                return <img src={copilotIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'windsurf':
                return <img src={windsurfIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'kiro':
                return <img src={kiroIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'cursor':
                return <img src={cursorIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'grok':
                return <Terminal size={18} color="#06b6d4" />;
            case 'codebuddy':
                return <img src={codebuddyIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'qoder':
                return <img src={qoderIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'zcode':
                return <img src={zcodeIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            case 'trae':
                return <img src={traeIcon} alt="" style={{ width: 18, height: 18, objectFit: 'contain' }} />;
            default:
                return <Users size={18} color="#94a3b8" />;
        }
    };

    const handleProviderClick = (id: string) => {
        if (id === 'total') {
            setSelectedProvider('all');
        } else {
            setSelectedProvider(id);
        }
    };

    const getPlatformRoute = (id: string) => {
        switch (id) {
            case 'relay': return '/api-relay';
            case 'claude': return '/platforms/claude';
            case 'codex': return '/platforms/codex';
            case 'antigravity': return '/platforms/antigravity';
            case 'zed': return '/platforms/zed';
            case 'github_copilot': return '/platforms/github-copilot';
            case 'windsurf': return '/platforms/windsurf';
            case 'kiro': return '/platforms/kiro';
            case 'cursor': return '/platforms/cursor';
            case 'codebuddy': return '/platforms/codebuddy-global';
            case 'qoder': return '/platforms/qoder';
            case 'trae': return '/platforms/trae';
            default: return '/accounts';
        }
    };

    // Filter accounts by provider domain
    const copilotAccounts = accounts.filter(a => a.domain.includes('copilot') || a.domain.includes('github'));
    const codebuddyAccounts = accounts.filter(a => a.domain.includes('codebuddy'));
    const zedAccounts = accounts.filter(a => a.domain.includes('zed'));

    // Fallbacks for display
    const copilotAccount = copilotAccounts[0] || {
        uid: 'gh_lamminh',
        nickname: 'lamminhtriet02@gmail.com',
        domain: 'github.com/copilot',
        plan_tier: 'PRO',
        is_current: true,
        quota_details: {
            suggestions: 'Included',
            chat: 'Included',
            premium_requests: '0 / 200',
            reset_time: '28d 15h (11/01 07:00)'
        }
    };

    const codebuddyAccount = codebuddyAccounts[0] || {
        uid: 'cb_ninja',
        nickname: 'ninjasamuraitriet@gmail.com',
        domain: 'codebuddy.ai',
        plan_tier: 'FREE',
        is_current: true,
        quota_details: {
            subscription_val: '0 / 100',
            credit_package_val: '0 / 0',
            next_refresh: '11/01/2026, 00:00:00'
        }
    };

    // Current & Recommended accounts for Antigravity widget
    const currentAccount = antigravityData?.current_account || {
        id: 'default_curr',
        email: 'ninjasamuraitriet@gmail.com',
        plan_tier: 'PRO',
        buckets: [
            { bucket_id: 'gemini-weekly', label: 'Gemini (Weekly)', remaining_percent: 87, time_left: '3d 17h 55m (10/07 09:27)' },
            { bucket_id: 'gemini-5h', label: 'Gemini (5h)', remaining_percent: 24, time_left: '2h 53m (10/03 18:25)' },
            { bucket_id: '3p-weekly', label: 'Claude (Weekly)', remaining_percent: 100, time_left: '6d 23h 59m (10/10 15:31)' },
            { bucket_id: '3p-5h', label: 'Claude (5h)', remaining_percent: 100, time_left: '4h 59m (10/03 20:31)' },
        ]
    };

    const recommendedAccount = antigravityData?.recommended_account || {
        id: 'default_rec',
        email: 'vuk560269@gmail.com',
        plan_tier: 'PRO',
        buckets: [
            { bucket_id: 'gemini-weekly', label: 'Gemini (Weekly)', remaining_percent: 25, time_left: '3d 17h 55m (10/07 09:27)' },
            { bucket_id: 'gemini-5h', label: 'Gemini (5h)', remaining_percent: 100, time_left: '2h 53m (10/03 18:25)' },
            { bucket_id: '3p-weekly', label: 'Claude (Weekly)', remaining_percent: 100, time_left: '6d 23h 59m (10/10 15:31)' },
            { bucket_id: '3p-5h', label: 'Claude (5h)', remaining_percent: 100, time_left: '4h 59m (10/03 20:31)' },
        ]
    };

    return (
        <div style={{ maxWidth: 1240, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            {/* Header */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
                <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                        <h1 style={{ fontSize: '1.5rem', fontWeight: 700, letterSpacing: '-0.02em', color: '#fff', margin: 0 }}>
                            Universe Cockpit
                        </h1>
                        <span className="badge badge-success" style={{ fontSize: '0.72rem' }}>
                            <span className="pulse-dot" style={{ width: 6, height: 6 }} /> Trực tuyến
                        </span>
                    </div>
                </div>

                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                    <button className="btn" onClick={loadData} disabled={refreshing}>
                        <RefreshCw size={14} className={refreshing ? 'spin' : ''} /> Làm mới
                    </button>
                    <button className="btn btn-primary" onClick={() => navigate('/instances')}>
                        <Plus size={14} /> Tạo Profile mới
                    </button>
                </div>
            </div>

            {/* Providers Overview Grid (15 items) */}
            <div style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(210px, 1fr))',
                gap: '0.85rem'
            }}>
                {displayStats.map((item) => {
                    const isSelected = selectedProvider === item.id || (selectedProvider === 'all' && item.id === 'total');
                    return (
                        <div
                            key={item.id}
                            onClick={() => handleProviderClick(item.id)}
                            style={{
                                background: isSelected ? '#162338' : '#131b26',
                                border: isSelected ? '1.5px solid #38bdf8' : '1px solid rgba(255, 255, 255, 0.07)',
                                boxShadow: isSelected ? '0 0 12px rgba(56, 189, 248, 0.25)' : 'none',
                                borderRadius: '12px',
                                padding: '0.85rem 1rem',
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                                cursor: 'pointer',
                                transition: 'all 0.2s ease',
                                position: 'relative',
                            }}
                            onMouseEnter={(e) => {
                                if (!isSelected) {
                                    e.currentTarget.style.borderColor = 'rgba(59, 130, 246, 0.4)';
                                    e.currentTarget.style.transform = 'translateY(-1px)';
                                }
                            }}
                            onMouseLeave={(e) => {
                                if (!isSelected) {
                                    e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.07)';
                                    e.currentTarget.style.transform = 'translateY(0)';
                                }
                            }}
                        >
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.85rem' }}>
                                {/* Icon Box */}
                                <div style={{
                                    width: 40,
                                    height: 40,
                                    borderRadius: 10,
                                    background: isSelected ? 'rgba(56, 189, 248, 0.15)' : 'rgba(255, 255, 255, 0.04)',
                                    border: isSelected ? '1px solid rgba(56, 189, 248, 0.3)' : '1px solid rgba(255, 255, 255, 0.06)',
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'center',
                                    flexShrink: 0
                                }}>
                                    {getProviderIcon(item.id)}
                                </div>
                                <div>
                                    <div style={{ fontSize: '0.8rem', color: isSelected ? '#38bdf8' : '#94a3b8', fontWeight: 500 }}>
                                        {item.name}
                                    </div>
                                    <div style={{ fontSize: '1.45rem', fontWeight: 700, color: '#fff', lineHeight: 1.1, marginTop: 2 }}>
                                        {loading ? '--' : item.count}
                                    </div>
                                </div>
                            </div>

                            {/* Optional Badge */}
                            {item.badge && (
                                <span style={{
                                    background: 'rgba(56, 189, 248, 0.15)',
                                    color: '#38bdf8',
                                    border: '1px solid rgba(56, 189, 248, 0.3)',
                                    fontSize: '0.68rem',
                                    fontWeight: 700,
                                    padding: '0.12rem 0.45rem',
                                    borderRadius: 9999,
                                    position: 'absolute',
                                    top: '0.65rem',
                                    right: '0.75rem'
                                }}>
                                    {item.badge}
                                </span>
                            )}
                        </div>
                    );
                })}
            </div>

            {/* Quick Provider Tab Selector */}
            <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap', alignItems: 'center' }}>
                <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginRight: '0.4rem', fontWeight: 600 }}>
                    Xem chi tiết:
                </span>
                <button
                    className="btn"
                    style={{
                        padding: '0.35rem 0.85rem',
                        fontSize: '0.8rem',
                        borderRadius: 20,
                        background: selectedProvider === 'antigravity' ? 'rgba(56, 189, 248, 0.2)' : 'rgba(255, 255, 255, 0.04)',
                        borderColor: selectedProvider === 'antigravity' ? 'var(--primary)' : 'var(--border)',
                        color: selectedProvider === 'antigravity' ? '#38bdf8' : 'var(--text-secondary)'
                    }}
                    onClick={() => setSelectedProvider('antigravity')}
                >
                    Antigravity ({displayStats.find(s => s.id === 'antigravity')?.count || 4})
                </button>
                <button
                    className="btn"
                    style={{
                        padding: '0.35rem 0.85rem',
                        fontSize: '0.8rem',
                        borderRadius: 20,
                        background: selectedProvider === 'github_copilot' ? 'rgba(56, 189, 248, 0.2)' : 'rgba(255, 255, 255, 0.04)',
                        borderColor: selectedProvider === 'github_copilot' ? 'var(--primary)' : 'var(--border)',
                        color: selectedProvider === 'github_copilot' ? '#38bdf8' : 'var(--text-secondary)'
                    }}
                    onClick={() => setSelectedProvider('github_copilot')}
                >
                    GitHub Copilot ({displayStats.find(s => s.id === 'github_copilot')?.count || 1})
                </button>
                <button
                    className="btn"
                    style={{
                        padding: '0.35rem 0.85rem',
                        fontSize: '0.8rem',
                        borderRadius: 20,
                        background: selectedProvider === 'codebuddy' ? 'rgba(56, 189, 248, 0.2)' : 'rgba(255, 255, 255, 0.04)',
                        borderColor: selectedProvider === 'codebuddy' ? 'var(--primary)' : 'var(--border)',
                        color: selectedProvider === 'codebuddy' ? '#38bdf8' : 'var(--text-secondary)'
                    }}
                    onClick={() => setSelectedProvider('codebuddy')}
                >
                    CodeBuddy ({displayStats.find(s => s.id === 'codebuddy')?.count || 1})
                </button>
                <button
                    className="btn"
                    style={{
                        padding: '0.35rem 0.85rem',
                        fontSize: '0.8rem',
                        borderRadius: 20,
                        background: selectedProvider === 'all' ? 'rgba(56, 189, 248, 0.2)' : 'rgba(255, 255, 255, 0.04)',
                        borderColor: selectedProvider === 'all' ? 'var(--primary)' : 'var(--border)',
                        color: selectedProvider === 'all' ? '#38bdf8' : 'var(--text-secondary)'
                    }}
                    onClick={() => setSelectedProvider('all')}
                >
                    Tất cả các nền tảng có tài khoản
                </button>
            </div>

            {/* WIDGET 1: ANTIGRAVITY (Shown when selectedProvider is 'antigravity' or 'all') */}
            {(selectedProvider === 'antigravity' || selectedProvider === 'all') && (
                <div style={{
                    background: '#131b26',
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    borderRadius: '14px',
                    padding: '1.25rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '1rem',
                }}>
                    {/* Antigravity Header */}
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                            <img src={antigravityIcon} alt="Antigravity" style={{ width: 22, height: 22 }} />
                            <span style={{ fontSize: '1.15rem', fontWeight: 700, color: '#fff' }}>
                                Antigravity
                            </span>
                        </div>

                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <button
                                className="btn"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.78rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}
                                onClick={loadData}
                                disabled={refreshing}
                            >
                                <RefreshCw size={13} className={refreshing ? 'spin' : ''} /> Refresh
                            </button>
                            <button
                                className="btn"
                                style={{ padding: '0.35rem 0.6rem' }}
                                onClick={() => setPrivacyMode(!privacyMode)}
                                title={privacyMode ? 'Hiện thông tin đầy đủ' : 'Ẩn email bảo mật'}
                            >
                                {privacyMode ? <EyeOff size={14} /> : <Eye size={14} />}
                            </button>
                            <button
                                className="btn"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.78rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                                onClick={() => navigate('/platforms/antigravity')}
                            >
                                <ExternalLink size={13} /> Quản lý
                            </button>
                        </div>
                    </div>

                    {/* Subcards Grid: Current Account & Recommended Account */}
                    <div style={{
                        display: 'grid',
                        gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))',
                        gap: '1.25rem'
                    }}>
                        {/* Left: CURRENT ACCOUNT */}
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.45rem' }}>
                            <div style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: '0.45rem',
                                color: '#94a3b8',
                                fontSize: '0.72rem',
                                fontWeight: 700,
                                letterSpacing: '0.06em'
                            }}>
                                <Clock size={13} /> CURRENT ACCOUNT
                            </div>

                            <div style={{
                                background: '#0d131f',
                                border: '1px solid rgba(255, 255, 255, 0.06)',
                                borderRadius: '10px',
                                padding: '1rem',
                                display: 'flex',
                                flexDirection: 'column',
                                justifyContent: 'space-between',
                                minHeight: 250
                            }}>
                                <div>
                                    {/* Email & Plan */}
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.85rem' }}>
                                        <span style={{ fontWeight: 600, color: '#fff', fontSize: '0.92rem' }}>
                                            {maskEmail(currentAccount.email)}
                                        </span>
                                        <span style={{
                                            background: currentAccount.plan_tier?.toUpperCase() === 'PRO' ? '#0284c7' : '#475569',
                                            color: '#fff',
                                            fontSize: '0.65rem',
                                            fontWeight: 700,
                                            padding: '0.15rem 0.45rem',
                                            borderRadius: 4
                                        }}>
                                            {currentAccount.plan_tier}
                                        </span>
                                    </div>

                                    {/* Quota items */}
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
                                        {currentAccount.buckets.map((b) => {
                                            const isHigh = b.remaining_percent >= 50;
                                            const barColor = isHigh ? '#22c55e' : '#f97316';
                                            return (
                                                <div key={b.bucket_id}>
                                                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.76rem', marginBottom: 3 }}>
                                                        <span style={{ color: 'var(--text-secondary)' }}>{b.label}</span>
                                                        <span style={{ color: barColor, fontWeight: 700 }}>{b.remaining_percent}%</span>
                                                    </div>
                                                    <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                        <div className="quota-fill" style={{ width: `${b.remaining_percent}%`, background: barColor }} />
                                                    </div>
                                                    <div style={{ fontSize: '0.66rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                                        {b.time_left}
                                                    </div>
                                                </div>
                                            );
                                        })}
                                    </div>
                                </div>

                                {/* Divider & Action icons */}
                                <div style={{
                                    borderTop: '1px dashed rgba(255, 255, 255, 0.1)',
                                    marginTop: '1rem',
                                    paddingTop: '0.65rem',
                                    display: 'flex',
                                    justifyContent: 'flex-end',
                                    gap: '0.85rem',
                                    color: '#64748b'
                                }}>
                                    <span title="Tags" style={{ cursor: 'pointer', display: 'inline-flex' }}><Tag size={15} /></span>
                                    <span title="Làm mới Quota" style={{ cursor: 'pointer', display: 'inline-flex' }} onClick={loadData}><RefreshCw size={15} /></span>
                                    <span title="Khởi chạy Profile" style={{ cursor: 'pointer', display: 'inline-flex' }} onClick={() => navigate('/instances')}><Play size={15} /></span>
                                </div>
                            </div>
                        </div>

                        {/* Right: RECOMMENDED ACCOUNT */}
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.45rem' }}>
                            <div style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: '0.45rem',
                                color: '#94a3b8',
                                fontSize: '0.72rem',
                                fontWeight: 700,
                                letterSpacing: '0.06em'
                            }}>
                                <Sparkles size={13} /> RECOMMENDED ACCOUNT
                            </div>

                            <div style={{
                                background: '#0d131f',
                                border: '1px solid rgba(255, 255, 255, 0.06)',
                                borderRadius: '10px',
                                padding: '1rem',
                                display: 'flex',
                                flexDirection: 'column',
                                justifyContent: 'space-between',
                                minHeight: 250
                            }}>
                                <div>
                                    {/* Email & Plan */}
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.85rem' }}>
                                        <span style={{ fontWeight: 600, color: '#fff', fontSize: '0.92rem' }}>
                                            {maskEmail(recommendedAccount.email)}
                                        </span>
                                        <span style={{
                                            background: recommendedAccount.plan_tier?.toUpperCase() === 'PRO' ? '#0284c7' : '#475569',
                                            color: '#fff',
                                            fontSize: '0.65rem',
                                            fontWeight: 700,
                                            padding: '0.15rem 0.45rem',
                                            borderRadius: 4
                                        }}>
                                            {recommendedAccount.plan_tier}
                                        </span>
                                    </div>

                                    {/* Quota items */}
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
                                        {recommendedAccount.buckets.map((b) => {
                                            const isHigh = b.remaining_percent >= 50;
                                            const barColor = isHigh ? '#22c55e' : '#f97316';
                                            return (
                                                <div key={b.bucket_id}>
                                                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.76rem', marginBottom: 3 }}>
                                                        <span style={{ color: 'var(--text-secondary)' }}>{b.label}</span>
                                                        <span style={{ color: barColor, fontWeight: 700 }}>{b.remaining_percent}%</span>
                                                    </div>
                                                    <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                        <div className="quota-fill" style={{ width: `${b.remaining_percent}%`, background: barColor }} />
                                                    </div>
                                                    <div style={{ fontSize: '0.66rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                                        {b.time_left}
                                                    </div>
                                                </div>
                                            );
                                        })}
                                    </div>
                                </div>

                                {/* Divider & Action icons */}
                                <div style={{
                                    borderTop: '1px dashed rgba(255, 255, 255, 0.1)',
                                    marginTop: '1rem',
                                    paddingTop: '0.65rem',
                                    display: 'flex',
                                    justifyContent: 'flex-end',
                                    gap: '0.85rem',
                                    color: '#64748b'
                                }}>
                                    <span title="Tags" style={{ cursor: 'pointer', display: 'inline-flex' }}><Tag size={15} /></span>
                                    <span title="Làm mới Quota" style={{ cursor: 'pointer', display: 'inline-flex' }} onClick={loadData}><RefreshCw size={15} /></span>
                                    <span title="Khởi chạy Profile" style={{ cursor: 'pointer', display: 'inline-flex' }} onClick={() => navigate('/instances')}><Play size={15} /></span>
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Bottom Button: View all accounts */}
                    <button
                        className="btn"
                        style={{
                            width: '100%',
                            background: 'rgba(37, 99, 235, 0.14)',
                            border: '1px solid rgba(59, 130, 246, 0.3)',
                            color: '#60a5fa',
                            fontWeight: 600,
                            fontSize: '0.88rem',
                            padding: '0.65rem',
                            borderRadius: 8,
                            cursor: 'pointer',
                            transition: 'background 0.2s ease',
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(37, 99, 235, 0.22)')}
                        onMouseLeave={(e) => (e.currentTarget.style.background = 'rgba(37, 99, 235, 0.14)')}
                        onClick={() => navigate('/platforms/antigravity')}
                    >
                        View all accounts (Antigravity)
                    </button>
                </div>
            )}

            {/* WIDGET 2: GITHUB COPILOT (Shown when selectedProvider is 'github_copilot' or 'all') */}
            {(selectedProvider === 'github_copilot' || selectedProvider === 'all') && (
                <div style={{
                    background: '#131b26',
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    borderRadius: '14px',
                    padding: '1.25rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '1rem',
                }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                            <img src={copilotIcon} alt="GitHub Copilot" style={{ width: 22, height: 22 }} />
                            <span style={{ fontSize: '1.15rem', fontWeight: 700, color: '#fff' }}>
                                GitHub Copilot
                            </span>
                        </div>

                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <button
                                className="btn"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.78rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                                onClick={() => navigate('/platforms/github-copilot')}
                            >
                                <ExternalLink size={13} /> Quản lý Copilot
                            </button>
                        </div>
                    </div>

                    <div style={{
                        background: '#0d131f',
                        border: '1px solid rgba(255, 255, 255, 0.06)',
                        borderRadius: '10px',
                        padding: '1.15rem',
                        maxWidth: 480
                    }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.85rem' }}>
                            <span style={{ fontWeight: 600, color: '#fff', fontSize: '0.95rem' }}>
                                {maskEmail(copilotAccount.nickname || copilotAccount.uid)}
                            </span>
                            <div style={{ display: 'flex', gap: '0.4rem', alignItems: 'center' }}>
                                <span style={{ background: '#16a34a', color: '#fff', fontSize: '0.65rem', fontWeight: 700, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                    Current
                                </span>
                                <span style={{ background: '#0284c7', color: '#fff', fontSize: '0.65rem', fontWeight: 700, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                    {copilotAccount.plan_tier || 'PRO'}
                                </span>
                            </div>
                        </div>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.85rem' }}>
                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Inline Suggestions</span>
                                    <span style={{ color: '#22c55e', fontWeight: 600 }}>Included</span>
                                </div>
                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                    <div className="quota-fill" style={{ width: '100%', background: '#22c55e' }} />
                                </div>
                                <div style={{ fontSize: '0.66rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                    {copilotAccount.quota_details?.reset_time || '28d 15h (11/01 07:00)'}
                                </div>
                            </div>

                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Chat messages</span>
                                    <span style={{ color: '#22c55e', fontWeight: 600 }}>Included</span>
                                </div>
                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                    <div className="quota-fill" style={{ width: '100%', background: '#22c55e' }} />
                                </div>
                                <div style={{ fontSize: '0.66rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                    {copilotAccount.quota_details?.reset_time || '28d 15h (11/01 07:00)'}
                                </div>
                            </div>

                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Premium requests</span>
                                    <span style={{ color: '#22c55e', fontWeight: 600 }}>0 / 200</span>
                                </div>
                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                    <div className="quota-fill" style={{ width: '0%', background: '#22c55e' }} />
                                </div>
                            </div>
                        </div>

                        <div style={{
                            borderTop: '1px dashed rgba(255, 255, 255, 0.1)',
                            marginTop: '1rem',
                            paddingTop: '0.65rem',
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            color: '#64748b'
                        }}>
                            <span style={{ fontSize: '0.72rem' }}>10/01/2026 15:01</span>
                            <div style={{ display: 'flex', gap: '0.75rem' }}>
                                <span title="Khởi chạy Profile" style={{ cursor: 'pointer' }} onClick={() => navigate('/instances')}><Play size={14} /></span>
                                <span title="Tags" style={{ cursor: 'pointer' }}><Tag size={14} /></span>
                                <span title="Làm mới Quota" style={{ cursor: 'pointer' }} onClick={loadData}><RefreshCw size={14} /></span>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* WIDGET 3: CODEBUDDY (Shown when selectedProvider is 'codebuddy' || 'all') */}
            {(selectedProvider === 'codebuddy' || selectedProvider === 'all') && (
                <div style={{
                    background: '#131b26',
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    borderRadius: '14px',
                    padding: '1.25rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '1rem',
                }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                            <img src={codebuddyIcon} alt="CodeBuddy" style={{ width: 22, height: 22 }} />
                            <span style={{ fontSize: '1.15rem', fontWeight: 700, color: '#fff' }}>
                                CodeBuddy
                            </span>
                        </div>

                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <button
                                className="btn"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.78rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                                onClick={() => navigate('/platforms/codebuddy-global')}
                            >
                                <ExternalLink size={13} /> Quản lý CodeBuddy
                            </button>
                        </div>
                    </div>

                    <div style={{
                        background: '#0d131f',
                        border: '1px solid rgba(255, 255, 255, 0.06)',
                        borderRadius: '10px',
                        padding: '1.15rem',
                        maxWidth: 480
                    }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.85rem' }}>
                            <span style={{ fontWeight: 600, color: '#fff', fontSize: '0.95rem' }}>
                                {maskEmail(codebuddyAccount.nickname || codebuddyAccount.uid)}
                            </span>
                            <span style={{ background: '#475569', color: '#fff', fontSize: '0.65rem', fontWeight: 700, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                {codebuddyAccount.plan_tier || 'FREE'}
                            </span>
                        </div>

                        <div style={{ fontSize: '0.8rem', color: '#94a3b8', marginBottom: '0.85rem' }}>
                            Usage Status <span style={{ color: '#22c55e', fontWeight: 600, marginLeft: 8 }}>Normal</span>
                        </div>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.85rem' }}>
                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Free Plan Subscription</span>
                                    <span style={{ color: '#22c55e', fontWeight: 600 }}>0 / 100</span>
                                </div>
                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                    <div className="quota-fill" style={{ width: '15%', background: '#22c55e' }} />
                                </div>
                                <div style={{ fontSize: '0.66rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                    Next refresh time: 11/01/2026, 00:00:00
                                </div>
                            </div>

                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Credit Package</span>
                                    <span style={{ color: '#22c55e', fontWeight: 600 }}>0 / 0</span>
                                </div>
                            </div>
                        </div>

                        <div style={{
                            borderTop: '1px dashed rgba(255, 255, 255, 0.1)',
                            marginTop: '1rem',
                            paddingTop: '0.65rem',
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            color: '#64748b'
                        }}>
                            <span style={{ fontSize: '0.72rem' }}>10/01/2026 15:02</span>
                            <div style={{ display: 'flex', gap: '0.75rem' }}>
                                <span title="Khởi chạy Profile" style={{ cursor: 'pointer' }} onClick={() => navigate('/instances')}><Play size={14} /></span>
                                <span title="Tags" style={{ cursor: 'pointer' }}><Tag size={14} /></span>
                                <span title="Làm mới Quota" style={{ cursor: 'pointer' }} onClick={loadData}><RefreshCw size={14} /></span>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* WIDGET 4: OTHER PROVIDERS (When user clicks Zed, Cursor, Windsurf, Trae, etc.) */}
            {selectedProvider !== 'antigravity' && selectedProvider !== 'github_copilot' && selectedProvider !== 'codebuddy' && selectedProvider !== 'all' && (
                <div style={{
                    background: '#131b26',
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    borderRadius: '14px',
                    padding: '1.25rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '1rem',
                }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                            <div style={{ width: 24, height: 24, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                                {getProviderIcon(selectedProvider)}
                            </div>
                            <span style={{ fontSize: '1.15rem', fontWeight: 700, color: '#fff', textTransform: 'capitalize' }}>
                                {displayStats.find(s => s.id === selectedProvider)?.name || selectedProvider}
                            </span>
                        </div>

                        <button
                            className="btn btn-primary"
                            style={{ padding: '0.35rem 0.75rem', fontSize: '0.78rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                            onClick={() => navigate(getPlatformRoute(selectedProvider))}
                        >
                            <ExternalLink size={13} /> Mở giao diện quản lý
                        </button>
                    </div>

                    {selectedProvider === 'zed' && zedAccounts.length > 0 ? (
                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: '1rem' }}>
                            {zedAccounts.map(acc => (
                                <div key={acc.uid} style={{ background: '#0d131f', border: '1px solid rgba(255,255,255,0.06)', borderRadius: 10, padding: '1rem' }}>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                        <span style={{ fontWeight: 600, color: '#fff' }}>{acc.nickname}</span>
                                        <span className="badge badge-info">{acc.plan_tier || 'PRO'}</span>
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : (
                        <div style={{
                            padding: '2.5rem 1.5rem',
                            textAlign: 'center',
                            background: '#0d131f',
                            borderRadius: '10px',
                            border: '1px dashed rgba(255, 255, 255, 0.1)',
                            display: 'flex',
                            flexDirection: 'column',
                            alignItems: 'center',
                            gap: '0.75rem'
                        }}>
                            <div style={{ width: 44, height: 44, borderRadius: '50%', background: 'rgba(255, 255, 255, 0.05)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                                {getProviderIcon(selectedProvider)}
                            </div>
                            <div style={{ fontWeight: 600, color: '#e2e8f0', fontSize: '0.95rem' }}>
                                Chưa kết nối tài khoản cho {displayStats.find(s => s.id === selectedProvider)?.name || selectedProvider}
                            </div>
                            <p style={{ color: 'var(--text-muted)', fontSize: '0.8rem', maxWidth: 460, margin: 0 }}>
                                Nền tảng này hiện có 0 tài khoản trong Cockpit. Bạn có thể mở giao diện quản lý để đăng nhập hoặc cấu hình tệp thực thi.
                            </p>
                            <button
                                className="btn btn-primary"
                                style={{ marginTop: '0.5rem', fontSize: '0.8rem' }}
                                onClick={() => navigate(getPlatformRoute(selectedProvider))}
                            >
                                <Plus size={14} /> Thêm tài khoản {displayStats.find(s => s.id === selectedProvider)?.name || selectedProvider}
                            </button>
                        </div>
                    )}
                </div>
            )}

            {/* Quick Virtual Profiles Launcher Table */}
            <div className="card" style={{ marginTop: '0.5rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                        <Rocket size={20} color="var(--primary)" />
                        <h2 style={{ fontSize: '1.05rem', fontWeight: 600, color: '#fff', margin: 0 }}>
                            Danh sách Profile Ảo hóa (Instances)
                        </h2>
                    </div>
                    <button className="btn" onClick={() => navigate('/instances')}>
                        Xem tất cả ({profiles.length})
                    </button>
                </div>

                {profiles.length === 0 ? (
                    <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                        <p>Chưa có profile nào được tạo. Hãy tạo một profile để mở IDE với phần cứng và tài khoản cô lập!</p>
                        <button className="btn btn-primary" style={{ marginTop: '1rem' }} onClick={() => navigate('/instances')}>
                            <Plus size={14} /> Tạo Profile đầu tiên
                        </button>
                    </div>
                ) : (
                    <table>
                        <thead>
                            <tr>
                                <th>Tên Profile</th>
                                <th>Nền tảng</th>
                                <th>Mã máy giả lập (Spoofing)</th>
                                <th>Trạng thái</th>
                                <th style={{ textAlign: 'right' }}>Thao tác tức thì</th>
                            </tr>
                        </thead>
                        <tbody>
                            {profiles.slice(0, 5).map((p) => {
                                const pid = runningInstances[p.id];
                                const isRunning = Boolean(pid);

                                return (
                                    <tr key={p.id}>
                                        <td>
                                            <div style={{ fontWeight: 600, color: '#fff' }}>{p.name}</div>
                                            <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>ID: {p.id.slice(0, 8)}...</div>
                                        </td>
                                        <td>
                                            <span className="badge badge-info">{p.platform_id}</span>
                                        </td>
                                        <td>
                                            <code style={{ fontSize: '0.75rem', background: 'rgba(0,0,0,0.3)', padding: '0.2rem 0.4rem', borderRadius: 4 }}>
                                                {p.hardware_fingerprint?.machine_id ? `${p.hardware_fingerprint.machine_id.slice(0, 14)}...` : 'N/A'}
                                            </code>
                                        </td>
                                        <td>
                                            {isRunning ? (
                                                <span className="badge badge-success">
                                                    <span className="pulse-dot" style={{ width: 6, height: 6 }} /> Đang chạy (PID {pid})
                                                </span>
                                            ) : (
                                                <span className="badge badge-warning">Đã dừng</span>
                                            )}
                                        </td>
                                        <td style={{ textAlign: 'right' }}>
                                            {isRunning ? (
                                                <button
                                                    className="btn btn-danger"
                                                    style={{ padding: '0.35rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => handleQuickStop(p.id)}
                                                >
                                                    <Square size={12} /> Dừng
                                                </button>
                                            ) : (
                                                <button
                                                    className="btn btn-success"
                                                    style={{ padding: '0.35rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => handleQuickLaunch(p.id)}
                                                    disabled={launchingId === p.id}
                                                >
                                                    <Play size={12} /> {launchingId === p.id ? 'Đang mở...' : 'Khởi chạy'}
                                                </button>
                                            )}
                                        </td>
                                    </tr>
                                );
                            })}
                        </tbody>
                    </table>
                )}
            </div>
        </div>
    );
}
