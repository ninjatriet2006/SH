import { useState, useEffect, useMemo } from 'react';
import {
    Play,
    Square,
    Copy,
    Trash2,
    Plus,
    RefreshCw,
    Folder,
    Edit3,
    Eye,
    EyeOff,
    Search,
    CheckCircle2,
    AlertCircle,
    X,
    Layers,
} from 'lucide-react';
import {
    listPlatformInstances,
    createPlatformInstance,
    updatePlatformInstance,
    deletePlatformInstance,
    launchPlatformInstance,
    stopPlatformInstance,
    openInstanceFolder,
    type InstanceProfile,
} from '../../../bridge/profiles_bridge';
import type { AccountInfo } from '../../../bridge/types';

interface PlatformInstancesContentProps {
    platformId: string;
    platformLabel: string;
    accounts: AccountInfo[];
    onSwitchAccount?: (account: AccountInfo) => void;
}

export const PlatformInstancesContent: React.FC<PlatformInstancesContentProps> = ({
    platformId,
    platformLabel,
    accounts,
}) => {
    const [instances, setInstances] = useState<InstanceProfile[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [sortBy, setSortBy] = useState<'creation_time' | 'launch_time' | 'name'>('creation_time');
    const [privacyMode, setPrivacyMode] = useState(false);
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    // Modal states
    const [createModalOpen, setCreateModalOpen] = useState(false);
    const [editModalOpen, setEditModalOpen] = useState(false);
    const [targetInstance, setTargetInstance] = useState<InstanceProfile | null>(null);

    // Create Form state
    const [newInstanceName, setNewInstanceName] = useState('');
    const [initMode, setInitMode] = useState<'copy_source' | 'blank' | 'existing_dir'>('copy_source');
    const [sourceInstanceId, setSourceInstanceId] = useState('');
    const [existingDir, setExistingDir] = useState('');
    const [bindAccountId, setBindAccountId] = useState('');
    const [extraArgs, setExtraArgs] = useState('');
    const [creating, setCreating] = useState(false);

    // Edit Form state
    const [editName, setEditName] = useState('');
    const [editBindAccountId, setEditBindAccountId] = useState('');
    const [editExtraArgs, setEditExtraArgs] = useState('');
    const [savingEdit, setSavingEdit] = useState(false);

    const showMsg = (text: string, ok: boolean) => {
        setActionMsg({ text, ok });
        setTimeout(() => setActionMsg(null), 4000);
    };

    const loadData = async (isManual = false) => {
        if (isManual) setRefreshing(true);
        else setLoading(true);
        try {
            const list = await listPlatformInstances(platformId);
            setInstances(list);
            if (isManual) showMsg('Đã làm mới danh sách instances!', true);
        } catch (err: any) {
            console.error('Failed to load instances:', err);
            showMsg(err.message || 'Lỗi khi tải danh sách instances', false);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadData();
    }, [platformId]);

    // Handle Launch
    const handleLaunch = async (inst: InstanceProfile) => {
        try {
            const res = await launchPlatformInstance(platformId, inst.id);
            showMsg(`Đã khởi chạy "${inst.name}" (PID: ${res.pid})`, true);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi khởi chạy instance', false);
        }
    };

    // Handle Stop
    const handleStop = async (inst: InstanceProfile) => {
        try {
            await stopPlatformInstance(platformId, inst.id, inst.last_pid || inst.lastPid || undefined);
            showMsg(`Đã dừng tiến trình "${inst.name}"`, true);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi dừng instance', false);
        }
    };

    // Handle Start All
    const handleStartAll = async () => {
        const stopped = instances.filter((i) => !(i.is_running || i.isRunning));
        if (stopped.length === 0) {
            showMsg('Tất cả instances đã đang chạy!', true);
            return;
        }
        for (const inst of stopped) {
            try {
                await launchPlatformInstance(platformId, inst.id);
            } catch (_) {}
        }
        showMsg(`Đã gửi lệnh khởi chạy cho ${stopped.length} instance(s)`, true);
        await loadData();
    };

    // Handle Stop All
    const handleStopAll = async () => {
        const running = instances.filter((i) => i.is_running || i.isRunning);
        if (running.length === 0) {
            showMsg('Không có instance nào đang chạy', true);
            return;
        }
        for (const inst of running) {
            try {
                await stopPlatformInstance(platformId, inst.id, inst.last_pid || inst.lastPid || undefined);
            } catch (_) {}
        }
        showMsg(`Đã gửi lệnh dừng cho ${running.length} instance(s)`, true);
        await loadData();
    };

    // Handle Delete
    const handleDelete = async (inst: InstanceProfile) => {
        if (inst.is_default || inst.isDefault || inst.id === 'default') {
            showMsg('Không thể xóa Default Instance của ứng dụng', false);
            return;
        }
        if (!confirm(`Bạn có chắc chắn muốn xóa instance "${inst.name}"? Dữ liệu cô lập sẽ bị xóa.`)) {
            return;
        }
        try {
            await deletePlatformInstance(platformId, inst.id);
            showMsg(`Đã xóa instance "${inst.name}" thành công`, true);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi xóa instance', false);
        }
    };

    // Handle Open Folder
    const handleOpenFolder = async (inst: InstanceProfile) => {
        const dir = inst.user_data_dir || inst.userDataDir;
        if (!dir || dir === 'default') {
            showMsg('Default Instance sử dụng thư mục dữ liệu mặc định của hệ thống', true);
            return;
        }
        try {
            await openInstanceFolder(dir);
        } catch (err: any) {
            showMsg(err.message || 'Không thể mở thư mục', false);
        }
    };

    // Open Create Modal
    const handleOpenCreateModal = () => {
        setNewInstanceName(`${platformId}_work_${instances.length}`);
        setInitMode('copy_source');
        setSourceInstanceId(instances[0]?.id || 'default');
        setExistingDir('');
        setBindAccountId('');
        setExtraArgs('');
        setCreateModalOpen(true);
    };

    // Handle Submit Create
    const handleCreateSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newInstanceName.trim()) {
            showMsg('Vui lòng nhập tên instance', false);
            return;
        }
        setCreating(true);
        try {
            await createPlatformInstance({
                platform: platformId,
                name: newInstanceName.trim(),
                init_mode: initMode,
                source_instance_id: initMode === 'copy_source' ? sourceInstanceId : undefined,
                existing_dir: initMode === 'existing_dir' ? existingDir.trim() : undefined,
                bind_account_id: bindAccountId || undefined,
                extra_args: extraArgs.trim() || undefined,
            });
            showMsg(`Đã tạo instance "${newInstanceName}" thành công!`, true);
            setCreateModalOpen(false);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi tạo instance', false);
        } finally {
            setCreating(false);
        }
    };

    // Open Edit Modal
    const handleOpenEditModal = (inst: InstanceProfile) => {
        setTargetInstance(inst);
        setEditName(inst.name);
        setEditBindAccountId(inst.bound_account_id || inst.bindAccountId || '');
        const argsStr = typeof inst.extra_args === 'string'
            ? inst.extra_args
            : Array.isArray(inst.extra_args)
            ? inst.extra_args.join(' ')
            : typeof inst.extraArgs === 'string'
            ? inst.extraArgs
            : '';
        setEditExtraArgs(argsStr);
        setEditModalOpen(true);
    };

    // Handle Submit Edit
    const handleEditSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!targetInstance) return;
        setSavingEdit(true);
        try {
            await updatePlatformInstance({
                platform: platformId,
                instance_id: targetInstance.id,
                name: editName.trim(),
                bind_account_id: editBindAccountId ? editBindAccountId : null,
                extra_args: editExtraArgs.trim(),
            });
            showMsg(`Đã cập nhật instance "${editName}" thành công!`, true);
            setEditModalOpen(false);
            await loadData();
        } catch (err: any) {
            showMsg(err.message || 'Lỗi khi lưu instance', false);
        } finally {
            setSavingEdit(false);
        }
    };

    // Filter and Sort
    const displayedInstances = useMemo(() => {
        let list = [...instances];
        if (searchQuery.trim()) {
            const q = searchQuery.toLowerCase().trim();
            list = list.filter((i) => {
                const name = i.name.toLowerCase();
                const dir = (i.user_data_dir || i.userDataDir || '').toLowerCase();
                return name.includes(q) || dir.includes(q);
            });
        }
        if (sortBy === 'creation_time') {
            list.sort((a, b) => (b.created_at || '').localeCompare(a.created_at || ''));
        } else if (sortBy === 'launch_time') {
            list.sort((a, b) => (b.last_launched_at || '').localeCompare(a.last_launched_at || ''));
        } else if (sortBy === 'name') {
            list.sort((a, b) => a.name.localeCompare(b.name));
        }
        return list;
    }, [instances, searchQuery, sortBy]);

    // Mask value helper
    const maskText = (text: string) => {
        if (!privacyMode) return text;
        if (text.includes('@')) {
            const parts = text.split('@');
            return `${parts[0].slice(0, 3)}***@${parts[1]}`;
        }
        return text.length > 8 ? `${text.slice(0, 4)}***` : text;
    };

    // Find account info for an instance
    const getBoundAccount = (inst: InstanceProfile): AccountInfo | undefined => {
        const boundId = inst.bound_account_id || inst.bindAccountId;
        if (!boundId) return undefined;
        return accounts.find((a) => a.uid === boundId || a.nickname === boundId);
    };

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            {/* Action notification toast */}
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

            {/* Top Toolbar matching Screenshot 1 */}
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
                {/* Search, Sort, Privacy Toggle */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', flex: 1, minWidth: '320px' }}>
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
                            maxWidth: '260px',
                        }}
                    >
                        <Search size={14} color="#64748b" />
                        <input
                            type="text"
                            placeholder="Search by name or dir..."
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

                    <select
                        value={sortBy}
                        onChange={(e) => setSortBy(e.target.value as any)}
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
                        <option value="creation_time">Sort by: creation time</option>
                        <option value="launch_time">Sort by: launch time</option>
                        <option value="name">Sort by: name</option>
                    </select>

                    <button
                        title={privacyMode ? 'Show sensitive info' : 'Hide sensitive info'}
                        onClick={() => setPrivacyMode(!privacyMode)}
                        style={{
                            backgroundColor: '#0f172a',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            color: privacyMode ? '#38bdf8' : '#94a3b8',
                            padding: '0.45rem 0.6rem',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                        }}
                    >
                        {privacyMode ? <EyeOff size={15} /> : <Eye size={15} />}
                    </button>
                </div>

                {/* Right Action Buttons */}
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <button
                        onClick={handleOpenCreateModal}
                        style={{
                            backgroundColor: '#0284c7',
                            color: '#ffffff',
                            border: 'none',
                            borderRadius: '0.4rem',
                            padding: '0.45rem 0.85rem',
                            fontSize: '0.8rem',
                            fontWeight: 600,
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            cursor: 'pointer',
                            boxShadow: '0 2px 4px rgba(2, 132, 199, 0.3)',
                        }}
                    >
                        <Plus size={15} /> New Instance
                    </button>

                    <button
                        onClick={handleStartAll}
                        style={{
                            backgroundColor: '#0f172a',
                            color: '#34d399',
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
                        <Play size={13} fill="#34d399" /> Start All
                    </button>

                    <button
                        onClick={handleStopAll}
                        style={{
                            backgroundColor: '#0f172a',
                            color: '#f87171',
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
                        <Square size={13} fill="#f87171" /> Stop All
                    </button>

                    <button
                        onClick={() => loadData(true)}
                        disabled={refreshing}
                        title="Làm mới"
                        style={{
                            backgroundColor: '#0f172a',
                            color: '#94a3b8',
                            border: '1px solid #334155',
                            borderRadius: '0.4rem',
                            padding: '0.45rem 0.6rem',
                            cursor: 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                        }}
                    >
                        <RefreshCw size={14} className={refreshing ? 'animate-spin' : ''} />
                    </button>
                </div>
            </div>

            {/* Table Header matching Screenshot 1 */}
            <div
                style={{
                    backgroundColor: '#1e293b',
                    borderRadius: '0.625rem',
                    border: '1px solid #334155',
                    overflow: 'hidden',
                }}
            >
                {/* Columns Bar */}
                <div
                    style={{
                        display: 'grid',
                        gridTemplateColumns: 'minmax(220px, 1.8fr) minmax(200px, 2fr) minmax(120px, 1fr) 180px',
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
                    <div>Instance</div>
                    <div>Account</div>
                    <div>Process</div>
                    <div style={{ textAlign: 'right' }}>Actions</div>
                </div>

                {/* Table Body */}
                {loading ? (
                    <div style={{ padding: '3rem', textAlign: 'center', color: '#94a3b8' }}>
                        <RefreshCw size={24} className="animate-spin" style={{ margin: '0 auto 0.5rem auto' }} />
                        <div>Đang tải danh sách instance...</div>
                    </div>
                ) : displayedInstances.length === 0 ? (
                    <div style={{ padding: '3rem', textAlign: 'center', color: '#64748b' }}>
                        <Layers size={32} style={{ margin: '0 auto 0.5rem auto', opacity: 0.5 }} />
                        <div>Chưa có instance nào. Bấm <b>+ New Instance</b> để tạo instance đầu tiên.</div>
                    </div>
                ) : (
                    <div>
                        {displayedInstances.map((inst) => {
                            const isRunning = inst.is_running || inst.isRunning;
                            const isDef = inst.is_default || inst.isDefault || inst.id === 'default';
                            const boundAcc = getBoundAccount(inst);
                            const pid = inst.last_pid || inst.lastPid;
                            const dir = inst.user_data_dir || inst.userDataDir;

                            return (
                                <div
                                    key={inst.id}
                                    style={{
                                        display: 'grid',
                                        gridTemplateColumns: 'minmax(220px, 1.8fr) minmax(200px, 2fr) minmax(120px, 1fr) 180px',
                                        padding: '1rem 1.25rem',
                                        borderBottom: '1px solid rgba(51, 65, 85, 0.6)',
                                        alignItems: 'center',
                                        backgroundColor: isRunning ? 'rgba(16, 185, 129, 0.03)' : 'transparent',
                                        transition: 'background-color 0.2s',
                                    }}
                                >
                                    {/* 1. Instance Name + Badge + Dir */}
                                    <div>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                                            <span style={{ fontWeight: 600, color: '#f8fafc', fontSize: '0.9rem' }}>
                                                {inst.name}
                                            </span>
                                            {/* Status Badge */}
                                            {isRunning ? (
                                                <span
                                                    style={{
                                                        display: 'inline-flex',
                                                        alignItems: 'center',
                                                        gap: '0.3rem',
                                                        backgroundColor: 'rgba(16, 185, 129, 0.15)',
                                                        color: '#34d399',
                                                        border: '1px solid rgba(16, 185, 129, 0.3)',
                                                        padding: '0.15rem 0.45rem',
                                                        borderRadius: '9999px',
                                                        fontSize: '0.7rem',
                                                        fontWeight: 700,
                                                    }}
                                                >
                                                    <span
                                                        style={{
                                                            width: '6px',
                                                            height: '6px',
                                                            borderRadius: '50%',
                                                            backgroundColor: '#10b981',
                                                            boxShadow: '0 0 6px #10b981',
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
                                                        padding: '0.15rem 0.45rem',
                                                        borderRadius: '9999px',
                                                        fontSize: '0.7rem',
                                                        fontWeight: 600,
                                                    }}
                                                >
                                                    <span
                                                        style={{
                                                            width: '6px',
                                                            height: '6px',
                                                            borderRadius: '50%',
                                                            backgroundColor: '#64748b',
                                                        }}
                                                    />
                                                    STOPPED
                                                </span>
                                            )}
                                        </div>
                                        <div style={{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.25rem' }}>
                                            {isDef ? 'Default profile' : maskText(dir || 'Custom profile')}
                                        </div>
                                    </div>

                                    {/* 2. Account */}
                                    <div>
                                        {boundAcc ? (
                                            <div>
                                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                                    <span style={{ fontSize: '0.825rem', color: '#e2e8f0', fontWeight: 500 }}>
                                                        {maskText(boundAcc.nickname || boundAcc.uid)}
                                                    </span>
                                                    <span
                                                        style={{
                                                            backgroundColor: 'rgba(56, 189, 248, 0.15)',
                                                            color: '#38bdf8',
                                                            border: '1px solid rgba(56, 189, 248, 0.3)',
                                                            borderRadius: '0.25rem',
                                                            padding: '0.1rem 0.35rem',
                                                            fontSize: '0.65rem',
                                                            fontWeight: 700,
                                                        }}
                                                    >
                                                        PRO
                                                    </span>
                                                </div>
                                                {/* Quota details if present */}
                                                <div style={{ fontSize: '0.72rem', color: '#94a3b8', marginTop: '0.2rem' }}>
                                                    • Claude (5h) 100%
                                                </div>
                                            </div>
                                        ) : (
                                            <span style={{ fontSize: '0.8rem', color: '#64748b' }}>
                                                -- Chưa gán tài khoản --
                                            </span>
                                        )}
                                    </div>

                                    {/* 3. Process / PID */}
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

                                    {/* 4. Actions */}
                                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '0.4rem' }}>
                                        {/* Open Folder */}
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
                                                title="Dừng instance"
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
                                                title="Khởi chạy instance"
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

                                        {/* Clone */}
                                        <button
                                            title="Nhân bản"
                                            onClick={() => {
                                                setNewInstanceName(`${inst.name}_clone`);
                                                setInitMode('copy_source');
                                                setSourceInstanceId(inst.id);
                                                setCreateModalOpen(true);
                                            }}
                                            style={{
                                                backgroundColor: '#0f172a',
                                                border: '1px solid #334155',
                                                borderRadius: '0.375rem',
                                                color: '#cbd5e1',
                                                padding: '0.4rem',
                                                cursor: 'pointer',
                                            }}
                                        >
                                            <Copy size={14} />
                                        </button>

                                        {/* Edit */}
                                        <button
                                            title="Chỉnh sửa cấu hình"
                                            onClick={() => handleOpenEditModal(inst)}
                                            style={{
                                                backgroundColor: '#0f172a',
                                                border: '1px solid #334155',
                                                borderRadius: '0.375rem',
                                                color: '#cbd5e1',
                                                padding: '0.4rem',
                                                cursor: 'pointer',
                                            }}
                                        >
                                            <Edit3 size={14} />
                                        </button>

                                        {/* Delete */}
                                        <button
                                            title={isDef ? 'Không thể xóa Default Instance' : 'Xóa instance'}
                                            disabled={isDef}
                                            onClick={() => handleDelete(inst)}
                                            style={{
                                                backgroundColor: '#0f172a',
                                                border: '1px solid #334155',
                                                borderRadius: '0.375rem',
                                                color: isDef ? '#475569' : '#f87171',
                                                padding: '0.4rem',
                                                cursor: isDef ? 'not-allowed' : 'pointer',
                                                opacity: isDef ? 0.35 : 1,
                                            }}
                                        >
                                            <Trash2 size={14} />
                                        </button>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                )}
            </div>

            {/* Modal: New Instance (1:1 with Screenshot 2) */}
            {createModalOpen && (
                <div
                    style={{
                        position: 'fixed',
                        inset: 0,
                        backgroundColor: 'rgba(0, 0, 0, 0.75)',
                        backdropFilter: 'blur(4px)',
                        zIndex: 9999,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        padding: '1rem',
                    }}
                >
                    <div
                        style={{
                            backgroundColor: '#1e293b',
                            border: '1px solid #334155',
                            borderRadius: '0.75rem',
                            width: '100%',
                            maxWidth: '560px',
                            boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.6)',
                            overflow: 'hidden',
                        }}
                    >
                        {/* Header */}
                        <div
                            style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                                padding: '1rem 1.25rem',
                                borderBottom: '1px solid #334155',
                            }}
                        >
                            <h3 style={{ fontSize: '1.05rem', fontWeight: 600, color: '#f8fafc', margin: 0 }}>
                                New Instance
                            </h3>
                            <button
                                onClick={() => setCreateModalOpen(false)}
                                style={{
                                    background: 'transparent',
                                    border: 'none',
                                    color: '#94a3b8',
                                    cursor: 'pointer',
                                }}
                            >
                                <X size={18} />
                            </button>
                        </div>

                        {/* Form */}
                        <form onSubmit={handleCreateSubmit} style={{ padding: '1.25rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                            {/* 1. Instance Name */}
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Instance Name *
                                </label>
                                <input
                                    type="text"
                                    required
                                    placeholder="e.g. antigravity_work"
                                    value={newInstanceName}
                                    onChange={(e) => setNewInstanceName(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                />
                            </div>

                            {/* 2. Init Mode */}
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Init Mode
                                </label>
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                                    {/* Option 1: Copy source instance */}
                                    <label
                                        style={{
                                            display: 'flex',
                                            alignItems: 'flex-start',
                                            gap: '0.6rem',
                                            padding: '0.6rem 0.75rem',
                                            backgroundColor: initMode === 'copy_source' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                            border: `1px solid ${initMode === 'copy_source' ? '#0284c7' : '#334155'}`,
                                            borderRadius: '0.375rem',
                                            cursor: 'pointer',
                                        }}
                                    >
                                        <input
                                            type="radio"
                                            name="initMode"
                                            checked={initMode === 'copy_source'}
                                            onChange={() => setInitMode('copy_source')}
                                            style={{ marginTop: '0.2rem' }}
                                        />
                                        <div>
                                            <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                                Copy source instance
                                            </div>
                                            <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                                Clone configuration and extensions from an existing instance
                                            </div>
                                            {initMode === 'copy_source' && (
                                                <div style={{ marginTop: '0.5rem' }}>
                                                    <select
                                                        value={sourceInstanceId}
                                                        onChange={(e) => setSourceInstanceId(e.target.value)}
                                                        style={{
                                                            width: '100%',
                                                            backgroundColor: '#1e293b',
                                                            border: '1px solid #334155',
                                                            borderRadius: '0.25rem',
                                                            padding: '0.35rem 0.5rem',
                                                            color: '#f8fafc',
                                                            fontSize: '0.78rem',
                                                            outline: 'none',
                                                        }}
                                                    >
                                                        {instances.map((i) => (
                                                            <option key={i.id} value={i.id}>
                                                                {i.name} {i.is_default || i.isDefault ? '(Default)' : ''}
                                                            </option>
                                                        ))}
                                                    </select>
                                                </div>
                                            )}
                                        </div>
                                    </label>

                                    {/* Option 2: Blank instance */}
                                    <label
                                        style={{
                                            display: 'flex',
                                            alignItems: 'flex-start',
                                            gap: '0.6rem',
                                            padding: '0.6rem 0.75rem',
                                            backgroundColor: initMode === 'blank' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                            border: `1px solid ${initMode === 'blank' ? '#0284c7' : '#334155'}`,
                                            borderRadius: '0.375rem',
                                            cursor: 'pointer',
                                        }}
                                    >
                                        <input
                                            type="radio"
                                            name="initMode"
                                            checked={initMode === 'blank'}
                                            onChange={() => setInitMode('blank')}
                                            style={{ marginTop: '0.2rem' }}
                                        />
                                        <div>
                                            <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                                Blank instance
                                            </div>
                                            <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                                Initialize an empty, independent user data directory
                                            </div>
                                        </div>
                                    </label>

                                    {/* Option 3: Use existing directory */}
                                    <label
                                        style={{
                                            display: 'flex',
                                            alignItems: 'flex-start',
                                            gap: '0.6rem',
                                            padding: '0.6rem 0.75rem',
                                            backgroundColor: initMode === 'existing_dir' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                            border: `1px solid ${initMode === 'existing_dir' ? '#0284c7' : '#334155'}`,
                                            borderRadius: '0.375rem',
                                            cursor: 'pointer',
                                        }}
                                    >
                                        <input
                                            type="radio"
                                            name="initMode"
                                            checked={initMode === 'existing_dir'}
                                            onChange={() => setInitMode('existing_dir')}
                                            style={{ marginTop: '0.2rem' }}
                                        />
                                        <div style={{ flex: 1 }}>
                                            <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                                Use existing directory
                                            </div>
                                            <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                                Point to an already existing user data directory
                                            </div>
                                            {initMode === 'existing_dir' && (
                                                <div style={{ marginTop: '0.5rem' }}>
                                                    <input
                                                        type="text"
                                                        placeholder="/path/to/existing/userDataDir"
                                                        value={existingDir}
                                                        onChange={(e) => setExistingDir(e.target.value)}
                                                        style={{
                                                            width: '100%',
                                                            backgroundColor: '#1e293b',
                                                            border: '1px solid #334155',
                                                            borderRadius: '0.25rem',
                                                            padding: '0.35rem 0.5rem',
                                                            color: '#f8fafc',
                                                            fontSize: '0.78rem',
                                                            outline: 'none',
                                                        }}
                                                    />
                                                </div>
                                            )}
                                        </div>
                                    </label>
                                </div>
                            </div>

                            {/* 3. Instance Directory Preview */}
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Instance Directory
                                </label>
                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                    <input
                                        type="text"
                                        readOnly
                                        value={
                                            initMode === 'existing_dir' && existingDir
                                                ? existingDir
                                                : `~/.cockpit_tools/instances/${platformId}/${newInstanceName || '<name>'}`
                                        }
                                        style={{
                                            flex: 1,
                                            backgroundColor: '#0f172a',
                                            border: '1px solid #334155',
                                            borderRadius: '0.375rem',
                                            padding: '0.5rem 0.75rem',
                                            color: '#94a3b8',
                                            fontSize: '0.8rem',
                                        }}
                                    />
                                    <button
                                        type="button"
                                        onClick={() => {
                                            setInitMode('existing_dir');
                                        }}
                                        style={{
                                            backgroundColor: '#0f172a',
                                            border: '1px solid #334155',
                                            borderRadius: '0.375rem',
                                            color: '#cbd5e1',
                                            padding: '0.5rem 0.75rem',
                                            fontSize: '0.78rem',
                                            cursor: 'pointer',
                                            whiteSpace: 'nowrap',
                                        }}
                                    >
                                        Select Folder
                                    </button>
                                </div>
                            </div>

                            {/* 4. Bind Account (Strictly Filtered to this platform!) */}
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Bind account
                                </label>
                                <select
                                    value={bindAccountId}
                                    onChange={(e) => setBindAccountId(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                >
                                    <option value="">-- Do not bind account --</option>
                                    {accounts.map((a) => (
                                        <option key={a.uid} value={a.uid}>
                                            {a.nickname || a.uid}
                                        </option>
                                    ))}
                                </select>
                                <span style={{ fontSize: '0.72rem', color: '#64748b', marginTop: '0.2rem', display: 'block' }}>
                                    Chỉ hiển thị các tài khoản thuộc nền tảng {platformLabel} để bảo vệ an toàn thông tin xác thực.
                                </span>
                            </div>

                            {/* 5. Custom launch args */}
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Custom launch args (optional)
                                </label>
                                <input
                                    type="text"
                                    placeholder="e.g. --disable-gpu"
                                    value={extraArgs}
                                    onChange={(e) => setExtraArgs(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                />
                            </div>

                            {/* Footer Buttons */}
                            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.6rem', marginTop: '0.75rem' }}>
                                <button
                                    type="button"
                                    onClick={() => setCreateModalOpen(false)}
                                    style={{
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        color: '#94a3b8',
                                        padding: '0.5rem 1rem',
                                        fontSize: '0.85rem',
                                        cursor: 'pointer',
                                    }}
                                >
                                    Cancel
                                </button>
                                <button
                                    type="submit"
                                    disabled={creating}
                                    style={{
                                        backgroundColor: '#0284c7',
                                        border: 'none',
                                        borderRadius: '0.375rem',
                                        color: '#ffffff',
                                        padding: '0.5rem 1.25rem',
                                        fontSize: '0.85rem',
                                        fontWeight: 600,
                                        cursor: 'pointer',
                                    }}
                                >
                                    {creating ? 'Creating...' : 'Create Instance'}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}

            {/* Modal: Edit Instance */}
            {editModalOpen && targetInstance && (
                <div
                    style={{
                        position: 'fixed',
                        inset: 0,
                        backgroundColor: 'rgba(0, 0, 0, 0.75)',
                        backdropFilter: 'blur(4px)',
                        zIndex: 9999,
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        padding: '1rem',
                    }}
                >
                    <div
                        style={{
                            backgroundColor: '#1e293b',
                            border: '1px solid #334155',
                            borderRadius: '0.75rem',
                            width: '100%',
                            maxWidth: '480px',
                            boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.6)',
                            overflow: 'hidden',
                        }}
                    >
                        <div
                            style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                                padding: '1rem 1.25rem',
                                borderBottom: '1px solid #334155',
                            }}
                        >
                            <h3 style={{ fontSize: '1.05rem', fontWeight: 600, color: '#f8fafc', margin: 0 }}>
                                Edit Instance: {targetInstance.name}
                            </h3>
                            <button
                                onClick={() => setEditModalOpen(false)}
                                style={{
                                    background: 'transparent',
                                    border: 'none',
                                    color: '#94a3b8',
                                    cursor: 'pointer',
                                }}
                            >
                                <X size={18} />
                            </button>
                        </div>

                        <form onSubmit={handleEditSubmit} style={{ padding: '1.25rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Instance Name
                                </label>
                                <input
                                    type="text"
                                    required
                                    disabled={targetInstance.is_default || targetInstance.isDefault}
                                    value={editName}
                                    onChange={(e) => setEditName(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                />
                            </div>

                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Bind Account
                                </label>
                                <select
                                    value={editBindAccountId}
                                    onChange={(e) => setEditBindAccountId(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                >
                                    <option value="">-- Do not bind account --</option>
                                    {accounts.map((a) => (
                                        <option key={a.uid} value={a.uid}>
                                            {a.nickname || a.uid}
                                        </option>
                                    ))}
                                </select>
                            </div>

                            <div>
                                <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                                    Custom launch args
                                </label>
                                <input
                                    type="text"
                                    placeholder="e.g. --disable-gpu"
                                    value={editExtraArgs}
                                    onChange={(e) => setEditExtraArgs(e.target.value)}
                                    style={{
                                        width: '100%',
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        padding: '0.5rem 0.75rem',
                                        color: '#f8fafc',
                                        fontSize: '0.85rem',
                                        outline: 'none',
                                    }}
                                />
                            </div>

                            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.6rem', marginTop: '0.75rem' }}>
                                <button
                                    type="button"
                                    onClick={() => setEditModalOpen(false)}
                                    style={{
                                        backgroundColor: '#0f172a',
                                        border: '1px solid #334155',
                                        borderRadius: '0.375rem',
                                        color: '#94a3b8',
                                        padding: '0.5rem 1rem',
                                        fontSize: '0.85rem',
                                        cursor: 'pointer',
                                    }}
                                >
                                    Cancel
                                </button>
                                <button
                                    type="submit"
                                    disabled={savingEdit}
                                    style={{
                                        backgroundColor: '#0284c7',
                                        border: 'none',
                                        borderRadius: '0.375rem',
                                        color: '#ffffff',
                                        padding: '0.5rem 1.25rem',
                                        fontSize: '0.85rem',
                                        fontWeight: 600,
                                        cursor: 'pointer',
                                    }}
                                >
                                    {savingEdit ? 'Saving...' : 'Save Changes'}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}
        </div>
    );
};
