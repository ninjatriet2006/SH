import {
    Search,
    Eye,
    EyeOff,
    Play,
    Square,
    RefreshCw,
    Plus,
} from 'lucide-react';

interface InstancesToolbarProps {
    searchQuery: string;
    setSearchQuery: (q: string) => void;
    sortBy: 'creation_time' | 'launch_time' | 'name';
    setSortBy: (sort: 'creation_time' | 'launch_time' | 'name') => void;
    privacyMode: boolean;
    setPrivacyMode: (p: boolean) => void;
    onStartAll: () => void;
    onStopAll: () => void;
    onRefresh: () => void;
    refreshing: boolean;
    onOpenCreateModal: () => void;
}

export function InstancesToolbar({
    searchQuery,
    setSearchQuery,
    sortBy,
    setSortBy,
    privacyMode,
    setPrivacyMode,
    onStartAll,
    onStopAll,
    onRefresh,
    refreshing,
    onOpenCreateModal,
}: InstancesToolbarProps) {
    return (
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
                        placeholder="Tìm kiếm instance..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        style={{
                            backgroundColor: 'transparent',
                            border: 'none',
                            color: '#f8fafc',
                            fontSize: '0.85rem',
                            outline: 'none',
                            width: '100%',
                        }}
                    />
                </div>

                {/* Sort */}
                <select
                    value={sortBy}
                    onChange={(e) => setSortBy(e.target.value as any)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        color: '#cbd5e1',
                        borderRadius: '0.4rem',
                        padding: '0.4rem 0.6rem',
                        fontSize: '0.8rem',
                        cursor: 'pointer',
                        outline: 'none',
                    }}
                >
                    <option value="creation_time">Sắp xếp: Thời gian tạo</option>
                    <option value="launch_time">Sắp xếp: Lần chạy gần nhất</option>
                    <option value="name">Sắp xếp: Tên A-Z</option>
                </select>

                {/* Privacy Mode Toggle */}
                <button
                    title={privacyMode ? 'Tắt ẩn thông tin nhạy cảm' : 'Bật ẩn thông tin nhạy cảm'}
                    onClick={() => setPrivacyMode(!privacyMode)}
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        borderRadius: '0.4rem',
                        padding: '0.4rem 0.6rem',
                        color: privacyMode ? '#38bdf8' : '#94a3b8',
                        display: 'flex',
                        alignItems: 'center',
                        cursor: 'pointer',
                    }}
                >
                    {privacyMode ? <EyeOff size={15} /> : <Eye size={15} />}
                </button>
            </div>

            {/* Right Action Buttons */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                <button
                    onClick={onStartAll}
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.35rem',
                        backgroundColor: 'rgba(16, 185, 129, 0.15)',
                        border: '1px solid rgba(16, 185, 129, 0.4)',
                        color: '#34d399',
                        borderRadius: '0.4rem',
                        padding: '0.45rem 0.75rem',
                        fontSize: '0.8rem',
                        fontWeight: 600,
                        cursor: 'pointer',
                    }}
                >
                    <Play size={13} fill="#34d399" />
                    Chạy tất cả
                </button>

                <button
                    onClick={onStopAll}
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.35rem',
                        backgroundColor: 'rgba(239, 68, 68, 0.15)',
                        border: '1px solid rgba(239, 68, 68, 0.4)',
                        color: '#f87171',
                        borderRadius: '0.4rem',
                        padding: '0.45rem 0.75rem',
                        fontSize: '0.8rem',
                        fontWeight: 600,
                        cursor: 'pointer',
                    }}
                >
                    <Square size={13} fill="#f87171" />
                    Dừng tất cả
                </button>

                <button
                    onClick={onRefresh}
                    title="Làm mới danh sách"
                    style={{
                        backgroundColor: '#0f172a',
                        border: '1px solid #334155',
                        color: '#cbd5e1',
                        borderRadius: '0.4rem',
                        padding: '0.45rem',
                        display: 'flex',
                        alignItems: 'center',
                        cursor: 'pointer',
                    }}
                >
                    <RefreshCw size={15} className={refreshing ? 'animate-spin' : ''} />
                </button>

                <button
                    onClick={onOpenCreateModal}
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.35rem',
                        backgroundColor: '#0284c7',
                        border: 'none',
                        color: '#ffffff',
                        borderRadius: '0.4rem',
                        padding: '0.45rem 0.85rem',
                        fontSize: '0.8rem',
                        fontWeight: 600,
                        cursor: 'pointer',
                    }}
                >
                    <Plus size={15} />
                    Tạo Instance Mới
                </button>
            </div>
        </div>
    );
}
