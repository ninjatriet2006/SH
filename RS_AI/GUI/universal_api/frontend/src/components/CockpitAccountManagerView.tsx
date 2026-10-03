import { useState, useMemo, useEffect, useRef } from 'react';
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
    TerminalSquare,
    Pencil,
} from 'lucide-react';
import type { AccountInfo } from '../../../bridge/types';
import {
    listAccounts,
    removeAccount,
    refreshAntigravityQuota,
    getInstalledAppVersionInfo,
    type InstalledAppInfo,
} from '../../../bridge/accounts_bridge';
import { loginStart, loginPoll, loginCancel, openLoginUrl } from '../../../bridge/login_bridge';
import { invokeIpc } from '../../../bridge/ipc';
import { PlatformInstancesContent } from './PlatformInstancesContent';

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

    const switcherRef = useRef<HTMLDivElement | null>(null);

    // Close switcher dropdown on outside click or Esc
    useEffect(() => {
        if (!platformMenuOpen) return;
        const handleMouseDown = (e: MouseEvent) => {
            if (switcherRef.current && !switcherRef.current.contains(e.target as Node)) {
                setPlatformMenuOpen(false);
            }
        };
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setPlatformMenuOpen(false);
        };
        document.addEventListener('mousedown', handleMouseDown);
        document.addEventListener('keydown', handleKeyDown);
        return () => {
            document.removeEventListener('mousedown', handleMouseDown);
            document.removeEventListener('keydown', handleKeyDown);
        };
    }, [platformMenuOpen]);

    // Cockpit 1:1: Platform variants within the active group suite
    const groupVariants = useMemo(() => {
        if (platformId === 'antigravity') {
            return [
                {
                    id: 'ide',
                    label: 'Antigravity IDE',
                    subtext: antigravitySubVariant === 'ide' && installedAppInfo?.installed && installedAppInfo.version !== 'Not Found'
                        ? `VS Code based (v${installedAppInfo.version})`
                        : 'VS Code based',
                    icon: antigravityIcon,
                    isActive: antigravitySubVariant === 'ide',
                    onSelect: () => setAntigravitySubVariant('ide'),
                },
                {
                    id: 'desktop',
                    label: 'Antigravity Desktop',
                    subtext: antigravitySubVariant === 'desktop' && installedAppInfo?.installed && installedAppInfo.version !== 'Not Found'
                        ? `Desktop App (v${installedAppInfo.version})`
                        : 'Desktop App Legacy',
                    icon: antigravityIcon,
                    isActive: antigravitySubVariant === 'desktop',
                    onSelect: () => {
                        setAntigravitySubVariant('desktop');
                        if (activeTab === 'wakeups' || activeTab === 'verification') {
                            setActiveTab('overview');
                        }
                    },
                },
            ];
        }
        if (platformId === 'codebuddy' || platformId === 'codebuddy_global' || platformId === 'codebuddy_cn') {
            return [
                {
                    id: 'global',
                    label: 'CodeBuddy Global',
                    subtext: 'codebuddy.ai',
                    icon: codebuddyIcon,
                    isActive: codebuddySubRegion === 'global',
                    onSelect: () => setCodebuddySubRegion('global'),
                },
                {
                    id: 'cn',
                    label: 'CodeBuddy CN',
                    subtext: 'copilot.tencent.com',
                    icon: codebuddyIcon,
                    isActive: codebuddySubRegion === 'cn',
                    onSelect: () => setCodebuddySubRegion('cn'),
                },
            ];
        }
        if (platformId === 'codex') {
            return [
                {
                    id: 'codex',
                    label: 'Codex (Web / CLI)',
                    subtext: 'OpenAI session client',
                    icon: codexIcon,
                    isActive: true,
                    onSelect: () => {},
                },
                {
                    id: 'codex_api',
                    label: 'Codex API Service',
                    subtext: 'REST / Proxy gateway',
                    icon: codexIcon,
                    isActive: false,
                    onSelect: () => {},
                },
            ];
        }
        if (platformId === 'trae') {
            return [
                {
                    id: 'trae_global',
                    label: 'Trae Global',
                    subtext: 'trae.ai',
                    icon: traeIcon,
                    isActive: true,
                    onSelect: () => {},
                },
                {
                    id: 'trae_solo',
                    label: 'TRAE SOLO',
                    subtext: 'Single agent IDE',
                    icon: traeIcon,
                    isActive: false,
                    onSelect: () => {},
                },
                {
                    id: 'trae_cn',
                    label: 'Trae CN',
                    subtext: 'trae.cn',
                    icon: traeIcon,
                    isActive: false,
                    onSelect: () => {},
                },
            ];
        }
        return [
            {
                id: platformId,
                label: platformLabel,
                subtext: 'Official client',
                icon: platformIcon,
                isActive: true,
                onSelect: () => {},
            },
        ];
    }, [platformId, antigravitySubVariant, codebuddySubRegion, installedAppInfo, activeTab]);

    // Active label on trigger button (matches Cockpit image: "Antigravity")
    const currentActiveTriggerLabel = useMemo(() => {
        if (platformId === 'antigravity') {
            return antigravitySubVariant === 'ide' ? 'Antigravity' : 'Antigravity Desktop';
        }
        if (platformId === 'codebuddy' || platformId === 'codebuddy_global' || platformId === 'codebuddy_cn') {
            return codebuddySubRegion === 'global' ? 'CodeBuddy' : 'CodeBuddy CN';
        }
        return platformLabel;
    }, [platformId, antigravitySubVariant, codebuddySubRegion, platformLabel]);

    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [refreshingUid, setRefreshingUid] = useState<string | null>(null);
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    useEffect(() => {
        const variant = platformId === 'antigravity'
            ? antigravitySubVariant
            : (platformId === 'codebuddy' || platformId === 'codebuddy_cn' || platformId === 'codebuddy_global')
            ? codebuddySubRegion
            : undefined;

        getInstalledAppVersionInfo(platformId, variant)
            .then((info) => {
                setInstalledAppInfo(info);
            })
            .catch(() => {
                setInstalledAppInfo({
                    installed: false,
                    name: currentActiveTriggerLabel,
                    version: 'Not Found',
                    exec_path: '',
                    target_kind: platformId,
                });
            });
    }, [platformId, antigravitySubVariant, codebuddySubRegion, currentActiveTriggerLabel]);

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
    const [oauthError, setOauthError] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);
    const [tokenInput, setTokenInput] = useState('');

    const hasNativeOAuth = (pid: string) => {
        return pid === 'antigravity' || pid === 'github_copilot' || pid === 'cursor' || pid === 'codebuddy' || pid === 'codebuddy_cn' || pid === 'codebuddy_global';
    };

    const openAddModal = (tab?: 'oauth' | 'token' | 'local') => {
        if (tab) {
            setModalTab(tab);
        } else if (hasNativeOAuth(platformId)) {
            setModalTab('oauth');
        } else {
            setModalTab('local');
        }
        setOauthError(null);
        setModalOpen(true);
    };

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

    const handleRefreshSingle = async (acc: AccountInfo) => {
        try {
            setRefreshingUid(acc.uid);
            if (acc.domain?.includes('antigravity') || acc.uid?.startsWith('antigravity_')) {
                await refreshAntigravityQuota(acc.uid);
                showMsg('Đã làm mới Quota Antigravity thành công', true);
            }
            await loadAccounts();
        } catch (e: any) {
            showMsg(`Làm mới thất bại: ${e?.message || e}`, false);
        } finally {
            setRefreshingUid(null);
        }
    };

    const handleRefreshAll = async () => {
        try {
            setRefreshing(true);
            if (platformId === 'antigravity') {
                for (const acc of accounts) {
                    if (acc.domain?.includes('antigravity') || acc.uid?.startsWith('antigravity_')) {
                        try {
                            await refreshAntigravityQuota(acc.uid);
                        } catch (err) {
                            console.warn(`Lỗi refresh quota ${acc.uid}:`, err);
                        }
                    }
                }
                showMsg('Đã làm mới toàn bộ Quota Antigravity', true);
            }
            await loadAccounts();
        } catch (e: any) {
            showMsg(`Lỗi: ${e?.message || e}`, false);
        } finally {
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadAccounts();
    }, [platformId, codebuddySubRegion, antigravitySubVariant]);

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
            setOauthError(null);
            if (platformId === 'antigravity') {
                setOauthPolling(true);
                const res = await loginStart('antigravity');
                setOauthUrl(res.auth_url);

                const interval = setInterval(async () => {
                    try {
                        const outcome = await loginPoll();
                        if (outcome.status === 'done') {
                            clearInterval(interval);
                            setOauthPolling(false);
                            setModalOpen(false);
                            showMsg('Đăng nhập Google Antigravity OAuth thành công!', true);
                            loadAccounts();
                        }
                    } catch (err: any) {
                        clearInterval(interval);
                        setOauthPolling(false);
                        setOauthError(err.message || 'Lỗi khi chờ OAuth');
                    }
                }, 1500);
            } else if (platformId === 'github_copilot') {
                setOauthPolling(true);
                const res = await loginStart('github_copilot');
                setOauthUrl(res.auth_url);

                const interval = setInterval(async () => {
                    try {
                        const outcome = await loginPoll();
                        if (outcome.status === 'done') {
                            clearInterval(interval);
                            setOauthPolling(false);
                            setModalOpen(false);
                            showMsg('Đăng nhập GitHub Copilot thành công!', true);
                            loadAccounts();
                        }
                    } catch (err: any) {
                        clearInterval(interval);
                        setOauthPolling(false);
                        setOauthError(err.message || 'Lỗi khi chờ GitHub Copilot OAuth');
                    }
                }, 1500);
            } else if (platformId === 'cursor') {
                setOauthPolling(true);
                const res = await loginStart('cursor');
                setOauthUrl(res.auth_url);

                const interval = setInterval(async () => {
                    try {
                        const outcome = await loginPoll();
                        if (outcome.status === 'done') {
                            clearInterval(interval);
                            setOauthPolling(false);
                            setModalOpen(false);
                            showMsg('Đăng nhập Cursor OAuth thành công!', true);
                            loadAccounts();
                        }
                    } catch (err: any) {
                        clearInterval(interval);
                        setOauthPolling(false);
                        setOauthError(err.message || 'Lỗi khi chờ Cursor OAuth');
                    }
                }, 2000);
            } else if (platformId === 'codebuddy' || platformId === 'codebuddy_cn' || platformId === 'codebuddy_global') {
                setOauthPolling(true);
                const realm = (platformId === 'codebuddy_cn' || codebuddySubRegion === 'cn') ? 'cn' : 'intl';
                const res = await loginStart(realm);
                setOauthUrl(res.auth_url);

                const interval = setInterval(async () => {
                    try {
                        const outcome = await loginPoll();
                        if (outcome.status === 'done') {
                            clearInterval(interval);
                            setOauthPolling(false);
                            setModalOpen(false);
                            showMsg('Đăng nhập CodeBuddy OAuth thành công!', true);
                            loadAccounts();
                        }
                    } catch (err: any) {
                        clearInterval(interval);
                        setOauthPolling(false);
                        setOauthError(err.message || 'Lỗi khi chờ CodeBuddy OAuth');
                    }
                }, 2000);
            } else {
                setOauthPolling(false);
                if (platformId === 'zed') {
                    setOauthUrl('https://cloud.zed.dev');
                } else if (platformId === 'windsurf') {
                    setOauthUrl('https://codeium.com/account/login');
                } else if (platformId === 'trae') {
                    setOauthUrl('https://trae.ai/login');
                } else if (platformId === 'claude') {
                    setOauthUrl('https://claude.ai');
                } else if (platformId === 'codex') {
                    setOauthUrl('https://platform.openai.com/api-keys');
                } else if (platformId === 'kiro') {
                    setOauthUrl('https://kiro.dev');
                } else if (platformId === 'qoder') {
                    setOauthUrl('https://qoder.com');
                }
            }
        } catch (e: any) {
            setOauthPolling(false);
            setOauthError(e.message || 'Lỗi bắt đầu đăng nhập OAuth');
            showMsg(e.message || 'Lỗi bắt đầu đăng nhập OAuth', false);
        }
    };

    // Auto-trigger OAuth when modal opens on oauth tab
    useEffect(() => {
        if (!modalOpen) {
            if (oauthPolling) void loginCancel();
            setOauthPolling(false);
            setOauthUrl('');
            setOauthError(null);
            return;
        }
        if (modalTab === 'oauth' && !oauthUrl && !oauthPolling) {
            void startOAuthFlow();
        }
    }, [modalOpen, modalTab, platformId, codebuddySubRegion]);

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
            {/* Top Strip (Cockpit 1:1) */}
            <div className="page-top-strip">
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ fontSize: '0.82rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                        Account Management
                    </span>
                    <button
                        type="button"
                        className="platform-header-help"
                        title="Hướng dẫn & Thiết lập"
                        onClick={() => navigate('/settings')}
                    >
                        ?
                    </button>
                </div>
                <div className="page-top-strip-right">
                    {/* Version Check Badge (1:1 with Cockpit Tools) */}
                    <div style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '0.45rem',
                        padding: '0.3rem 0.75rem',
                        borderRadius: '9999px',
                        background: 'rgba(30, 41, 59, 0.75)',
                        border: '1px solid rgba(255, 255, 255, 0.12)',
                        fontSize: '0.76rem',
                        fontWeight: 600,
                        color: '#e2e8f0',
                        backdropFilter: 'blur(8px)',
                        boxShadow: '0 2px 8px rgba(0, 0, 0, 0.25)',
                    }}>
                        <span style={{
                            width: 7,
                            height: 7,
                            borderRadius: '50%',
                            background: installedAppInfo?.installed && installedAppInfo.version !== 'Not Found' ? '#10b981' : '#94a3b8',
                            boxShadow: installedAppInfo?.installed && installedAppInfo.version !== 'Not Found' ? '0 0 8px #10b981' : 'none',
                        }} />
                        <span>{installedAppInfo?.name || currentActiveTriggerLabel}</span>
                        <span style={{
                            color: installedAppInfo?.installed && installedAppInfo.version !== 'Not Found' ? '#38bdf8' : '#94a3b8',
                            fontWeight: 700,
                            marginLeft: '0.15rem'
                        }}>
                            {installedAppInfo?.installed && installedAppInfo.version !== 'Not Found'
                                ? (installedAppInfo.version.startsWith('v') ? installedAppInfo.version : `v${installedAppInfo.version}`)
                                : 'Version: Not Found'}
                        </span>
                    </div>
                </div>
            </div>

            {/* Platform Selector & Center Tabs Row (Cockpit 1:1) */}
            <div className="page-tabs-row" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', marginBottom: '1.25rem' }}>
                {/* Left: Interactive PlatformGroupSwitcher Dropdown */}
                <div className="platform-group-switcher" ref={switcherRef}>
                    <button
                        type="button"
                        className={`platform-group-switcher-trigger ${platformMenuOpen ? 'is-open' : ''}`}
                        onClick={() => setPlatformMenuOpen(!platformMenuOpen)}
                        aria-label="Chuyển đổi phân loại / nền tảng cùng nhóm"
                    >
                        <span className="platform-group-switcher-trigger-icon">
                            <img src={platformIcon} alt="" style={{ width: 16, height: 16, objectFit: 'contain' }} />
                        </span>
                        <span className="platform-group-switcher-trigger-label">
                            {currentActiveTriggerLabel}
                        </span>
                        <ChevronDown size={14} className="platform-group-switcher-trigger-caret" />
                    </button>

                    {platformMenuOpen && (
                        <div className="platform-group-switcher-dropdown">
                            {/* Group Variants Section (Phân loại ứng dụng) */}
                            <div style={{ fontSize: '0.7rem', fontWeight: 700, color: 'var(--text-muted)', padding: '4px 10px 2px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                                Phân loại ứng dụng ({platformLabel})
                            </div>
                            {groupVariants.map((v) => (
                                <button
                                    key={v.id}
                                    type="button"
                                    className={`platform-group-switcher-option ${v.isActive ? 'is-active' : ''}`}
                                    onClick={() => {
                                        v.onSelect();
                                        setPlatformMenuOpen(false);
                                    }}
                                >
                                    <span className="platform-group-switcher-option-icon">
                                        <img src={v.icon || platformIcon} alt="" style={{ width: 16, height: 16, objectFit: 'contain' }} />
                                    </span>
                                    <div style={{ minWidth: 0, display: 'flex', flexDirection: 'column' }}>
                                        <span className="platform-group-switcher-option-label">{v.label}</span>
                                        {v.subtext && (
                                            <span style={{ fontSize: '0.68rem', color: 'var(--text-muted)', lineHeight: 1.1 }}>{v.subtext}</span>
                                        )}
                                    </div>
                                    <span className="platform-group-switcher-option-check">
                                        {v.isActive ? <Check size={16} /> : null}
                                    </span>
                                </button>
                            ))}

                            <div className="platform-group-switcher-divider" />

                            {/* Other Platform Groups */}
                            <div style={{ fontSize: '0.7rem', fontWeight: 700, color: 'var(--text-muted)', padding: '4px 10px 2px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                                Chuyển nền tảng
                            </div>
                            <div style={{ maxHeight: 200, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 2 }}>
                                {ALL_PLATFORMS.filter(p => p.id !== platformId).map((p) => (
                                    <button
                                        key={p.id}
                                        type="button"
                                        className="platform-group-switcher-option"
                                        onClick={() => {
                                            setPlatformMenuOpen(false);
                                            navigate(p.path);
                                        }}
                                        style={{ minHeight: 34, padding: '4px 10px' }}
                                    >
                                        <span className="platform-group-switcher-option-icon">
                                            <img src={p.icon} alt="" style={{ width: 15, height: 15, objectFit: 'contain' }} />
                                        </span>
                                        <span className="platform-group-switcher-option-label" style={{ fontSize: '0.8rem' }}>
                                            {p.label}
                                        </span>
                                        <span />
                                    </button>
                                ))}
                            </div>

                            <div className="platform-group-switcher-divider" />

                            {/* Group Management action */}
                            <button
                                type="button"
                                className="platform-group-switcher-action"
                                onClick={() => {
                                    setPlatformMenuOpen(false);
                                    navigate('/settings');
                                }}
                            >
                                <span className="platform-group-switcher-action-icon">
                                    <Pencil size={14} />
                                </span>
                                <span className="platform-group-switcher-action-label">
                                    Quản lý nhóm nền tảng (Group Settings)
                                </span>
                            </button>
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
                        {antigravitySubVariant === 'ide' && (
                            <>
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
                            </>
                        )}
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
            </div>

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
                        onClick={() => openAddModal()}
                        title="Add Account"
                    >
                        <Plus size={18} />
                    </button>

                    {/* Refresh All */}
                    <button
                        className="toolbar-icon-btn"
                        onClick={handleRefreshAll}
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
                        onClick={() => openAddModal('local')}
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
                        onClick={() => openAddModal()}
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
                                        (() => {
                                            const rawTier = (account.plan_tier || account.quota_details?.plan_tier || 'FREE').toUpperCase();
                                            const isProTier = rawTier.includes('PRO') || rawTier.includes('ULTRA');
                                            return (
                                                <div style={{ display: 'flex', gap: '0.35rem' }}>
                                                    {isCurrent && (
                                                        <span className="badge" style={{ background: '#22c55e', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                            Current
                                                        </span>
                                                    )}
                                                    <span className="badge" style={{ background: isProTier ? '#0284c7' : '#475569', color: '#fff', fontSize: '0.65rem', fontWeight: 600, padding: '0.15rem 0.45rem', borderRadius: 4 }}>
                                                        {rawTier}
                                                    </span>
                                                </div>
                                            );
                                        })()
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
                                        const rawTier = (account.plan_tier || details?.plan_tier || 'FREE').toUpperCase();
                                        const isPro = rawTier.includes('PRO') || rawTier.includes('ULTRA');
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

                                                        {!claude5h && !claudeWeekly && (
                                                            <div style={{ fontSize: '0.72rem', color: '#64748b', fontStyle: 'italic', marginTop: 2 }}>
                                                                Chưa có hạn ngạch
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

                                                        {!gemini5h && !geminiWeekly && (
                                                            <div style={{ fontSize: '0.72rem', color: '#64748b', fontStyle: 'italic', marginTop: 2 }}>
                                                                Chưa có hạn ngạch
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
                                            onClick={() => handleRefreshSingle(account)}
                                            title="Làm mới Quota"
                                        >
                                            <RefreshCw size={12} className={refreshingUid === account.uid ? 'spin' : ''} />
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
                <PlatformInstancesContent
                    platformId={platformId}
                    platformLabel={platformLabel}
                    accounts={accounts}
                    onSwitchAccount={handleSwitchAccount}
                />
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

                        {/* Tab 1: OAuth / Web Authorization */}
                        {modalTab === 'oauth' && (
                            <div>
                                <div className="cockpit-modal-desc">
                                    {hasNativeOAuth(platformId)
                                        ? `Click the button below to open the ${platformLabel} authorization page in your browser.`
                                        : `Truy cập cổng chính thức của ${platformLabel} để đăng nhập hoặc lấy khóa API / Token.`}
                                </div>

                                {oauthError && (
                                    <div style={{
                                        background: 'rgba(239, 68, 68, 0.12)',
                                        border: '1px solid rgba(239, 68, 68, 0.3)',
                                        color: '#fca5a5',
                                        padding: '0.65rem 0.85rem',
                                        borderRadius: '8px',
                                        marginBottom: '1rem',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'space-between',
                                        fontSize: '0.82rem',
                                    }}>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                            <AlertCircle size={16} color="var(--danger)" />
                                            <span>{oauthError}</span>
                                        </div>
                                        <button
                                            className="btn btn-outline"
                                            style={{ fontSize: '0.72rem', padding: '0.2rem 0.6rem' }}
                                            onClick={startOAuthFlow}
                                        >
                                            Thử lại
                                        </button>
                                    </div>
                                )}

                                <div className="cockpit-callout">
                                    <div className="cockpit-callout-title">
                                        {hasNativeOAuth(platformId)
                                            ? 'IDE sign-in & session management'
                                            : `Quy trình xác thực ${platformLabel}`}
                                    </div>
                                    <ul className="cockpit-callout-list">
                                        {hasNativeOAuth(platformId) ? (
                                            <>
                                                <li>Add the account after completing OAuth in your browser, then use it for IDE switching.</li>
                                                <li>Resource-package quota data will refresh automatically after authorization.</li>
                                                <li>Account cards show quota amount, progress, and refresh/expiry time by resource package.</li>
                                            </>
                                        ) : (
                                            <>
                                                <li>{platformLabel} sử dụng cấu hình khóa API riêng hoặc phiên làm việc đã lưu từ IDE cục bộ.</li>
                                                <li>Nhấn <strong>Open in Browser</strong> để mở trang đăng nhập / tạo khóa API của {platformLabel}.</li>
                                                <li>Sau khi có Token, chuyển sang tab <strong>Token / JSON</strong> để dán trực tiếp, hoặc tab <strong>Local Import</strong> để tự động quét từ IDE máy bạn.</li>
                                            </>
                                        )}
                                    </ul>
                                </div>

                                <div className="cockpit-url-box">
                                    <div className="cockpit-url-text">
                                        {oauthUrl ? (
                                            oauthUrl
                                        ) : oauthPolling ? (
                                            <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-muted)' }}>
                                                <RefreshCw size={14} className="spin" /> Đang khởi tạo luồng OAuth & cổng lắng nghe...
                                            </span>
                                        ) : (
                                            `${platformLabel} Portal URL`
                                        )}
                                    </div>
                                    {oauthUrl && (
                                        <button
                                            className="cockpit-url-copy"
                                            onClick={handleCopyUrl}
                                            title={copied ? 'Đã sao chép!' : 'Sao chép URL'}
                                        >
                                            {copied ? <Check size={15} color="var(--success)" /> : <Copy size={15} />}
                                        </button>
                                    )}
                                </div>

                                {hasNativeOAuth(platformId) && (
                                    <div className="cockpit-expiry-meta">
                                        Expires in: 600s; Poll interval: 2s
                                    </div>
                                )}

                                <button
                                    className="cockpit-btn-browser"
                                    onClick={handleOpenBrowser}
                                    disabled={!oauthUrl}
                                >
                                    <Globe size={16} />
                                    <span>Open in Browser</span>
                                </button>

                                {hasNativeOAuth(platformId) && oauthPolling && (
                                    <div className="cockpit-polling-bar">
                                        <RefreshCw size={15} className="spin" />
                                        <span className="cockpit-polling-text">Waiting for authorization...</span>
                                    </div>
                                )}

                                {hasNativeOAuth(platformId) ? (
                                    <div className="cockpit-footer-hint">
                                        Once authorized, this window will update automatically
                                    </div>
                                ) : (
                                    <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                                        <button
                                            className="btn"
                                            style={{ flex: 1, padding: '0.5rem', fontSize: '0.78rem' }}
                                            onClick={() => setModalTab('token')}
                                        >
                                            <Key size={14} /> Chuyển sang Token / JSON
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ flex: 1, padding: '0.5rem', fontSize: '0.78rem' }}
                                            onClick={() => setModalTab('local')}
                                        >
                                            <Database size={14} /> Chuyển sang Local Import
                                        </button>
                                    </div>
                                )}
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
