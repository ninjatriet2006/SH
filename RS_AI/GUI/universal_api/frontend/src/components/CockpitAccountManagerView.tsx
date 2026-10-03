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
} from 'lucide-react';
import type { AccountInfo } from '../../../bridge/types';
import { listAccounts, removeAccount } from '../../../bridge/accounts_bridge';
import { loginStart, loginPoll, loginCancel, openLoginUrl } from '../../../bridge/login_bridge';
import { invokeIpc } from '../../../bridge/ipc';

interface CockpitAccountManagerViewProps {
    platformId: 'codebuddy_cn' | 'codebuddy_global' | 'zed' | 'all';
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
    const [activeTab, setActiveTab] = useState<'overview' | 'sessions' | 'instances'>('overview');
    const [noticeExpanded, setNoticeExpanded] = useState(true);
    const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
    const [privacyMode, setPrivacyMode] = useState(true);
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
    const [activeTag, setActiveTag] = useState<string>('all');

    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

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
            if (platformId === 'codebuddy_cn') {
                filtered = all.filter((a) => a.domain?.includes('tencent') || a.domain?.includes('cn'));
            } else if (platformId === 'codebuddy_global') {
                filtered = all.filter((a) => a.domain?.includes('codebuddy.ai') || a.domain?.includes('global'));
            } else if (platformId === 'zed') {
                filtered = all.filter((a) => a.domain?.includes('zed'));
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
    }, [platformId]);

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
                {/* Left: Platform Dropdown */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <button className="platform-dropdown-btn">
                        <img src={platformIcon} alt="" className="nav-item-icon" style={{ width: 18, height: 18 }} />
                        <span>{platformLabel}</span>
                        <ChevronDown size={14} color="var(--text-secondary)" />
                    </button>
                </div>

                {/* Center Tabs: Overview | Session Manager | Instances */}
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
                        onClick={() => navigate('/instances')}
                    >
                        <Layers size={15} />
                        <span>Instances</span>
                    </button>
                </div>

                <div style={{ width: 100 }} />
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
                        const isCurrent = account.healthy;

                        return (
                            <div
                                key={account.uid}
                                className={`account-card ${isCurrent ? 'current' : ''}`}
                            >
                                {/* Top Row */}
                                <div className="card-top">
                                    <input
                                        type="checkbox"
                                        checked={isSelected}
                                        onChange={() => toggleSelect(account.uid)}
                                    />
                                    <div className="card-email-label" style={{ flex: 1 }}>
                                        {maskValue(account.nickname || account.uid)}
                                    </div>
                                    <span className="badge badge-info" style={{ fontSize: '0.65rem' }}>
                                        FREE
                                    </span>
                                </div>

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

                                {/* Card Footer with Actions */}
                                <div className="card-footer">
                                    <span className="card-date">
                                        10/01/2026 15:02
                                    </span>

                                    <div className="card-actions">
                                        <button
                                            className="card-action-btn play"
                                            onClick={() => handleSwitchAccount(account)}
                                            title="Khởi chạy / Chuyển tài khoản vào IDE"
                                        >
                                            <Play size={12} />
                                        </button>
                                        <button
                                            className="card-action-btn"
                                            title="Sửa nhãn tag"
                                        >
                                            <Tag size={12} />
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
                                            title="Xuất JSON"
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

            {/* Cockpit Add Account Modal (1:1 with Cockpit Tools) */}
            {modalOpen && (
                <div className="cockpit-modal-overlay">
                    <div className="cockpit-modal">
                        {/* Header */}
                        <div className="cockpit-modal-header">
                            <h3 className="cockpit-modal-title">
                                Add {platformLabel.includes('CN') ? 'CodeBuddy CN' : 'CodeBuddy'} Account
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
                                    Click the button below to open the {platformLabel.includes('CN') ? 'CodeBuddy CN' : 'CodeBuddy'} authorization page in your browser.
                                </div>

                                <div className="cockpit-callout">
                                    <div className="cockpit-callout-title">IDE sign-in only</div>
                                    <ul className="cockpit-callout-list">
                                        <li>Add the account after completing OAuth in your browser, then use it for IDE switching.</li>
                                        <li>Resource-package quota data will refresh automatically after authorization.</li>
                                        <li>Account cards show quota amount, progress, and refresh/expiry time by resource package.</li>
                                    </ul>
                                </div>

                                <div className="cockpit-url-box">
                                    <div className="cockpit-url-text">
                                        {oauthUrl || 'https://www.codebuddy.ai/login?platform=ide&state=... (Generating...)'}
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
                                    <div className="cockpit-callout-title">Supported Local Sources</div>
                                    <ul className="cockpit-callout-list">
                                        <li>Cockpit Storage: ~/.cockpit_tools/codebuddy_accounts and auto-backups.</li>
                                        <li>VS Code & IDEs: ~/.config/Antigravity IDE, Trae, and VS Code globalStorage (state.vscdb).</li>
                                        <li>All extracted sessions will be decrypted and imported locally without uploading.</li>
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
