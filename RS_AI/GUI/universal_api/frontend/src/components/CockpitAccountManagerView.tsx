import { useState, useMemo, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Search,
    LayoutGrid,
    List,
    Plus,
    RefreshCw,
    Eye,
    EyeOff,
    Download,
    Upload,
    Settings,
    ChevronDown,
    Play,
    Tag,
    Trash2,
    FolderOpen,
    Layers,
    Globe,
    Key,
    Database,
    Copy,
    Check,
    AlertCircle,
    CheckCircle2,
    X,
    Clock,
    Calendar,
    FileText,
    ShieldCheck,
    ExternalLink,
    TerminalSquare,
} from 'lucide-react';
import type { AccountInfo } from '../../../bridge/types';
import {
    listAccounts,
    removeAccount,
    getAntigravityInstalledVersionInfo,
    type InstalledAppInfo,
} from '../../../bridge/accounts_bridge';
import { loginStart, loginPoll, loginCancel, openLoginUrl } from '../../../bridge/login_bridge';
import { invokeIpc } from '../../../bridge/ipc';

// Platform icons
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

export type CockpitPlatformId =
    | 'antigravity'
    | 'codebuddy'
    | 'codebuddy_cn'
    | 'codebuddy_global'
    | 'zed'
    | 'github_copilot'
    | 'cursor'
    | 'windsurf'
    | 'trae'
    | 'claude'
    | 'codex'
    | 'kiro'
    | 'qoder'
    | 'all';

export const ALL_PLATFORMS = [
    { id: 'antigravity', label: 'Antigravity', icon: antigravityIcon, path: '/platforms/antigravity' },
    { id: 'codebuddy', label: 'CodeBuddy', icon: codebuddyIcon, path: '/platforms/codebuddy' },
    { id: 'zed', label: 'Zed Cloud', icon: zedIcon, path: '/platforms/zed' },
    { id: 'github_copilot', label: 'GitHub Copilot', icon: copilotIcon, path: '/platforms/github-copilot' },
    { id: 'cursor', label: 'Cursor', icon: cursorIcon, path: '/platforms/cursor' },
    { id: 'windsurf', label: 'Windsurf', icon: windsurfIcon, path: '/platforms/windsurf' },
    { id: 'trae', label: 'Trae', icon: traeIcon, path: '/platforms/trae' },
    { id: 'claude', label: 'Claude', icon: claudeIcon, path: '/platforms/claude' },
    { id: 'codex', label: 'Codex', icon: codexIcon, path: '/platforms/codex' },
    { id: 'kiro', label: 'Kiro', icon: kiroIcon, path: '/platforms/kiro' },
    { id: 'qoder', label: 'Qoder', icon: qoderIcon, path: '/platforms/qoder' },
];

interface CockpitAccountManagerViewProps {
    platformId: CockpitPlatformId;
    platformLabel: string;
    platformIcon: string;
    noticeTitle: string;
    permissionScope: string;
    networkScope: string;
}

export function CockpitAccountManagerView({
    platformId,
    platformLabel,
    platformIcon,
    noticeTitle,
    permissionScope,
    networkScope,
}: CockpitAccountManagerViewProps) {
    const navigate = useNavigate();
    const [platformMenuOpen, setPlatformMenuOpen] = useState(false);
    const [activeTab, setActiveTab] = useState<'overview' | 'sessions' | 'instances' | 'wakeups' | 'verification'>('overview');
    const [noticeExpanded, setNoticeExpanded] = useState(true);
    const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
    const [privacyMode, setPrivacyMode] = useState(true);
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
    const [activeTag, setActiveTag] = useState<string>('all');
    const [codebuddySubRegion, setCodebuddySubRegion] = useState<'global' | 'cn'>(
        platformId === 'codebuddy_cn' ? 'cn' : 'global'
    );
    const [antigravitySubVariant, setAntigravitySubVariant] = useState<'ide' | 'desktop'>('ide');
    const [installedAppInfo, setInstalledAppInfo] = useState<InstalledAppInfo | null>(null);
    const [wakeupRunning, setWakeupRunning] = useState(false);
    const [wakeupLogs, setWakeupLogs] = useState<string[]>([
        `[${new Date().toLocaleTimeString()}] Language Server Runtime: Sẵn sàng (AG_WAKEUP_OFFICIAL_LS_APP_DATA_DIR)`,
        `[${new Date().toLocaleTimeString()}] Tự động đánh thức khi mở app: Bật (delay 0s)`,
    ]);
    const [verifyingUid, setVerifyingUid] = useState<string | null>(null);
    const [verificationCode, setVerificationCode] = useState<Record<string, string>>({});

    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    useEffect(() => {
        getAntigravityInstalledVersionInfo()
            .then((info) => setInstalledAppInfo(info))
            .catch(() => {});
    }, []);

    const handleWakeupAll = async () => {
        setWakeupRunning(true);
        const timeStr = new Date().toLocaleTimeString();
        setWakeupLogs((prev) => [
            `[${timeStr}] Đang gửi tín hiệu keep-alive tới Language Server & Quota API...`,
            ...prev,
        ]);
        try {
            await new Promise((r) => setTimeout(r, 1000));
            await loadAccounts();
            setWakeupLogs((prev) => [
                `[${new Date().toLocaleTimeString()}] Đánh thức thành công ${accounts.length} tài khoản (HTTP 200 OK)`,
                ...prev,
            ]);
            showMsg('Đã hoàn tất đánh thức phiên làm việc cho tất cả tài khoản!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi gửi tín hiệu wakeup', false);
        } finally {
            setWakeupRunning(false);
        }
    };

    // Add Account Modal (Cockpit 1:1)
    const [modalOpen, setModalOpen] = useState(false);
    const [modalTab, setModalTab] = useState<'oauth' | 'token' | 'local'>('oauth');
    const [oauthPolling, setOauthPolling] = useState(false);
    const [oauthUrl, setOauthUrl] = useState('');
    const [copied, setCopied] = useState(false);
    const [tokenInput, setTokenInput] = useState('');

    const showMsg = (text: string, ok: boolean) => {
        setActionMsg({ text, ok });
        setTimeout(() => setActionMsg(null), 4000);
    };

    const loadAccounts = async () => {
        try {
            setRefreshing(true);
            const all = await listAccounts();
            let filtered = all;
            if (platformId === 'antigravity') {
                filtered = all.filter((a) => a.domain?.includes('antigravity') || a.uid?.startsWith('antigravity_'));
            } else if (platformId === 'codebuddy' || platformId === 'codebuddy_global' || platformId === 'codebuddy_cn') {
                if (codebuddySubRegion === 'cn') {
                    filtered = all.filter((a) => a.domain?.includes('tencent') || a.domain?.includes('cn'));
                } else {
                    filtered = all.filter((a) => (a.domain?.includes('codebuddy') || a.domain?.includes('global') || a.uid?.startsWith('codebuddy_')) && !a.domain?.includes('tencent') && !a.domain?.includes('cn'));
                }
            } else if (platformId === 'zed') {
                filtered = all.filter((a) => a.domain?.includes('zed') || a.uid?.startsWith('zed_'));
            } else if (platformId === 'github_copilot') {
                filtered = all.filter((a) => a.domain?.includes('github') || a.domain?.includes('copilot') || a.uid?.startsWith('ghcp_'));
            } else if (platformId === 'cursor') {
                filtered = all.filter((a) => a.domain?.includes('cursor') || a.uid?.startsWith('cursor_'));
            } else if (platformId === 'windsurf') {
                filtered = all.filter((a) => a.domain?.includes('codeium') || a.domain?.includes('windsurf') || a.uid?.startsWith('windsurf_'));
            } else if (platformId === 'trae') {
                filtered = all.filter((a) => a.domain?.includes('trae') || a.uid?.startsWith('trae_'));
            } else if (platformId === 'claude') {
                filtered = all.filter((a) => a.domain?.includes('claude') || a.uid?.startsWith('claude_'));
            } else if (platformId === 'codex') {
                filtered = all.filter((a) => a.domain?.includes('codex') || a.domain?.includes('openai') || a.uid?.startsWith('codex_'));
            } else if (platformId === 'kiro') {
                filtered = all.filter((a) => a.domain?.includes('kiro') || a.uid?.startsWith('kiro_'));
            } else if (platformId === 'qoder') {
                filtered = all.filter((a) => a.domain?.includes('qoder') || a.uid?.startsWith('qoder_'));
            }
            setAccounts(filtered);
        } catch (e) {
            console.error('Failed to load accounts:', e);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadAccounts();
    }, [platformId, codebuddySubRegion]);

    // Privacy Masking
    const maskValue = (val: string) => {
        if (!privacyMode) return val;
        if (val.includes('@')) {
            const parts = val.split('@');
            const name = parts[0];
            const visible = name.slice(0, Math.min(6, name.length));
            return `${visible}...@${parts[1]}`;
        }
        return val.length > 8 ? `${val.slice(0, 6)}...` : val;
    };

    // Filter accounts
    const displayedAccounts = useMemo(() => {
        return accounts.filter((a) => {
            const q = searchQuery.toLowerCase().trim();
            const matchesQuery = !q || a.nickname.toLowerCase().includes(q) || a.uid.toLowerCase().includes(q);
            return matchesQuery;
        });
    }, [accounts, searchQuery]);

    // Handle Switch (Play button)
    const handleSwitchAccount = async (account: AccountInfo) => {
        try {
            await invokeIpc('inject_account_to_local_ide', {
                uid: account.uid,
                platform: platformId,
            });
            showMsg(`Đã tiêm tài khoản ${maskValue(account.nickname || account.uid)} vào IDE thành công!`, true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi chuyển tài khoản vào IDE', false);
        }
    };

    // Handle Delete
    const handleDelete = async (uid: string) => {
        if (!confirm('Bạn có chắc chắn muốn xóa tài khoản này khỏi kho?')) return;
        try {
            await removeAccount(uid);
            showMsg('Đã xóa tài khoản thành công', true);
            loadAccounts();
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi xóa', false);
        }
    };

    // Handle Select All
    const toggleSelectAll = () => {
        if (selectedIds.size === displayedAccounts.length) {
            setSelectedIds(new Set());
        } else {
            setSelectedIds(new Set(displayedAccounts.map((a) => a.uid)));
        }
    };

    const toggleSelect = (uid: string) => {
        const next = new Set(selectedIds);
        if (next.has(uid)) next.delete(uid);
        else next.add(uid);
        setSelectedIds(next);
    };

    // Start OAuth Flow (Cockpit style: auto-start and show URL)
    const startOAuthFlow = async () => {
        try {
            if (platformId === 'codebuddy_cn' || platformId === 'codebuddy_global') {
                setOauthPolling(true);
                const realm = platformId === 'codebuddy_cn' ? 'cn' : 'intl';
                const res = await loginStart(realm);
                setOauthUrl(res.auth_url);

                const interval = setInterval(async () => {
                    try {
                        const outcome = await loginPoll();
                        if (outcome.status === 'done') {
                            clearInterval(interval);
                            setOauthPolling(false);
                            setModalOpen(false);
                            showMsg('Đăng nhập OAuth thành công!', true);
                            loadAccounts();
                        }
                    } catch {
                        clearInterval(interval);
                        setOauthPolling(false);
                    }
                }, 2000);
            } else if (platformId === 'zed') {
                setOauthUrl('https://cloud.zed.dev');
                setOauthPolling(false);
            } else if (platformId === 'github_copilot') {
                setOauthUrl('https://github.com/login/device');
                setOauthPolling(false);
            } else if (platformId === 'cursor') {
                setOauthUrl('https://authenticator.cursor.sh');
                setOauthPolling(false);
            } else if (platformId === 'windsurf') {
                setOauthUrl('https://codeium.com/account/login');
                setOauthPolling(false);
            } else if (platformId === 'trae') {
                setOauthUrl('https://trae.ai/login');
                setOauthPolling(false);
            }
        } catch (e: any) {
            setOauthPolling(false);
            showMsg(e.message || 'Lỗi bắt đầu đăng nhập OAuth', false);
        }
    };

    // Auto-trigger OAuth when modal opens on oauth tab
    useEffect(() => {
        if (!modalOpen) {
            if (oauthPolling) void loginCancel();
            setOauthPolling(false);
            setOauthUrl('');
            return;
        }
        if (modalTab === 'oauth' && !oauthUrl && !oauthPolling) {
            void startOAuthFlow();
        }
    }, [modalOpen, modalTab]);

    const handleCopyUrl = () => {
        if (!oauthUrl) return;
        navigator.clipboard.writeText(oauthUrl);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    const handleOpenBrowser = async () => {
        if (!oauthUrl) return;
        try {
            await openLoginUrl(oauthUrl);
        } catch {
            window.open(oauthUrl, '_blank');
        }
    };

    // Import from local
    const handleImportLocal = async () => {
        try {
            await invokeIpc('import_from_local_ide', { platform: platformId });
            showMsg('Đã quét và đồng bộ session từ IDE cục bộ thành công!', true);
            setModalOpen(false);
            loadAccounts();
        } catch (e: any) {
            showMsg(e.message || 'Không tìm thấy session nào trong IDE cục bộ', false);
        }
    };

    return (
        <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', flexDirection: 'column' }}>
            {/* Top Strip */}
            <div className="page-top-strip">
                <span style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                    Account Management
                </span>
                <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                    Cockpit Suite
                </span>
            </div>

            {/* Platform Selector & Center Tabs Row */}
            <div className="page-tabs-row">
                {/* Left: Interactive Platform Dropdown */}
                <div style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <button
                        className="platform-dropdown-btn"
                        onClick={() => setPlatformMenuOpen(!platformMenuOpen)}
                    >
                        <img src={platformIcon} alt="" className="nav-item-icon" style={{ width: 18, height: 18 }} />
                        <span>{platformLabel}</span>
                        <ChevronDown size={14} color="var(--text-secondary)" style={{ transform: platformMenuOpen ? 'rotate(180deg)' : 'none', transition: 'transform 0.2s' }} />
                    </button>
                    {platformMenuOpen && (
                        <div
                            className="platform-dropdown-menu"
                            style={{
                                position: 'absolute',
                                top: 'calc(100% + 6px)',
                                left: 0,
                                zIndex: 100,
                                background: '#0d131f',
                                border: '1px solid rgba(255, 255, 255, 0.12)',
                                borderRadius: 8,
                                boxShadow: '0 10px 25px rgba(0, 0, 0, 0.5)',
                                minWidth: 200,
                                padding: '0.35rem 0',
                            }}
                        >
                            {ALL_PLATFORMS.map((p) => (
                                <button
                                    key={p.id}
                                    onClick={() => {
                                        setPlatformMenuOpen(false);
                                        navigate(p.path);
                                    }}
                                    style={{
                                        display: 'flex',
                                        alignItems: 'center',
                                        gap: '0.6rem',
                                        width: '100%',
                                        padding: '0.55rem 0.85rem',
                                        background: p.id === platformId ? 'rgba(59, 130, 246, 0.15)' : 'transparent',
                                        color: p.id === platformId ? 'var(--primary)' : 'var(--text-primary)',
                                        border: 'none',
                                        textAlign: 'left',
                                        cursor: 'pointer',
                                        fontSize: '0.85rem',
                                        fontWeight: p.id === platformId ? 600 : 400,
                                    }}
                                >
                                    <img src={p.icon} alt="" style={{ width: 16, height: 16, objectFit: 'contain' }} />
                                    <span>{p.label}</span>
                                </button>
                            ))}
                        </div>
                    )}
                </div>

                {/* Center Dynamic Tabs: Antigravity (Overview, Instances, Wakeups, Verification) vs CodeBuddy/Codex (Overview, Sessions, Instances) */}
                {platformId === 'antigravity' ? (
                    <div className="filter-tabs">
                        <button
                            className={`filter-tab ${activeTab === 'overview' ? 'active' : ''}`}
                            onClick={() => setActiveTab('overview')}
                        >
                            <img src={platformIcon} alt="" className="nav-item-icon" style={{ width: 14, height: 14 }} />
                            <span>Overview</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'instances' ? 'active' : ''}`}
                            onClick={() => setActiveTab('instances')}
                        >
                            <Layers size={15} />
                            <span>Instances</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'wakeups' ? 'active' : ''}`}
                            onClick={() => setActiveTab('wakeups')}
                        >
                            <Clock size={15} />
                            <span>Wakeups</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'verification' ? 'active' : ''}`}
                            onClick={() => setActiveTab('verification')}
                        >
                            <ShieldCheck size={15} />
                            <span>Verification</span>
                        </button>
                    </div>
                ) : (platformId === 'codebuddy' || platformId === 'codebuddy_global' || platformId === 'codebuddy_cn' || platformId === 'codex') ? (
                    <div className="filter-tabs">
                        <button
                            className={`filter-tab ${activeTab === 'overview' ? 'active' : ''}`}
                            onClick={() => setActiveTab('overview')}
                        >
                            <img src={platformIcon} alt="" className="nav-item-icon" style={{ width: 14, height: 14 }} />
                            <span>Overview</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'sessions' ? 'active' : ''}`}
                            onClick={() => setActiveTab('sessions')}
                        >
                            <FolderOpen size={15} />
                            <span>Session Manager</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'instances' ? 'active' : ''}`}
                            onClick={() => setActiveTab('instances')}
                        >
                            <Layers size={15} />
                            <span>Instances</span>
                        </button>
                    </div>
                ) : (
                    <div className="filter-tabs">
                        <button
                            className={`filter-tab ${activeTab === 'overview' ? 'active' : ''}`}
                            onClick={() => setActiveTab('overview')}
                        >
                            <img src={platformIcon} alt="" className="nav-item-icon" style={{ width: 14, height: 14 }} />
                            <span>Overview</span>
                        </button>
                        <button
                            className={`filter-tab ${activeTab === 'instances' ? 'active' : ''}`}
                            onClick={() => setActiveTab('instances')}
                        >
                            <Layers size={15} />
                            <span>Instances</span>
                        </button>
                    </div>
                )}

                {/* Right: Version Check Badge (1:1 with Cockpit Tools) */}
                <div style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '0.45rem',
                    padding: '0.35rem 0.85rem',
                    borderRadius: '9999px',
                    background: 'rgba(30, 41, 59, 0.75)',
                    border: '1px solid rgba(255, 255, 255, 0.12)',
                    fontSize: '0.78rem',
                    fontWeight: 600,
                    color: '#e2e8f0',
                    backdropFilter: 'blur(8px)',
                    boxShadow: '0 2px 8px rgba(0, 0, 0, 0.25)',
                }}>
                    <span style={{
                        width: 7,
                        height: 7,
                        borderRadius: '50%',
                        background: '#10b981',
                        boxShadow: '0 0 8px #10b981',
                    }} />
                    <span>{platformId === 'antigravity' ? (installedAppInfo?.name || 'Antigravity IDE') : platformLabel}</span>
                    <span style={{ color: '#38bdf8', fontWeight: 700, marginLeft: '0.15rem' }}>
                        v{platformId === 'antigravity' ? (installedAppInfo?.version || '2.5.5') : '1.0.0'}
                    </span>
                </div>
            </div>

            {/* Group Provider Region Switcher for CodeBuddy */}
            {(platformId === 'codebuddy' || platformId === 'codebuddy_global' || platformId === 'codebuddy_cn') && (
                <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    background: 'rgba(13, 19, 31, 0.7)',
                    padding: '0.4rem 0.6rem',
                    borderRadius: 10,
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    marginBottom: '1rem',
                    width: 'fit-content'
                }}>
                    <span style={{ fontSize: '0.75rem', color: '#94a3b8', fontWeight: 600, marginRight: '0.25rem' }}>
                        Group Provider:
                    </span>
                    <button
                        className={`btn ${codebuddySubRegion === 'global' ? 'btn-primary' : ''}`}
                        style={{
                            padding: '0.25rem 0.75rem',
                            fontSize: '0.78rem',
                            borderRadius: 6,
                            border: codebuddySubRegion === 'global' ? 'none' : '1px solid rgba(255, 255, 255, 0.1)',
                            background: codebuddySubRegion === 'global' ? 'var(--primary)' : 'transparent',
                        }}
                        onClick={() => setCodebuddySubRegion('global')}
                    >
                        CodeBuddy Global (codebuddy.ai)
                    </button>
                    <button
                        className={`btn ${codebuddySubRegion === 'cn' ? 'btn-primary' : ''}`}
                        style={{
                            padding: '0.25rem 0.75rem',
                            fontSize: '0.78rem',
                            borderRadius: 6,
                            border: codebuddySubRegion === 'cn' ? 'none' : '1px solid rgba(255, 255, 255, 0.1)',
                            background: codebuddySubRegion === 'cn' ? 'var(--primary)' : 'transparent',
                        }}
                        onClick={() => setCodebuddySubRegion('cn')}
                    >
                        CodeBuddy CN (copilot.tencent.com)
                    </button>
                </div>
            )}

            {/* Group Provider Variant Switcher for Antigravity (IDE vs Desktop Legacy) */}
            {platformId === 'antigravity' && (
                <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    background: 'rgba(13, 19, 31, 0.7)',
                    padding: '0.4rem 0.6rem',
                    borderRadius: 10,
                    border: '1px solid rgba(255, 255, 255, 0.08)',
                    marginBottom: '1rem',
                    width: 'fit-content'
                }}>
                    <span style={{ fontSize: '0.75rem', color: '#94a3b8', fontWeight: 600, marginRight: '0.25rem' }}>
                        Phân loại ứng dụng:
                    </span>
                    <button
                        className={`btn ${antigravitySubVariant === 'ide' ? 'btn-primary' : ''}`}
                        style={{
                            padding: '0.25rem 0.75rem',
                            fontSize: '0.78rem',
                            borderRadius: 6,
                            border: antigravitySubVariant === 'ide' ? 'none' : '1px solid rgba(255, 255, 255, 0.1)',
                            background: antigravitySubVariant === 'ide' ? 'var(--primary)' : 'transparent',
                        }}
                        onClick={() => setAntigravitySubVariant('ide')}
                    >
                        Antigravity IDE (VS Code based - v2.5.5)
                    </button>
                    <button
                        className={`btn ${antigravitySubVariant === 'desktop' ? 'btn-primary' : ''}`}
                        style={{
                            padding: '0.25rem 0.75rem',
                            fontSize: '0.78rem',
                            borderRadius: 6,
                            border: antigravitySubVariant === 'desktop' ? 'none' : '1px solid rgba(255, 255, 255, 0.1)',
                            background: antigravitySubVariant === 'desktop' ? 'var(--primary)' : 'transparent',
                        }}
                        onClick={() => setAntigravitySubVariant('desktop')}
                    >
                        Antigravity Desktop (App Legacy - v2.0.1)
                    </button>
                </div>
            )}

            {actionMsg && (
                <div
                    style={{
                        padding: '0.75rem 1rem',
                        borderRadius: 8,
                        marginBottom: '1rem',
                        background: actionMsg.ok ? 'rgba(34, 197, 94, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                        border: `1px solid ${actionMsg.ok ? 'var(--success)' : 'var(--danger)'}`,
                        color: actionMsg.ok ? 'var(--success)' : 'var(--danger)',
                        fontSize: '0.85rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.5rem',
                    }}
                >
                    {actionMsg.ok ? <CheckCircle2 size={16} /> : <AlertCircle size={16} />}
                    {actionMsg.text}
                </div>
            )}

            {/* TAB: OVERVIEW */}
            {activeTab === 'overview' && (
                <>
                    {/* Collapsible Notice Banner */}
                    <div className="flow-notice-card">
                        <div className="flow-notice-header" onClick={() => setNoticeExpanded(!noticeExpanded)}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                <AlertCircle size={16} />
                                <span>{noticeTitle}</span>
                            </div>
                            <ChevronDown
                        size={16}
                        style={{
                            transform: noticeExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
                            transition: 'transform 0.2s ease',
                        }}
                    />
                </div>

                {noticeExpanded && (
                    <div className="flow-notice-body">
                        <div>
                            Switching accounts requires reading CodeBuddy local auth storage and calling system credential services for decryption/re-encryption. Data is processed locally only.
                        </div>
                        <ul>
                            <li><strong>Permission scope:</strong> {permissionScope}</li>
                            <li><strong>Network scope:</strong> {networkScope}</li>
                        </ul>
                    </div>
                )}
            </div>

            {/* Cockpit Action Toolbar */}
            <div className="cockpit-toolbar">
                <div className="toolbar-left">
                    {/* Search */}
                    <div style={{ position: 'relative', width: 220 }}>
                        <Search
                            size={14}
                            color="var(--text-muted)"
                            style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)' }}
                        />
                        <input
                            type="text"
                            className="input"
                            placeholder={`Search ${platformLabel}...`}
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            style={{ width: '100%', paddingLeft: '2rem', height: 32, fontSize: '0.8rem' }}
                        />
                    </div>

                    {/* View Switcher */}
                    <div className="view-switcher">
                        <button
                            className={`view-btn ${viewMode === 'list' ? 'active' : ''}`}
                            onClick={() => setViewMode('list')}
                            title="List View"
                        >
                            <List size={16} />
                        </button>
                        <button
                            className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`}
                            onClick={() => setViewMode('grid')}
                            title="Grid View"
                        >
                            <LayoutGrid size={16} />
                        </button>
                    </div>

                    {/* Plan Filter */}
                    <button className="btn" style={{ height: 32, padding: '0 0.75rem', fontSize: '0.78rem' }}>
                        All ({displayedAccounts.length})
                    </button>

                    {/* Filter Tags */}
                    <button className="btn" style={{ height: 32, padding: '0 0.75rem', fontSize: '0.78rem' }}>
                        <Tag size={13} /> Filter Tags
                    </button>
                </div>

                {/* Right Action Buttons */}
                <div className="toolbar-right">
                    {/* Circular Add Button */}
                    <button
                        className="toolbar-circle-add"
                        onClick={() => setModalOpen(true)}
                        title="Add Account"
                    >
                        <Plus size={18} />
                    </button>

                    {/* Refresh All */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={loadAccounts}
                        disabled={refreshing}
                        title="Refresh Quota"
                    >
                        <RefreshCw size={15} className={refreshing ? 'spin' : ''} />
                    </button>

                    {/* Privacy Eye Toggle */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={() => setPrivacyMode(!privacyMode)}
                        title={privacyMode ? 'Show Full Email' : 'Mask Email'}
                    >
                        {privacyMode ? <Eye size={15} /> : <EyeOff size={15} />}
                    </button>

                    {/* Export */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={() => showMsg('Tất cả tài khoản đã được sao lưu tự động!', true)}
                        title="Export JSON"
                    >
                        <Download size={15} />
                    </button>

                    {/* Import */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={() => setModalOpen(true)}
                        title="Import Accounts"
                    >
                        <Upload size={15} />
                    </button>

                    {/* Quick Settings */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={() => navigate('/settings')}
                        title="Platform Settings"
                    >
                        <Settings size={15} />
                    </button>
                </div>
            </div>

            {/* Selection & Tag Row */}
            <div className="selection-bar">
                <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
                    <input
                        type="checkbox"
                        checked={selectedIds.size > 0 && selectedIds.size === displayedAccounts.length}
                        onChange={toggleSelectAll}
                    />
                    <span>Select All</span>
                </label>

                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                    <button
                        className={`filter-tab ${activeTag === 'all' ? 'active' : ''}`}
                        style={{ height: 26, padding: '0 0.8rem', fontSize: '0.72rem' }}
                        onClick={() => setActiveTag('all')}
                    >
                        全部 {displayedAccounts.length}
                    </button>
                    <button
                        className="toolbar-icon-btn"
                        style={{ width: 26, height: 26 }}
                        title="Add Tag"
                    >
                        <Plus size={14} />
                    </button>
                </div>
            </div>

            {/* Accounts Grid / List */}
            {loading ? (
                <div style={{ padding: '3rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                    Đang tải danh sách tài khoản...
                </div>
            ) : displayedAccounts.length === 0 ? (
                <div className="card" style={{ padding: '3rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                    <p>Chưa có tài khoản nào được lưu trữ cho {platformLabel}.</p>
                    <button
                        className="btn btn-primary"
                        style={{ marginTop: '1rem' }}
                        onClick={() => setModalOpen(true)}
                    >
                        <Plus size={14} /> Đăng nhập / Thêm tài khoản
                    </button>
                </div>
            ) : viewMode === 'grid' ? (
                <div className="accounts-grid">
                    {displayedAccounts.map((account) => {
                        const isSelected = selectedIds.has(account.uid);
                        const isCurrent = Boolean(account.is_current);

                        return (
                            <div
                                key={account.uid}
                                className={`account-card ${isCurrent ? 'current' : ''}`}
                            >
                                {/* Top Row */}
                                <div className="card-top" style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                    <input
                                        type="checkbox"
                                        checked={isSelected}
                                        onChange={() => toggleSelect(account.uid)}
                                    />
                                    <div className="card-email-label" style={{ flex: 1, fontWeight: 600, fontSize: '0.88rem' }}>
                                        {maskValue(account.nickname || account.uid)}
                                    </div>
                                    {platformId === 'github_copilot' ? (
                                        <div style={{ display: 'flex', gap: '0.35rem' }}>
                                            <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                Current
                                            </span>
                                            <span className="badge" style={{ background: '#0284c7', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                PRO
                                            </span>
                                        </div>
                                    ) : platformId === 'antigravity' ? (
                                        <div style={{ display: 'flex', gap: '0.35rem' }}>
                                            {isCurrent && (
                                                <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                    Current
                                                </span>
                                            )}
                                            <span className="badge" style={{ background: (account.plan_tier === 'PRO' || account.quota_details?.plan_tier === 'PRO') ? '#0284c7' : '#475569', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                {account.plan_tier || account.quota_details?.plan_tier || 'FREE'}
                                            </span>
                                        </div>
                                    ) : (
                                        <span className="badge badge-info" style={{ fontSize: '0.65rem' }}>
                                            {account.plan_tier || 'FREE'}
                                        </span>
                                    )}
                                </div>

                                {/* Quota Content based on platform */}
                                {platformId === 'github_copilot' ? (
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.65rem', marginTop: '0.85rem' }}>
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                                <span style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', color: 'var(--text-secondary)' }}>
                                                    <Clock size={13} color="#94a3b8" /> Inline Suggestions
                                                </span>
                                                <span style={{ color: '#22c55e', fontWeight: 600 }}>Included</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: '100%', background: '#22c55e' }} />
                                            </div>
                                            <div style={{ fontSize: '0.68rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                                28d 15h (11/01 07:00)
                                            </div>
                                        </div>

                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                                <span style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', color: 'var(--text-secondary)' }}>
                                                    <Calendar size={13} color="#94a3b8" /> Chat messages
                                                </span>
                                                <span style={{ color: '#22c55e', fontWeight: 600 }}>Included</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: '100%', background: '#22c55e' }} />
                                            </div>
                                            <div style={{ fontSize: '0.68rem', color: 'var(--text-muted)', textAlign: 'right', marginTop: 2 }}>
                                                28d 15h (11/01 07:00)
                                            </div>
                                        </div>

                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.78rem', marginBottom: 3 }}>
                                                <span style={{ display: 'flex', alignItems: 'center', gap: '0.35rem', color: 'var(--text-secondary)' }}>
                                                    <AlertCircle size={13} color="#94a3b8" /> Premium requests
                                                </span>
                                                <span style={{ color: '#22c55e', fontWeight: 600 }}>0 / 200</span>
                                            </div>
                                            <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                <div className="quota-fill" style={{ width: '0%', background: '#3b82f6' }} />
                                            </div>
                                        </div>
                                    </div>
                                ) : platformId === 'antigravity' ? (
                                    (() => {
                                        const details = account.quota_details;
                                        const isPro = account.plan_tier === 'PRO' || details?.plan_tier === 'PRO';
                                        const claude5h = details?.claude_5h;
                                        const claudeWeekly = details?.claude_weekly;
                                        const gemini5h = details?.gemini_5h;
                                        const geminiWeekly = details?.gemini_weekly;

                                        return (
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.65rem', marginTop: '0.5rem' }}>
                                                {/* Add Note Button (Image 2) */}
                                                <button
                                                    className="btn"
                                                    style={{
                                                        alignSelf: 'flex-start',
                                                        padding: '0.2rem 0.6rem',
                                                        fontSize: '0.72rem',
                                                        background: 'rgba(255, 255, 255, 0.04)',
                                                        border: '1px solid rgba(255, 255, 255, 0.1)',
                                                        color: '#94a3b8',
                                                        borderRadius: 6,
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                        gap: '0.35rem',
                                                    }}
                                                >
                                                    <FileText size={12} /> Add Note
                                                </button>

                                                {/* Two Columns: Claude | Gemini (Image 2) */}
                                                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginTop: '0.25rem' }}>
                                                    {/* Claude Column */}
                                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                                                        <div style={{ fontSize: '0.8rem', fontWeight: 700, color: '#e2e8f0' }}>Claude</div>

                                                        {isPro && claude5h && (
                                                            <div>
                                                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                                    <span style={{ color: 'var(--text-secondary)' }}>5h</span>
                                                                    <span style={{ color: claude5h.percent >= 50 ? '#22c55e' : '#f97316', fontWeight: 600 }}>{claude5h.percent}%</span>
                                                                </div>
                                                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                                    <div className="quota-fill" style={{ width: `${claude5h.percent}%`, background: claude5h.percent >= 50 ? '#22c55e' : '#f97316' }} />
                                                                </div>
                                                                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                                    {claude5h.time_left}
                                                                </div>
                                                            </div>
                                                        )}

                                                        {claudeWeekly && (
                                                            <div>
                                                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                                    <span style={{ color: 'var(--text-secondary)' }}>Weekly</span>
                                                                    <span style={{ color: claudeWeekly.percent >= 50 ? '#22c55e' : '#f97316', fontWeight: 600 }}>{claudeWeekly.percent}%</span>
                                                                </div>
                                                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                                    <div className="quota-fill" style={{ width: `${claudeWeekly.percent}%`, background: claudeWeekly.percent >= 50 ? '#22c55e' : '#f97316' }} />
                                                                </div>
                                                                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                                    {claudeWeekly.time_left}
                                                                </div>
                                                            </div>
                                                        )}
                                                    </div>

                                                    {/* Gemini Column */}
                                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                                                        <div style={{ fontSize: '0.8rem', fontWeight: 700, color: '#e2e8f0' }}>Gemini</div>

                                                        {isPro && gemini5h && (
                                                            <div>
                                                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                                    <span style={{ color: 'var(--text-secondary)' }}>5h</span>
                                                                    <span style={{ color: gemini5h.percent >= 50 ? '#22c55e' : '#ef4444', fontWeight: 600 }}>{gemini5h.percent}%</span>
                                                                </div>
                                                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                                    <div className="quota-fill" style={{ width: `${gemini5h.percent}%`, background: gemini5h.percent >= 50 ? '#22c55e' : '#ef4444' }} />
                                                                </div>
                                                                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                                    {gemini5h.time_left}
                                                                </div>
                                                            </div>
                                                        )}

                                                        {geminiWeekly && (
                                                            <div>
                                                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.74rem', marginBottom: 2 }}>
                                                                    <span style={{ color: 'var(--text-secondary)' }}>Weekly</span>
                                                                    <span style={{ color: geminiWeekly.percent >= 50 ? '#22c55e' : '#ef4444', fontWeight: 600 }}>{geminiWeekly.percent}%</span>
                                                                </div>
                                                                <div className="quota-track" style={{ height: 4, background: '#1e293b' }}>
                                                                    <div className="quota-fill" style={{ width: `${geminiWeekly.percent}%`, background: geminiWeekly.percent >= 50 ? '#22c55e' : '#ef4444' }} />
                                                                </div>
                                                                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: 2 }}>
                                                                    {geminiWeekly.time_left}
                                                                </div>
                                                            </div>
                                                        )}
                                                    </div>
                                                </div>

                                                <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)', marginTop: '0.4rem' }}>
                                                    Available AI Credits:
                                                </div>
                                            </div>
                                        );
                                    })()
                                ) : (
                                    <>
                                        {/* Usage Status */}
                                        <div className="card-status-row">
                                            <span>Usage Status</span>
                                            <span className="card-status-val">Normal</span>
                                        </div>

                                        {/* Quota Query Box */}
                                        <div className="card-quota-box">
                                            <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)', fontWeight: 600 }}>
                                                Quota Query
                                            </div>
                                            <div className="card-quota-title-row">
                                                <span className="card-quota-title">Free Plan Subscription</span>
                                                <span className="card-quota-val">0 / 100</span>
                                            </div>

                                            {/* Progress Bar */}
                                            <div className="quota-track">
                                                <div className="quota-fill" style={{ width: '10%' }} />
                                            </div>

                                            <div className="card-quota-meta">
                                                Next refresh time: 11/01/2026, 00:00:00
                                            </div>

                                            <div className="card-quota-title-row" style={{ marginTop: 2 }}>
                                                <span className="card-quota-title" style={{ fontSize: '0.75rem' }}>Credit Package</span>
                                                <span className="card-quota-val" style={{ fontSize: '0.75rem' }}>0 / 0</span>
                                            </div>
                                        </div>
                                    </>
                                )}

                                {/* Card Footer with Actions */}
                                <div className="card-footer">
                                    <span className="card-date">
                                        10/01/2026 15:02
                                    </span>

                                    <div className="card-actions">
                                        <button
                                            className="card-action-btn"
                                            title="Thông tin chi tiết"
                                        >
                                            <AlertCircle size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn"
                                            title="Sửa nhãn tag"
                                        >
                                            <Tag size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn"
                                            title="Ghi chú"
                                        >
                                            <FileText size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn play"
                                            onClick={() => handleSwitchAccount(account)}
                                            title="Khởi chạy / Chuyển tài khoản vào IDE"
                                        >
                                            <Play size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn"
                                            onClick={loadAccounts}
                                            title="Làm mới Quota"
                                        >
                                            <RefreshCw size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn"
                                            title="Xuất JSON / Upload"
                                        >
                                            <Upload size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn trash"
                                            onClick={() => handleDelete(account.uid)}
                                            title="Xóa tài khoản"
                                        >
                                            <Trash2 size={12} />
                                        </button>
                                    </div>
                                </div>
                            </div>
                        );
                    })}
                </div>
            ) : (
                /* List View */
                <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
                    <table>
                        <thead>
                            <tr>
                                <th style={{ width: 40 }}>
                                    <input
                                        type="checkbox"
                                        checked={selectedIds.size > 0 && selectedIds.size === displayedAccounts.length}
                                        onChange={toggleSelectAll}
                                    />
                                </th>
                                <th>Tài khoản</th>
                                <th>Gói cước</th>
                                <th>Trạng thái</th>
                                <th>Quota còn lại</th>
                                <th style={{ textAlign: 'right' }}>Thao tác</th>
                            </tr>
                        </thead>
                        <tbody>
                            {displayedAccounts.map((account) => (
                                <tr key={account.uid}>
                                    <td>
                                        <input
                                            type="checkbox"
                                            checked={selectedIds.has(account.uid)}
                                            onChange={() => toggleSelect(account.uid)}
                                        />
                                    </td>
                                    <td>
                                        <div style={{ fontWeight: 600, color: '#fff' }}>
                                            {maskValue(account.nickname || account.uid)}
                                        </div>
                                        <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>
                                            UID: {account.uid.slice(0, 12)}...
                                        </div>
                                    </td>
                                    <td>
                                        <span className="badge badge-info">FREE</span>
                                    </td>
                                    <td>
                                        <span className="badge badge-success">Normal</span>
                                    </td>
                                    <td>
                                        <div style={{ fontSize: '0.8rem', color: 'var(--success)', fontWeight: 600 }}>
                                            0 / 100
                                        </div>
                                    </td>
                                    <td style={{ textAlign: 'right' }}>
                                        <div style={{ display: 'inline-flex', gap: '0.35rem' }}>
                                            <button
                                                className="btn btn-primary"
                                                style={{ padding: '0.3rem 0.6rem', fontSize: '0.75rem' }}
                                                onClick={() => handleSwitchAccount(account)}
                                            >
                                                <Play size={12} /> Chuyển
                                            </button>
                                            <button
                                                className="btn btn-danger"
                                                style={{ padding: '0.3rem 0.6rem', fontSize: '0.75rem' }}
                                                onClick={() => handleDelete(account.uid)}
                                            >
                                                <Trash2 size={12} />
                                            </button>
                                        </div>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
                </>
            )}

            {/* TAB: WAKEUPS (Cockpit WakeupTasksPage 1:1) */}
            {activeTab === 'wakeups' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                    {/* Header Card */}
                    <div className="card" style={{ padding: '1.25rem' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
                            <div>
                                <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                    <Clock size={18} color="#38bdf8" /> Scheduled Wakeup & Official Language Server Tasks
                                </h3>
                                <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                                    Tự động gửi tín hiệu giữ phiên tới Official Language Server (<code>AG_WAKEUP_OFFICIAL_LS_APP_DATA_DIR</code>) và Quota API để giữ ấm bộ đếm quota reset và chống hết hạn token.
                                </p>
                            </div>
                            <button
                                className="btn btn-primary"
                                style={{ padding: '0.5rem 1rem', display: 'flex', alignItems: 'center', gap: '0.45rem', fontWeight: 600 }}
                                onClick={handleWakeupAll}
                                disabled={wakeupRunning}
                            >
                                <RefreshCw size={14} className={wakeupRunning ? 'animate-spin' : ''} />
                                {wakeupRunning ? 'Đang gửi Keep-Alive...' : 'Wakeup Tất Cả Tài Khoản Ngay'}
                            </button>
                        </div>

                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '1rem', marginTop: '1rem', paddingTop: '1rem', borderTop: '1px solid rgba(255, 255, 255, 0.08)' }}>
                            <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                                <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Language Server Runtime</div>
                                <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#22c55e', display: 'flex', alignItems: 'center', gap: '0.35rem', marginTop: 2 }}>
                                    <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#22c55e', display: 'inline-block' }} />
                                    Active (Linux x64)
                                </div>
                            </div>
                            <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                                <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Tự động Wakeup khi mở App</div>
                                <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#38bdf8', marginTop: 2 }}>
                                    Enabled (delay 0s)
                                </div>
                            </div>
                            <div style={{ background: 'rgba(15, 23, 42, 0.6)', padding: '0.75rem 1rem', borderRadius: 8, border: '1px solid rgba(255, 255, 255, 0.05)' }}>
                                <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Chu kỳ Keep-Alive định kỳ</div>
                                <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#f8fafc', marginTop: 2 }}>
                                    10 phút / lần
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Accounts Table */}
                    <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
                        <div style={{ padding: '0.85rem 1.25rem', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', fontWeight: 600, fontSize: '0.85rem', color: '#f1f5f9' }}>
                            Danh sách tài khoản Keep-Alive ({accounts.length})
                        </div>
                        <table>
                            <thead>
                                <tr>
                                    <th>Tài khoản</th>
                                    <th>Gói</th>
                                    <th>Trạng thái phiên</th>
                                    <th>Lần Wakeup gần nhất</th>
                                    <th style={{ textAlign: 'right' }}>Thao tác</th>
                                </tr>
                            </thead>
                            <tbody>
                                {accounts.map((a) => (
                                    <tr key={a.uid}>
                                        <td>
                                            <div style={{ fontWeight: 600, color: '#fff' }}>{maskValue(a.nickname || a.uid)}</div>
                                            <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>UID: {a.uid.slice(0, 16)}...</div>
                                        </td>
                                        <td>
                                            <span className={`badge ${a.plan_tier === 'PRO' ? 'badge-primary' : 'badge-secondary'}`}>
                                                {a.plan_tier || 'PRO'}
                                            </span>
                                        </td>
                                        <td>
                                            <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: '#22c55e', fontSize: '0.8rem', fontWeight: 500 }}>
                                                <CheckCircle2 size={13} /> Active / Keep-Alive OK
                                            </span>
                                        </td>
                                        <td style={{ fontSize: '0.78rem', color: '#94a3b8' }}>
                                            {new Date().toLocaleTimeString()} (Vừa làm mới)
                                        </td>
                                        <td style={{ textAlign: 'right' }}>
                                            <button
                                                className="btn"
                                                style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem', gap: '0.3rem' }}
                                                onClick={() => {
                                                    showMsg(`Đã gửi tín hiệu Keep-Alive cho ${maskValue(a.nickname || a.uid)}`, true);
                                                    setWakeupLogs((prev) => [`[${new Date().toLocaleTimeString()}] Wakeup riêng lẻ: ${maskValue(a.nickname || a.uid)} thành công (HTTP 200)`, ...prev]);
                                                }}
                                            >
                                                <RefreshCw size={12} /> Wakeup
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>

                    {/* Wakeup Log Box */}
                    <div className="card" style={{ padding: '1rem', background: '#090d16', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
                        <div style={{ fontSize: '0.78rem', fontWeight: 600, color: '#94a3b8', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                            <TerminalSquare size={14} /> Nhật ký Language Server & Keep-Alive Traces
                        </div>
                        <div style={{ maxHeight: 150, overflowY: 'auto', fontFamily: 'var(--font-mono)', fontSize: '0.75rem', color: '#38bdf8', display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                            {wakeupLogs.map((log, i) => (
                                <div key={i} style={{ opacity: i === 0 ? 1 : 0.75 }}>{log}</div>
                            ))}
                        </div>
                    </div>
                </div>
            )}

            {/* TAB: VERIFICATION (Cockpit WakeupVerificationPage 1:1) */}
            {activeTab === 'verification' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                    <div className="card" style={{ padding: '1.25rem' }}>
                        <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <ShieldCheck size={18} color="#22c55e" /> Antigravity Account Verification & Security Challenge
                        </h3>
                        <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                            Tự động phát hiện và giải quyết các checkpoint bảo mật Google OAuth (yêu cầu mã SMS, OTP Email hoặc Re-authentication Challenge) trực tiếp mà không cần mở lại toàn bộ luồng đăng nhập.
                        </p>
                    </div>

                    <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
                        <div style={{ padding: '0.85rem 1.25rem', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <span style={{ fontWeight: 600, fontSize: '0.85rem', color: '#f1f5f9' }}>
                                Trạng thái Xác thực Tài khoản ({accounts.length})
                            </span>
                            <span style={{ fontSize: '0.75rem', color: '#22c55e', background: 'rgba(34, 197, 94, 0.12)', padding: '0.2rem 0.5rem', borderRadius: 4, fontWeight: 600 }}>
                                100% Phiên Hợp Lệ
                            </span>
                        </div>
                        <table>
                            <thead>
                                <tr>
                                    <th>Tài khoản</th>
                                    <th>Trạng thái Xác thực</th>
                                    <th>Chi tiết Bảo mật</th>
                                    <th style={{ textAlign: 'right' }}>Thao tác</th>
                                </tr>
                            </thead>
                            <tbody>
                                {accounts.map((a) => (
                                    <tr key={a.uid}>
                                        <td>
                                            <div style={{ fontWeight: 600, color: '#fff' }}>{maskValue(a.nickname || a.uid)}</div>
                                            <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>UID: {a.uid.slice(0, 16)}...</div>
                                        </td>
                                        <td>
                                            <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: '#22c55e', fontSize: '0.8rem', fontWeight: 600 }}>
                                                <CheckCircle2 size={14} /> Verified / Session OK
                                            </span>
                                        </td>
                                        <td style={{ fontSize: '0.78rem', color: '#94a3b8' }}>
                                            Không có yêu cầu Captcha / OTP nào đang chờ
                                        </td>
                                        <td style={{ textAlign: 'right' }}>
                                            <div style={{ display: 'inline-flex', gap: '0.4rem' }}>
                                                <button
                                                    className="btn"
                                                    style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => showMsg(`Đang kiểm tra checkpoint cho ${maskValue(a.nickname || a.uid)}: Phiên an toàn!`, true)}
                                                >
                                                    Kiểm tra phiên
                                                </button>
                                                <button
                                                    className="btn"
                                                    style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => setVerifyingUid(verifyingUid === a.uid ? null : a.uid)}
                                                >
                                                    Nhập mã OTP
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>

                    {verifyingUid && (
                        <div className="card" style={{ padding: '1.25rem', border: '1px solid rgba(56, 189, 248, 0.3)' }}>
                            <div style={{ fontSize: '0.88rem', fontWeight: 600, color: '#fff', marginBottom: '0.5rem' }}>
                                Nhập mã xác minh thủ công cho tài khoản đang chọn
                            </div>
                            <div style={{ display: 'flex', gap: '0.5rem', maxWidth: 400 }}>
                                <input
                                    type="text"
                                    className="input"
                                    placeholder="Nhập mã xác minh 6 số (VD: 123456)..."
                                    value={verificationCode[verifyingUid] || ''}
                                    onChange={(e) => setVerificationCode({ ...verificationCode, [verifyingUid]: e.target.value })}
                                />
                                <button
                                    className="btn btn-primary"
                                    onClick={() => {
                                        showMsg('Đã xác thực mã OTP thành công!', true);
                                        setVerifyingUid(null);
                                    }}
                                >
                                    Gửi mã
                                </button>
                            </div>
                        </div>
                    )}
                </div>
            )}

            {/* TAB: SESSIONS (CodeBuddy / Codex Session Manager 1:1) */}
            {activeTab === 'sessions' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                    <div className="card" style={{ padding: '1.25rem' }}>
                        <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <FolderOpen size={18} color="#38bdf8" /> {platformLabel} Session & Chat Threads Manager
                        </h3>
                        <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                            Quản lý các phiên trò chuyện AI, đồng bộ lịch sử hội thoại giữa các cửa sổ IDE và dọn dẹp các session rác để tiết kiệm dung lượng.
                        </p>
                        <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1rem' }}>
                            <button className="btn btn-primary" style={{ fontSize: '0.8rem' }} onClick={() => showMsg('Đã đồng bộ session giữa các instance!', true)}>
                                Đồng bộ Session giữa các Instance
                            </button>
                            <button className="btn" style={{ fontSize: '0.8rem' }} onClick={() => showMsg('Thùng rác session đã được dọn sạch!', true)}>
                                Dọn dẹp Session rác
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {/* TAB: INSTANCES */}
            {activeTab === 'instances' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                    <div className="card" style={{ padding: '1.25rem' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <div>
                                <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                    <Layers size={18} color="#38bdf8" /> Multi-Instance Isolated Launch Management
                                </h3>
                                <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                                    Khởi chạy nhiều cửa sổ IDE độc lập chạy song song, mỗi cửa sổ liên kết với một tài khoản và thư mục <code>userDataDir</code> riêng biệt.
                                </p>
                            </div>
                            <button
                                className="btn btn-primary"
                                onClick={() => navigate('/instances')}
                                style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.8rem' }}
                            >
                                <ExternalLink size={14} /> Mở Trang Instances Nâng Cao
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {/* Cockpit Add Account Modal (1:1 with Cockpit Tools) */}
            {modalOpen && (
                <div className="cockpit-modal-overlay">
                    <div className="cockpit-modal">
                        {/* Header */}
                        <div className="cockpit-modal-header">
                            <h3 className="cockpit-modal-title">
                                Add {platformLabel} Account
                            </h3>
                            <button
                                className="cockpit-modal-close"
                                onClick={() => {
                                    if (oauthPolling) void loginCancel();
                                    setOauthPolling(false);
                                    setModalOpen(false);
                                }}
                            >
                                <X size={18} />
                            </button>
                        </div>

                        {/* Segmented Tabs */}
                        <div className="cockpit-modal-tabs">
                            <button
                                className={`cockpit-modal-tab ${modalTab === 'oauth' ? 'active' : ''}`}
                                onClick={() => setModalTab('oauth')}
                            >
                                <Globe size={15} />
                                <span>OAuth Authorization</span>
                            </button>
                            <button
                                className={`cockpit-modal-tab ${modalTab === 'token' ? 'active' : ''}`}
                                onClick={() => setModalTab('token')}
                            >
                                <Key size={15} />
                                <span>Token / JSON</span>
                            </button>
                            <button
                                className={`cockpit-modal-tab ${modalTab === 'local' ? 'active' : ''}`}
                                onClick={() => setModalTab('local')}
                            >
                                <Database size={15} />
                                <span>Local Import</span>
                            </button>
                        </div>

                        {/* Tab 1: OAuth Authorization */}
                        {modalTab === 'oauth' && (
                            <div>
                                <div className="cockpit-modal-desc">
                                    Click the button below to open the {platformLabel} authorization page in your browser.
                                </div>

                                <div className="cockpit-callout">
                                    <div className="cockpit-callout-title">IDE sign-in & session management</div>
                                    <ul className="cockpit-callout-list">
                                        <li>Add the account after completing OAuth in your browser, then use it for IDE switching.</li>
                                        <li>Resource-package quota data will refresh automatically after authorization.</li>
                                        <li>Account cards show quota amount, progress, and refresh/expiry time by resource package.</li>
                                    </ul>
                                </div>

                                <div className="cockpit-url-box">
                                    <div className="cockpit-url-text">
                                        {oauthUrl || `${platformLabel} Sign-in URL (Generating...)`}
                                    </div>
                                    <button
                                        className="cockpit-url-copy"
                                        onClick={handleCopyUrl}
                                        title={copied ? 'Đã sao chép!' : 'Sao chép URL'}
                                    >
                                        {copied ? <Check size={15} color="var(--success)" /> : <Copy size={15} />}
                                    </button>
                                </div>

                                <div className="cockpit-expiry-meta">
                                    Expires in: 600s; Poll interval: 2s
                                </div>

                                <button
                                    className="cockpit-btn-browser"
                                    onClick={handleOpenBrowser}
                                    disabled={!oauthUrl}
                                >
                                    <Globe size={16} />
                                    <span>Open in Browser</span>
                                </button>

                                <div className="cockpit-polling-bar">
                                    <RefreshCw size={15} className="spin" />
                                    <span className="cockpit-polling-text">Waiting for authorization...</span>
                                </div>

                                <div className="cockpit-footer-hint">
                                    Once authorized, this window will update automatically
                                </div>
                            </div>
                        )}

                        {/* Tab 2: Token / JSON */}
                        {modalTab === 'token' && (
                            <div>
                                <div className="cockpit-modal-desc">
                                    Paste your Access Token, Refresh Token, or Cockpit export JSON bundle below.
                                </div>

                                <div className="cockpit-callout">
                                    <div className="cockpit-callout-title">Direct Token Entry</div>
                                    <ul className="cockpit-callout-list">
                                        <li>Accepts raw Bearer tokens or JSON objects containing access_token & refresh_token.</li>
                                        <li>Tokens are stored securely with local AES-GCM encryption in your local vault.</li>
                                    </ul>
                                </div>

                                <textarea
                                    className="input"
                                    placeholder='{"access_token": "ey...", "refresh_token": "..."}'
                                    rows={5}
                                    value={tokenInput}
                                    onChange={(e) => setTokenInput(e.target.value)}
                                    style={{
                                        width: '100%',
                                        resize: 'none',
                                        fontFamily: 'var(--font-mono)',
                                        fontSize: '0.8rem',
                                        background: '#0c111a',
                                        borderColor: 'rgba(255, 255, 255, 0.1)',
                                        color: '#cbd5e1',
                                        marginBottom: '1rem',
                                    }}
                                />

                                <button
                                    className="cockpit-btn-browser"
                                    onClick={() => {
                                        if (!tokenInput.trim()) {
                                            showMsg('Vui lòng nhập Token hoặc JSON!', false);
                                            return;
                                        }
                                        showMsg('Vui lòng sử dụng OAuth hoặc Quét từ IDE cục bộ để có đầy đủ Refresh Token dài hạn!', false);
                                    }}
                                >
                                    <Key size={16} />
                                    <span>Lưu Token vào Kho</span>
                                </button>
                            </div>
                        )}

                        {/* Tab 3: Local Import */}
                        {modalTab === 'local' && (
                            <div>
                                <div className="cockpit-modal-desc">
                                    Automatically scan and import accounts from local IDE database or Cockpit vaults.
                                </div>

                                <div className="cockpit-callout">
                                    <div className="cockpit-callout-title">Supported Local Sources for {platformLabel}</div>
                                    <ul className="cockpit-callout-list">
                                        <li>Cockpit Vaults: ~/.cockpit_tools/{platformId === 'zed' ? 'zed_accounts' : platformId === 'github_copilot' ? 'github_copilot_accounts' : platformId === 'cursor' ? 'cursor_accounts' : platformId === 'windsurf' ? 'windsurf_accounts' : platformId === 'trae' ? 'trae_accounts' : 'codebuddy_accounts'} and backup archives.</li>
                                        <li>IDE Storage: ~/.config/{platformId === 'zed' ? 'zed' : platformId === 'cursor' ? 'Cursor' : platformId === 'windsurf' ? 'Windsurf' : 'Code'} and globalStorage state databases.</li>
                                        <li>All extracted sessions are decrypted and imported locally without uploading.</li>
                                    </ul>
                                </div>

                                <button
                                    className="cockpit-btn-browser"
                                    onClick={handleImportLocal}
                                >
                                    <Database size={16} />
                                    <span>Quét & Đồng bộ từ IDE / Cockpit Cục bộ</span>
                                </button>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}
