import React, { useState, useEffect, useMemo } from 'react';
import {
    CheckCircle2,
    AlertCircle,
    Layers,
    Plus,
} from 'lucide-react';
import {
    listPlatformInstances,
    createPlatformInstance,
    updatePlatformInstance,
    deletePlatformInstance,
    launchPlatformInstance,
    stopPlatformInstance,
    openInstanceFolder,
} from '../../../bridge/profiles_bridge';
import {
    type PlatformInstancesContentProps,
    type InstanceProfile,
    InstancesToolbar,
    InstanceCard,
    CreateInstanceModal,
    EditInstanceModal,
} from './instances';

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

            {/* Top Toolbar */}
            <InstancesToolbar
                searchQuery={searchQuery}
                setSearchQuery={setSearchQuery}
                sortBy={sortBy}
                setSortBy={setSortBy}
                privacyMode={privacyMode}
                setPrivacyMode={setPrivacyMode}
                onStartAll={handleStartAll}
                onStopAll={handleStopAll}
                onRefresh={() => loadData(true)}
                refreshing={refreshing}
                onOpenCreateModal={handleOpenCreateModal}
            />

            {/* Content Table / Empty state */}
            {loading ? (
                <div style={{ textAlign: 'center', padding: '3rem', color: '#94a3b8' }}>
                    Đang tải danh sách instances...
                </div>
            ) : displayedInstances.length === 0 ? (
                <div
                    style={{
                        textAlign: 'center',
                        padding: '3rem 1.5rem',
                        backgroundColor: '#1e293b',
                        border: '1px solid #334155',
                        borderRadius: '0.625rem',
                    }}
                >
                    <Layers size={36} color="#64748b" style={{ margin: '0 auto 1rem' }} />
                    <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', marginBottom: '0.5rem' }}>
                        Chưa có instance nào
                    </h3>
                    <p style={{ fontSize: '0.85rem', color: '#94a3b8', maxWidth: '420px', margin: '0 auto 1.5rem' }}>
                        Tạo instance độc lập để đăng nhập nhiều tài khoản {platformLabel} cùng lúc mà không lo xung đột cấu hình.
                    </p>
                    <button
                        onClick={handleOpenCreateModal}
                        style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            backgroundColor: '#0284c7',
                            border: 'none',
                            color: '#ffffff',
                            borderRadius: '0.375rem',
                            padding: '0.5rem 1rem',
                            fontSize: '0.85rem',
                            fontWeight: 600,
                            cursor: 'pointer',
                        }}
                    >
                        <Plus size={16} />
                        Tạo Instance Đầu Tiên
                    </button>
                </div>
            ) : (
                <div
                    style={{
                        backgroundColor: '#1e293b',
                        border: '1px solid #334155',
                        borderRadius: '0.625rem',
                        overflow: 'hidden',
                    }}
                >
                    {/* Header */}
                    <div
                        style={{
                            display: 'grid',
                            gridTemplateColumns: 'minmax(220px, 1.8fr) minmax(200px, 2fr) minmax(120px, 1fr) 180px',
                            padding: '0.75rem 1.25rem',
                            backgroundColor: '#0f172a',
                            borderBottom: '1px solid #334155',
                            fontSize: '0.75rem',
                            fontWeight: 600,
                            color: '#94a3b8',
                            textTransform: 'uppercase',
                            letterSpacing: '0.05em',
                        }}
                    >
                        <div>Instance / Thư mục</div>
                        <div>Tài khoản gán</div>
                        <div>Tiến trình</div>
                        <div style={{ textAlign: 'right' }}>Thao tác</div>
                    </div>

                    {/* Rows */}
                    {displayedInstances.map((inst) => (
                        <InstanceCard
                            key={inst.id}
                            inst={inst}
                            accounts={accounts}
                            onLaunch={handleLaunch}
                            onStop={handleStop}
                            onOpenFolder={handleOpenFolder}
                            onClone={() => {
                                setNewInstanceName(`${inst.name}_clone`);
                                setInitMode('copy_source');
                                setSourceInstanceId(inst.id);
                                setCreateModalOpen(true);
                            }}
                            onEdit={handleOpenEditModal}
                            onDelete={handleDelete}
                            maskText={maskText}
                        />
                    ))}
                </div>
            )}

            {/* Modal: Create Instance */}
            <CreateInstanceModal
                isOpen={createModalOpen}
                onClose={() => setCreateModalOpen(false)}
                platformId={platformId}
                platformLabel={platformLabel}
                instances={instances}
                accounts={accounts}
                newInstanceName={newInstanceName}
                setNewInstanceName={setNewInstanceName}
                initMode={initMode}
                setInitMode={setInitMode}
                sourceInstanceId={sourceInstanceId}
                setSourceInstanceId={setSourceInstanceId}
                existingDir={existingDir}
                setExistingDir={setExistingDir}
                bindAccountId={bindAccountId}
                setBindAccountId={setBindAccountId}
                extraArgs={extraArgs}
                setExtraArgs={setExtraArgs}
                creating={creating}
                onSubmit={handleCreateSubmit}
            />

            {/* Modal: Edit Instance */}
            <EditInstanceModal
                isOpen={editModalOpen}
                onClose={() => setEditModalOpen(false)}
                targetInstance={targetInstance}
                platformLabel={platformLabel}
                accounts={accounts}
                editName={editName}
                setEditName={setEditName}
                editBindAccountId={editBindAccountId}
                setEditBindAccountId={setEditBindAccountId}
                editExtraArgs={editExtraArgs}
                setEditExtraArgs={setEditExtraArgs}
                savingEdit={savingEdit}
                onSubmit={handleEditSubmit}
            />
        </div>
    );
};
