import { useEffect, useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Play,
    Square,
    Folder,
    Layers,
    Activity,
    Search,
    RefreshCw,
    ExternalLink,
    AlertCircle,
    CheckCircle2,
    HardDrive,
    Info,
    ArrowRight,
} from 'lucide-react';
import {
    listProfiles,
    launchPlatformInstance,
    stopPlatformInstance,
    openInstanceFolder,
    type InstanceProfile,
} from '../../../bridge/profiles_bridge';
import { listAccounts } from '../../../bridge/accounts_bridge';
import type { AccountInfo } from '../../../bridge/types';

// Platform metadata mapping
const PLATFORMS_MAP: Record<string, { label: string; path: string; color: string }> = {
    antigravity: { label: 'Antigravity', path: '/platforms/antigravity', color: '#38bdf8' },
    antigravity_ide: { label: 'Antigravity IDE', path: '/platforms/antigravity', color: '#38bdf8' },
    antigravity_desktop: { label: 'Antigravity Desktop', path: '/platforms/antigravity', color: '#38bdf8' },
    codebuddy: { label: 'CodeBuddy', path: '/platforms/codebuddy', color: '#a855f7' },
    codebuddy_cn: { label: 'CodeBuddy CN', path: '/platforms/codebuddy', color: '#a855f7' },
    codebuddy_global: { label: 'CodeBuddy Global', path: '/platforms/codebuddy', color: '#a855f7' },
    zed: { label: 'Zed Cloud', path: '/platforms/zed', color: '#10b981' },
    github_copilot: { label: 'GitHub Copilot', path: '/platforms/github-copilot', color: '#64748b' },
    vscode: { label: 'VS Code', path: '/platforms/github-copilot', color: '#0ea5e9' },
    cursor: { label: 'Cursor', path: '/platforms/cursor', color: '#ec4899' },
    windsurf: { label: 'Windsurf', path: '/platforms/windsurf', color: '#06b6d4' },
    trae: { label: 'Trae', path: '/platforms/trae', color: '#f59e0b' },
    claude: { label: 'Claude', path: '/platforms/claude', color: '#d97706' },
    codex: { label: 'Codex', path: '/platforms/codex', color: '#10b981' },
    kiro: { label: 'Kiro', path: '/platforms/kiro', color: '#8b5cf6' },
    qoder: { label: 'Qoder', path: '/platforms/qoder', color: '#3b82f6' },
};

export function InstancesPage() {
    const navigate = useNavigate();
    const [instances, setInstances] = useState<InstanceProfile[]>([]);
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [platformFilter, setPlatformFilter] = useState('all');
    const [statusFilter, setStatusFilter] = useState<'all' | 'running' | 'stopped'>('all');
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    const showMsg = (text: string, ok: boolean) => {
        setActionMsg({ text, ok });
        setTimeout(() => setActionMsg(null), 4000);
    };

    const loadData = async (isManual = false) => {
        if (isManual) setRefreshing(true);
        else setLoading(true);
        try {
            const [instList, accList] = await Promise.all([
                listProfiles(),
                listAccounts().catch(() => [] as AccountInfo[]),
            ]);
            setInstances(instList);
            setAccounts(accList);
            if (isManual) showMsg('Đã làm mới danh sách instances toàn hệ thống!', true);
        } catch (err: any) {
            console.error('Failed to load global instances:', err);
            showMsg(err.message || 'Lỗi khi tải danh sách instances', false);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadData();
    }, []);

    // Handle Launch
    const handleLaunch = async (inst: InstanceProfile) => {
        try {
            const res = await launchPlatformInstance(inst.platform_id, inst.id);
            showMsg(`Đã khởi chạy "${inst.name}" [${inst.platform_id}] (PID: ${res.pid})`, true);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi khởi chạy', false);
        }
    };

    // Handle Stop
    const handleStop = async (inst: InstanceProfile) => {
        try {
            await stopPlatformInstance(inst.platform_id, inst.id, inst.last_pid || inst.lastPid || undefined);
            showMsg(`Đã dừng tiến trình "${inst.name}" [${inst.platform_id}]`, true);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi dừng', false);
        }
    };

    // Handle Open Folder
    const handleOpenFolder = async (inst: InstanceProfile) => {
        const dir = inst.user_data_dir || inst.userDataDir;
        if (!dir || dir === 'default') {
            showMsg('Default Instance sử dụng thư mục cấu hình mặc định của hệ thống', true);
            return;
        }
        try {
            await openInstanceFolder(dir);
        } catch (err: any) {
            showMsg(err.message || 'Không thể mở thư mục', false);
        }
    };

    // Filtered instances
    const filteredInstances = useMemo(() => {
        return instances.filter((inst) => {
            const isRunning = inst.is_running || inst.isRunning;
            if (statusFilter === 'running' && !isRunning) return false;
            if (statusFilter === 'stopped' && isRunning) return false;

            if (platformFilter !== 'all' && inst.platform_id.toLowerCase() !== platformFilter.toLowerCase()) {
                return false;
            }

            if (searchQuery.trim()) {
                const q = searchQuery.toLowerCase().trim();
                const name = inst.name.toLowerCase();
                const platform = inst.platform_id.toLowerCase();
                const dir = (inst.user_data_dir || inst.userDataDir || '').toLowerCase();
                const acc = (inst.bound_account_id || inst.bindAccountId || '').toLowerCase();
                return name.includes(q) || platform.includes(q) || dir.includes(q) || acc.includes(q);
            }

            return true;
        });
    }, [instances, platformFilter, statusFilter, searchQuery]);

    // Statistics
    const stats = useMemo(() => {
        const total = instances.length;
        const running = instances.filter((i) => i.is_running || i.isRunning).length;
        const stopped = total - running;
        const platforms = new Set(instances.map((i) => i.platform_id)).size;
        return { total, running, stopped, platforms };
    }, [instances]);

    // Account resolver
    const getAccountLabel = (inst: InstanceProfile) => {
        const uid = inst.bound_account_id || inst.bindAccountId;
        if (!uid) return null;
        const acc = accounts.find((a) => a.uid === uid || a.nickname === uid);
        return acc ? (acc.nickname || acc.uid) : uid;
    };

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', paddingBottom: '3rem' }}>
            {/* Action Feedback Toast */}
            {actionMsg && (
                <div
                    style={{
                        padding: '0.75rem 1rem',
                        borderRadius: '0.5rem',
                        backgroundColor: actionMsg.ok ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                        border: `1px solid ${actionMsg.ok ? '#10b981' : '#ef4444'}`,
                        color: actionMsg.ok ? '#34d399' : '#f87171',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.6rem',
                        fontSize: '0.85rem',
                    }}
                >
                    {actionMsg.ok ? <CheckCircle2 size={16} /> : <AlertCircle size={16} />}
                    {actionMsg.text}
                </div>
            )}

            {/* Page Header */}
            <div>
                <h1 style={{ fontSize: '1.5rem', fontWeight: 700, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                    <Layers size={24} color="#38bdf8" /> Global Instances & Process Monitor
                </h1>
                <p style={{ fontSize: '0.85rem', color: '#94a3b8', marginTop: '0.25rem' }}>
                    Giám sát trạng thái tiến trình (PID, trạng thái chạy/dừng, thư mục userDataDir) của toàn bộ các Instances trên hệ sinh thái IDE.
                </p>
            </div>

            {/* Architecture Info Banner */}
            <div
                style={{
                    backgroundColor: 'rgba(56, 189, 248, 0.08)',
                    border: '1px solid rgba(56, 189, 248, 0.25)',
                    borderRadius: '0.625rem',
                    padding: '1rem 1.25rem',
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: '0.85rem',
                }}
            >
                <Info size={20} color="#38bdf8" style={{ marginTop: '0.15rem', flexShrink: 0 }} />
                <div style={{ fontSize: '0.825rem', color: '#cbd5e1', lineHeight: '1.45' }}>
                    <span style={{ fontWeight: 600, color: '#38bdf8' }}>Cơ chế quản lý Instance theo chuẩn Cockpit Tools:</span>{' '}
                    Mỗi nền tảng (Antigravity, CodeBuddy, Zed, Claude, Cursor, Windsurf, Trae...) có không gian lưu trữ và danh sách tài khoản riêng biệt để tránh rò rỉ hoặc nhầm lẫn credential.
                    Để tạo mới hoặc cấu hình gán tài khoản cho Instance, vui lòng nhấp vào nút <span style={{ color: '#38bdf8' }}>"Mở Quản lý [Platform]"</span> tương ứng.
                </div>
            </div>

            {/* Metrics Bar */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
                {/* Total */}
                <div className="card" style={{ padding: '1rem 1.25rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ backgroundColor: 'rgba(56, 189, 248, 0.15)', color: '#38bdf8', padding: '0.75rem', borderRadius: '0.5rem' }}>
                        <Layers size={22} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase', fontWeight: 600 }}>
                            Tổng số Instances
                        </div>
                        <div style={{ fontSize: '1.4rem', fontWeight: 700, color: '#f8fafc' }}>
                            {stats.total}
                        </div>
                    </div>
                </div>

                {/* Running */}
                <div className="card" style={{ padding: '1rem 1.25rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ backgroundColor: 'rgba(16, 185, 129, 0.15)', color: '#34d399', padding: '0.75rem', borderRadius: '0.5rem' }}>
                        <Activity size={22} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase', fontWeight: 600 }}>
                            Đang Chạy (Live)
                        </div>
                        <div style={{ fontSize: '1.4rem', fontWeight: 700, color: '#34d399', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            {stats.running}
                            {stats.running > 0 && (
                                <span style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: '#10b981', boxShadow: '0 0 8px #10b981' }} />
                            )}
                        </div>
                    </div>
                </div>

                {/* Stopped */}
                <div className="card" style={{ padding: '1rem 1.25rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ backgroundColor: 'rgba(100, 116, 139, 0.15)', color: '#94a3b8', padding: '0.75rem', borderRadius: '0.5rem' }}>
                        <Square size={22} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase', fontWeight: 600 }}>
                            Đang Dừng
                        </div>
                        <div style={{ fontSize: '1.4rem', fontWeight: 700, color: '#cbd5e1' }}>
                            {stats.stopped}
                        </div>
                    </div>
                </div>

                {/* Platforms In Use */}
                <div className="card" style={{ padding: '1rem 1.25rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ backgroundColor: 'rgba(168, 85, 247, 0.15)', color: '#c084fc', padding: '0.75rem', borderRadius: '0.5rem' }}>
                        <HardDrive size={22} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase', fontWeight: 600 }}>
                            Nền tảng phát hiện
                        </div>
                        <div style={{ fontSize: '1.4rem', fontWeight: 700, color: '#f8fafc' }}>
                            {stats.platforms}
                        </div>
                    </div>
                </div>
            </div>

            {/* Quick Platform Jump Pills */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                <div style={{ fontSize: '0.8rem', color: '#94a3b8', fontWeight: 600 }}>
                    Truy cập nhanh trang quản lý Instances theo Nền tảng:
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem' }}>
                    {Object.entries(PLATFORMS_MAP)
                        .filter(([k]) => ['antigravity', 'codebuddy', 'zed', 'github_copilot', 'cursor', 'windsurf', 'trae', 'claude', 'codex', 'kiro', 'qoder'].includes(k))
                        .map(([key, info]) => (
                            <button
                                key={key}
                                onClick={() => navigate(info.path)}
                                style={{
                                    backgroundColor: '#1e293b',
                                    border: '1px solid #334155',
                                    borderRadius: '0.375rem',
                                    padding: '0.35rem 0.65rem',
                                    fontSize: '0.75rem',
                                    color: '#cbd5e1',
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '0.4rem',
                                    cursor: 'pointer',
                                    transition: 'all 0.15s',
                                }}
                            >
                                <span style={{ width: '6px', height: '6px', borderRadius: '50%', backgroundColor: info.color }} />
                                {info.label}
                                <ArrowRight size={12} color="#64748b" />
                            </button>
                        ))}
                </div>
            </div>

            {/* Controls Toolbar */}
            <div
                style={{
                    display: 'flex',
                    flexWrap: 'wrap',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: '0.75rem',
                    backgroundColor: '#1e293b',
                    padding: '0.75rem 1rem',
                    borderRadius: '0.625rem',
                    border: '1px solid #334155',
                }}
            >
                {/* Search & Filters */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', flex: 1, minWidth: '320px', flexWrap: 'wrap' }}>
                    <div
                        style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.5rem',
                            backgroundColor: '#0f172a',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            padding: '0.4rem 0.6rem',
                            flex: 1,
                            maxWidth: '280px',
                        }}
                    >
                        <Search size={14} color="#64748b" />
                        <input
                            type="text"
                            placeholder="Tìm kiếm instance, platform, dir..."
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            style={{
                                background: 'transparent',
                                border: 'none',
                                color: '#f8fafc',
                                fontSize: '0.8rem',
                                outline: 'none',
                                width: '100%',
                            }}
                        />
                    </div>

                    {/* Platform Filter */}
                    <select
                        value={platformFilter}
                        onChange={(e) => setPlatformFilter(e.target.value)}
                        style={{
                            backgroundColor: '#0f172a',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            color: '#cbd5e1',
                            fontSize: '0.8rem',
                            padding: '0.45rem 0.6rem',
                            outline: 'none',
                            cursor: 'pointer',
                        }}
                    >
                        <option value="all">Tất cả Nền tảng</option>
                        {Array.from(new Set(instances.map((i) => i.platform_id))).map((p) => (
                            <option key={p} value={p}>
                                {PLATFORMS_MAP[p]?.label || p}
                            </option>
                        ))}
                    </select>

                    {/* Status Filter */}
                    <select
                        value={statusFilter}
                        onChange={(e) => setStatusFilter(e.target.value as any)}
                        style={{
                            backgroundColor: '#0f172a',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            color: '#cbd5e1',
                            fontSize: '0.8rem',
                            padding: '0.45rem 0.6rem',
                            outline: 'none',
                            cursor: 'pointer',
                        }}
                    >
                        <option value="all">Tất cả Trạng thái</option>
                        <option value="running">Đang Chạy (Running)</option>
                        <option value="stopped">Đang Dừng (Stopped)</option>
                    </select>
                </div>

                {/* Right controls */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <button
                        onClick={() => loadData(true)}
                        disabled={refreshing}
                        title="Làm mới trạng thái"
                        style={{
                            backgroundColor: '#0f172a',
                            color: '#cbd5e1',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            padding: '0.45rem 0.75rem',
                            fontSize: '0.8rem',
                            fontWeight: 500,
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            cursor: 'pointer',
                        }}
                    >
                        <RefreshCw size={14} className={refreshing ? 'animate-spin' : ''} />
                        Làm mới
                    </button>
                </div>
            </div>

            {/* Global Table */}
            <div
                style={{
                    backgroundColor: '#1e293b',
                    borderRadius: '0.625rem',
                    border: '1px solid #334155',
                    overflow: 'hidden',
                }}
            >
                {/* Columns Header */}
                <div
                    style={{
                        display: 'grid',
                        gridTemplateColumns: '150px minmax(200px, 1.8fr) minmax(180px, 1.5fr) minmax(110px, 1fr) 180px',
                        padding: '0.75rem 1.25rem',
                        backgroundColor: '#172033',
                        borderBottom: '1px solid #334155',
                        color: '#94a3b8',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        textTransform: 'uppercase',
                        letterSpacing: '0.05em',
                    }}
                >
                    <div>Nền tảng</div>
                    <div>Instance</div>
                    <div>Tài khoản gán</div>
                    <div>Tiến trình (PID)</div>
                    <div style={{ textAlign: 'right' }}>Thao tác</div>
                </div>

                {/* Body */}
                {loading ? (
                    <div style={{ padding: '3rem', textAlign: 'center', color: '#94a3b8' }}>
                        <RefreshCw size={24} className="animate-spin" style={{ margin: '0 auto 0.5rem auto' }} />
                        <div>Đang quét tiến trình và nạp thông tin instances...</div>
                    </div>
                ) : filteredInstances.length === 0 ? (
                    <div style={{ padding: '3rem', textAlign: 'center', color: '#64748b' }}>
                        <Layers size={32} style={{ margin: '0 auto 0.5rem auto', opacity: 0.5 }} />
                        <div>Không tìm thấy instance nào phù hợp với bộ lọc.</div>
                    </div>
                ) : (
                    <div>
                        {filteredInstances.map((inst) => {
                            const isRunning = inst.is_running || inst.isRunning;
                            const isDef = inst.is_default || inst.isDefault || inst.id === 'default';
                            const pid = inst.last_pid || inst.lastPid;
                            const dir = inst.user_data_dir || inst.userDataDir;
                            const accLabel = getAccountLabel(inst);
                            const platInfo = PLATFORMS_MAP[inst.platform_id.toLowerCase()] || {
                                label: inst.platform_id,
                                path: `/platforms/${inst.platform_id}`,
                                color: '#94a3b8',
                            };

                            return (
                                <div
                                    key={`${inst.platform_id}_${inst.id}`}
                                    style={{
                                        display: 'grid',
                                        gridTemplateColumns: '150px minmax(200px, 1.8fr) minmax(180px, 1.5fr) minmax(110px, 1fr) 180px',
                                        padding: '1rem 1.25rem',
                                        borderBottom: '1px solid rgba(51, 65, 85, 0.6)',
                                        alignItems: 'center',
                                        backgroundColor: isRunning ? 'rgba(16, 185, 129, 0.03)' : 'transparent',
                                    }}
                                >
                                    {/* 1. Platform Badge */}
                                    <div>
                                        <span
                                            style={{
                                                display: 'inline-flex',
                                                alignItems: 'center',
                                                gap: '0.4rem',
                                                backgroundColor: '#0f172a',
                                                border: `1px solid ${platInfo.color}40`,
                                                color: platInfo.color,
                                                borderRadius: '0.375rem',
                                                padding: '0.25rem 0.55rem',
                                                fontSize: '0.75rem',
                                                fontWeight: 600,
                                            }}
                                        >
                                            <span
                                                style={{
                                                    width: '6px',
                                                    height: '6px',
                                                    borderRadius: '50%',
                                                    backgroundColor: platInfo.color,
                                                }}
                                            />
                                            {platInfo.label}
                                        </span>
                                    </div>

                                    {/* 2. Instance Name & Status */}
                                    <div>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                                            <span style={{ fontWeight: 600, color: '#f8fafc', fontSize: '0.9rem' }}>
                                                {inst.name}
                                            </span>
                                            {isRunning ? (
                                                <span
                                                    style={{
                                                        display: 'inline-flex',
                                                        alignItems: 'center',
                                                        gap: '0.3rem',
                                                        backgroundColor: 'rgba(16, 185, 129, 0.15)',
                                                        color: '#34d399',
                                                        border: '1px solid rgba(16, 185, 129, 0.3)',
                                                        padding: '0.12rem 0.4rem',
                                                        borderRadius: '9999px',
                                                        fontSize: '0.68rem',
                                                        fontWeight: 700,
                                                    }}
                                                >
                                                    <span
                                                        style={{
                                                            width: '5px',
                                                            height: '5px',
                                                            borderRadius: '50%',
                                                            backgroundColor: '#10b981',
                                                        }}
                                                    />
                                                    RUNNING
                                                </span>
                                            ) : (
                                                <span
                                                    style={{
                                                        display: 'inline-flex',
                                                        alignItems: 'center',
                                                        gap: '0.3rem',
                                                        backgroundColor: 'rgba(100, 116, 139, 0.15)',
                                                        color: '#94a3b8',
                                                        border: '1px solid rgba(100, 116, 139, 0.3)',
                                                        padding: '0.12rem 0.4rem',
                                                        borderRadius: '9999px',
                                                        fontSize: '0.68rem',
                                                        fontWeight: 600,
                                                    }}
                                                >
                                                    STOPPED
                                                </span>
                                            )}
                                        </div>
                                        <div style={{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.2rem' }}>
                                            {isDef ? 'Mặc định hệ thống' : dir}
                                        </div>
                                    </div>

                                    {/* 3. Account */}
                                    <div>
                                        {accLabel ? (
                                            <span style={{ fontSize: '0.825rem', color: '#e2e8f0', fontWeight: 500 }}>
                                                {accLabel}
                                            </span>
                                        ) : (
                                            <span style={{ fontSize: '0.78rem', color: '#64748b' }}>-- Chưa gán --</span>
                                        )}
                                    </div>

                                    {/* 4. Process PID */}
                                    <div>
                                        {isRunning && pid ? (
                                            <span
                                                style={{
                                                    display: 'inline-flex',
                                                    alignItems: 'center',
                                                    gap: '0.3rem',
                                                    backgroundColor: 'rgba(6, 182, 212, 0.12)',
                                                    color: '#22d3ee',
                                                    border: '1px solid rgba(6, 182, 212, 0.3)',
                                                    padding: '0.2rem 0.5rem',
                                                    borderRadius: '0.375rem',
                                                    fontSize: '0.75rem',
                                                    fontWeight: 600,
                                                    fontFamily: 'monospace',
                                                }}
                                            >
                                                PID: {pid}
                                            </span>
                                        ) : (
                                            <span style={{ color: '#64748b', fontSize: '0.85rem' }}>-</span>
                                        )}
                                    </div>

                                    {/* 5. Actions */}
                                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '0.4rem' }}>
                                        {/* Folder */}
                                        <button
                                            title="Mở thư mục"
                                            onClick={() => handleOpenFolder(inst)}
                                            style={{
                                                backgroundColor: '#0f172a',
                                                border: '1px solid #334155',
                                                borderRadius: '0.375rem',
                                                color: '#cbd5e1',
                                                padding: '0.4rem',
                                                cursor: 'pointer',
                                            }}
                                        >
                                            <Folder size={14} />
                                        </button>

                                        {/* Play / Stop */}
                                        {isRunning ? (
                                            <button
                                                title="Dừng tiến trình"
                                                onClick={() => handleStop(inst)}
                                                style={{
                                                    backgroundColor: 'rgba(239, 68, 68, 0.15)',
                                                    border: '1px solid rgba(239, 68, 68, 0.4)',
                                                    borderRadius: '0.375rem',
                                                    color: '#f87171',
                                                    padding: '0.4rem',
                                                    cursor: 'pointer',
                                                }}
                                            >
                                                <Square size={14} fill="#f87171" />
                                            </button>
                                        ) : (
                                            <button
                                                title="Khởi chạy tiến trình"
                                                onClick={() => handleLaunch(inst)}
                                                style={{
                                                    backgroundColor: 'rgba(16, 185, 129, 0.15)',
                                                    border: '1px solid rgba(16, 185, 129, 0.4)',
                                                    borderRadius: '0.375rem',
                                                    color: '#34d399',
                                                    padding: '0.4rem',
                                                    cursor: 'pointer',
                                                }}
                                            >
                                                <Play size={14} fill="#34d399" />
                                            </button>
                                        )}

                                        {/* Go to provider manager */}
                                        <button
                                            title={`Mở quản lý ${platInfo.label}`}
                                            onClick={() => navigate(platInfo.path)}
                                            style={{
                                                backgroundColor: '#0f172a',
                                                border: '1px solid #334155',
                                                borderRadius: '0.375rem',
                                                color: platInfo.color,
                                                padding: '0.4rem 0.6rem',
                                                fontSize: '0.75rem',
                                                fontWeight: 600,
                                                display: 'flex',
                                                alignItems: 'center',
                                                gap: '0.3rem',
                                                cursor: 'pointer',
                                            }}
                                        >
                                            Quản lý <ExternalLink size={12} />
                                        </button>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                )}
            </div>
        </div>
    );
}
export default InstancesPage;
