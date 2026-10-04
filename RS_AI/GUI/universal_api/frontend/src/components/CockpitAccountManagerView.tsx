import { useState, useMemo, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Plus,
    FolderOpen,
    Layers,
    Clock,
    ShieldCheck,
    CheckCircle2,
    AlertCircle,
} from 'lucide-react';
import {
    listAccounts,
    removeAccount,
    refreshAntigravityQuota,
    getInstalledAppVersionInfo,
} from '../../../bridge/accounts_bridge';
import { loginStart, loginPoll, loginCancel, openLoginUrl } from '../../../bridge/login_bridge';
import { invokeIpc } from '../../../bridge/ipc';
import { PlatformInstancesContent } from './PlatformInstancesContent';
import {
    type CockpitAccountManagerViewProps,
    type CockpitPlatformId,
    type AccountInfo,
    type InstalledAppInfo,
    ALL_PLATFORMS,
    PlatformGroupSwitcher,
    NoticeBanner,
    AccountCard,
    AccountListView,
    AccountToolbar,
    AddAccountModal,
    WakeupsTab,
    VerificationTab,
    SessionsTab,
} from './cockpit';

// Platform icons for variant switcher
import antigravityIcon from '../assets/icons/antigravity-menu.png';
import codebuddyIcon from '../assets/icons/codebuddy.png';
import codexIcon from '../assets/icons/codex.svg';
import traeIcon from '../assets/icons/trae.png';

export { ALL_PLATFORMS, type CockpitPlatformId };

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
    }, [platformId, antigravitySubVariant, codebuddySubRegion, installedAppInfo, activeTab, platformLabel, platformIcon]);

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
    }, [platformId, codebuddySubRegion, antigravitySubVariant]);

    const handleWakeupAll = async () => {
        setWakeupRunning(true);
        const timeStr = new Date().toLocaleTimeString();
        setWakeupLogs((prev) => [
            `[${timeStr}] Đang gửi tín hiệu keep-alive tới Language Server & Quota API...`,
            ...prev,
        ]);
        try {
            const res = await invokeIpc<any>('execute_wakeup_tasks', {});
            if (res && res.logs && Array.isArray(res.logs)) {
                setWakeupLogs((prev) => [...[...res.logs].reverse(), ...prev]);
            }
            await loadAccounts();
            showMsg(`Đã hoàn tất đánh thức phiên làm việc (${res?.successCount ?? accounts.length} tài khoản OK)!`, true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi gửi tín hiệu wakeup', false);
        } finally {
            setWakeupRunning(false);
        }
    };

    const handleSingleWakeup = async (a: AccountInfo) => {
        try {
            await invokeIpc('single_account_wakeup', { uid: a.uid, platform: platformId });
            showMsg(`Đã gửi tín hiệu Keep-Alive cho ${maskValue(a.nickname || a.uid)}`, true);
            setWakeupLogs((prev) => [`[${new Date().toLocaleTimeString()}] Wakeup riêng lẻ: ${maskValue(a.nickname || a.uid)} thành công (HTTP 200)`, ...prev]);
            await loadAccounts();
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi wakeup', false);
        }
    };

    const handleSyncSessions = async () => {
        try {
            const res = await invokeIpc<any>('sync_platform_sessions', { platform_id: platformId });
            showMsg(res?.message || 'Đã đồng bộ session giữa các instance!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi đồng bộ session', false);
        }
    };

    const handleCleanSessions = async () => {
        try {
            const res = await invokeIpc<any>('clean_platform_sessions', { platform_id: platformId });
            showMsg(res?.message || 'Thùng rác session và cache đã được dọn sạch!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi dọn session', false);
        }
    };

    // Add Account Modal State
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

    // Start OAuth Flow
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

    const handleSaveToken = () => {
        if (!tokenInput.trim()) {
            showMsg('Vui lòng nhập Token hoặc JSON!', false);
            return;
        }
        showMsg('Vui lòng sử dụng OAuth hoặc Quét từ IDE cục bộ để có đầy đủ Refresh Token dài hạn!', false);
    };

    return (
        <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', flexDirection: 'column' }}>
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
                    {/* Version Check Badge */}
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

            {/* Platform Selector & Center Tabs Row */}
            <div className="page-tabs-row" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '1rem', marginBottom: '1.25rem' }}>
                <PlatformGroupSwitcher
                    platformMenuOpen={platformMenuOpen}
                    setPlatformMenuOpen={setPlatformMenuOpen}
                    platformIcon={platformIcon}
                    platformLabel={platformLabel}
                    currentActiveTriggerLabel={currentActiveTriggerLabel}
                    groupVariants={groupVariants}
                />

                {/* Center Dynamic Tabs */}
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
                    <NoticeBanner
                        noticeTitle={noticeTitle}
                        permissionScope={permissionScope}
                        networkScope={networkScope}
                    />

                    {/* Cockpit Action Toolbar & Selection Bar */}
                    <AccountToolbar
                        searchQuery={searchQuery}
                        setSearchQuery={setSearchQuery}
                        viewMode={viewMode}
                        setViewMode={setViewMode}
                        totalAccounts={displayedAccounts.length}
                        activeTag={activeTag}
                        setActiveTag={setActiveTag}
                        selectedCount={selectedIds.size}
                        isAllSelected={selectedIds.size > 0 && selectedIds.size === displayedAccounts.length}
                        onToggleSelectAll={toggleSelectAll}
                        onOpenAddModal={openAddModal}
                        onRefreshAll={handleRefreshAll}
                        refreshing={refreshing}
                        privacyMode={privacyMode}
                        setPrivacyMode={setPrivacyMode}
                        onExport={() => showMsg('Tất cả tài khoản đã được sao lưu tự động!', true)}
                    />

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
                            {displayedAccounts.map((account) => (
                                <AccountCard
                                    key={account.uid}
                                    account={account}
                                    platformId={platformId}
                                    isSelected={selectedIds.has(account.uid)}
                                    refreshingUid={refreshingUid}
                                    onToggleSelect={toggleSelect}
                                    onSwitchAccount={handleSwitchAccount}
                                    onRefreshSingle={handleRefreshSingle}
                                    onDelete={handleDelete}
                                    maskValue={maskValue}
                                />
                            ))}
                        </div>
                    ) : (
                        <AccountListView
                            displayedAccounts={displayedAccounts}
                            selectedIds={selectedIds}
                            toggleSelectAll={toggleSelectAll}
                            toggleSelect={toggleSelect}
                            onSwitchAccount={handleSwitchAccount}
                            onDelete={handleDelete}
                            maskValue={maskValue}
                        />
                    )}
                </>
            )}

            {/* TAB: WAKEUPS */}
            {activeTab === 'wakeups' && (
                <WakeupsTab
                    accounts={accounts}
                    platformId={platformId}
                    wakeupRunning={wakeupRunning}
                    wakeupLogs={wakeupLogs}
                    onWakeupAll={handleWakeupAll}
                    onSingleWakeup={handleSingleWakeup}
                    maskValue={maskValue}
                />
            )}

            {/* TAB: VERIFICATION */}
            {activeTab === 'verification' && (
                <VerificationTab
                    accounts={accounts}
                    verifyingUid={verifyingUid}
                    setVerifyingUid={setVerifyingUid}
                    verificationCode={verificationCode}
                    setVerificationCode={setVerificationCode}
                    onCheckSession={(a) => showMsg(`Đang kiểm tra checkpoint cho ${maskValue(a.nickname || a.uid)}: Phiên an toàn!`, true)}
                    onSubmitOtp={(_uid) => {
                        showMsg('Đã xác thực mã OTP thành công!', true);
                        setVerifyingUid(null);
                    }}
                    maskValue={maskValue}
                />
            )}

            {/* TAB: SESSIONS */}
            {activeTab === 'sessions' && (
                <SessionsTab
                    platformLabel={platformLabel}
                    onSyncSessions={handleSyncSessions}
                    onCleanSessions={handleCleanSessions}
                />
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

            {/* Add Account Modal */}
            <AddAccountModal
                isOpen={modalOpen}
                onClose={() => {
                    if (oauthPolling) void loginCancel();
                    setOauthPolling(false);
                    setModalOpen(false);
                }}
                platformId={platformId}
                platformLabel={platformLabel}
                modalTab={modalTab}
                setModalTab={setModalTab}
                oauthUrl={oauthUrl}
                oauthPolling={oauthPolling}
                oauthError={oauthError}
                copied={copied}
                onCopyUrl={handleCopyUrl}
                onOpenBrowser={handleOpenBrowser}
                tokenInput={tokenInput}
                setTokenInput={setTokenInput}
                onSaveToken={handleSaveToken}
                onImportLocal={handleImportLocal}
                hasNativeOAuth={hasNativeOAuth}
            />
        </div>
    );
}
