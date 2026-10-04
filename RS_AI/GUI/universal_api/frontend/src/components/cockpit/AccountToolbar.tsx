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
    Tag,
} from 'lucide-react';

interface AccountToolbarProps {
    searchQuery: string;
    setSearchQuery: (query: string) => void;
    viewMode: 'grid' | 'list';
    setViewMode: (mode: 'grid' | 'list') => void;
    totalAccounts: number;
    activeTag: string;
    setActiveTag: (tag: string) => void;
    selectedCount: number;
    isAllSelected: boolean;
    onToggleSelectAll: () => void;
    onOpenAddModal: (tab?: 'oauth' | 'token' | 'local') => void;
    onRefreshAll: () => void;
    refreshing: boolean;
    privacyMode: boolean;
    setPrivacyMode: (privacy: boolean) => void;
    onExport: () => void;
}

export function AccountToolbar({
    searchQuery,
    setSearchQuery,
    viewMode,
    setViewMode,
    totalAccounts,
    activeTag,
    setActiveTag,
    isAllSelected,
    onToggleSelectAll,
    onOpenAddModal,
    onRefreshAll,
    refreshing,
    privacyMode,
    setPrivacyMode,
    onExport,
}: AccountToolbarProps) {
    const navigate = useNavigate();

    return (
        <>
            <div className="page-toolbar">
                <div className="toolbar-left">
                    <div className="search-box">
                        <Search size={15} />
                        <input
                            type="text"
                            placeholder="Search Nickname or UID..."
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                        />
                    </div>

                    <div className="view-toggle">
                        <button
                            className={`view-toggle-btn ${viewMode === 'grid' ? 'active' : ''}`}
                            onClick={() => setViewMode('grid')}
                            title="Grid View"
                        >
                            <LayoutGrid size={15} />
                        </button>
                        <button
                            className={`view-toggle-btn ${viewMode === 'list' ? 'active' : ''}`}
                            onClick={() => setViewMode('list')}
                            title="List View"
                        >
                            <List size={15} />
                        </button>
                    </div>

                    <button className="btn" style={{ height: 32, padding: '0 0.75rem', fontSize: '0.78rem' }}>
                        All ({totalAccounts})
                    </button>

                    <button className="btn" style={{ height: 32, padding: '0 0.75rem', fontSize: '0.78rem' }}>
                        <Tag size={13} /> Filter Tags
                    </button>
                </div>

                <div className="toolbar-right">
                    <button
                        className="toolbar-circle-add"
                        onClick={() => onOpenAddModal()}
                        title="Add Account"
                    >
                        <Plus size={18} />
                    </button>

                    <button
                        className="toolbar-icon-btn"
                        onClick={onRefreshAll}
                        disabled={refreshing}
                        title="Refresh Quota"
                    >
                        <RefreshCw size={15} className={refreshing ? 'spin' : ''} />
                    </button>

                    <button
                        className="toolbar-icon-btn"
                        onClick={() => setPrivacyMode(!privacyMode)}
                        title={privacyMode ? 'Show Full Email' : 'Mask Email'}
                    >
                        {privacyMode ? <Eye size={15} /> : <EyeOff size={15} />}
                    </button>

                    <button
                        className="toolbar-icon-btn"
                        onClick={onExport}
                        title="Export JSON"
                    >
                        <Download size={15} />
                    </button>

                    <button
                        className="toolbar-icon-btn"
                        onClick={() => onOpenAddModal('local')}
                        title="Import Accounts"
                    >
                        <Upload size={15} />
                    </button>

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
                        checked={isAllSelected}
                        onChange={onToggleSelectAll}
                    />
                    <span>Select All</span>
                </label>

                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                    <button
                        className={`filter-tab ${activeTag === 'all' ? 'active' : ''}`}
                        style={{ height: 26, padding: '0 0.8rem', fontSize: '0.72rem' }}
                        onClick={() => setActiveTag('all')}
                    >
                        全部 {totalAccounts}
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
        </>
    );
}
