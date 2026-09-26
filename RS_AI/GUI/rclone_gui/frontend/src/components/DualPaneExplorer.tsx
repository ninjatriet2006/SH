/*
[INTEGRITY NOTES]
- Mục đích: Bộ điều khiển trung tâm của Dual-Pane File Explorer.
- Trách nhiệm: Render hai Pane trái - phải, thanh công cụ tác vụ (Thêm thư mục, file, Copy/Move sang pane đối diện, Xoá, Terminal).
- Tương tác: Dùng `useExplorerStore`, `useJobsStore`, và `files_bridge.ts`.
*/

import {
  ArrowLeft,
  ArrowRight,
  Bookmark,
  BookmarkCheck,
  Copy,
  CornerDownRight,
  FolderPlus,
  MoveRight,
  Plus,
  RefreshCw,
  Search,
  Terminal,
  Trash2,
} from 'lucide-react';
import React, { useState } from 'react';
import { checkConflicts, openInTerminal } from '../../../bridge/files_bridge';
import type { ConflictInfo } from '../../../bridge/types';
import { type PaneType, useExplorerStore } from '../store/useExplorerStore';
import { useJobsStore } from '../store/useJobsStore';
import { formatBytes, joinPath } from '../utils/formatters';
import { Breadcrumbs } from './Breadcrumbs';
import { FileTable } from './FileTable';

interface DualPaneExplorerProps {
  onShowConflicts?: (conflicts: ConflictInfo[], onProceed: (skips: string[]) => void) => void;
  onShowProperties?: (path: string) => void;
  onShowSearch?: (path: string) => void;
}

export const DualPaneExplorer: React.FC<DualPaneExplorerProps> = ({
  onShowConflicts,
  onShowSearch,
}) => {
  const leftState = useExplorerStore((state) => state.left);
  const rightState = useExplorerStore((state) => state.right);
  const activePane = useExplorerStore((state) => state.activePane);
  const setActivePane = useExplorerStore((state) => state.setActivePane);
  const navigateUp = useExplorerStore((state) => state.navigateUp);
  const goBack = useExplorerStore((state) => state.goBack);
  const goForward = useExplorerStore((state) => state.goForward);
  const refreshPane = useExplorerStore((state) => state.refreshPane);
  const setSearchQuery = useExplorerStore((state) => state.setSearchQuery);
  const bookmarks = useExplorerStore((state) => state.bookmarks);
  const toggleBookmark = useExplorerStore((state) => state.toggleBookmark);

  const enqueueJob = useJobsStore((state) => state.enqueueJob);

  const [newFolderPrompt, setNewFolderPrompt] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newFilePrompt, setNewFilePrompt] = useState(false);
  const [newFileName, setNewFileName] = useState('');

  const activeState = activePane === 'left' ? leftState : rightState;
  const oppositePane: PaneType = activePane === 'left' ? 'right' : 'left';
  const oppositeState = oppositePane === 'left' ? leftState : rightState;

  const isCurrentBookmarked = bookmarks.some((b) => b.path === activeState.path);

  // Tạo thư mục mới
  const handleCreateFolder = async () => {
    if (!newFolderName.trim()) return;
    const fullPath = joinPath(activeState.path, newFolderName.trim());
    try {
      await enqueueJob('mkdir', fullPath);
      setNewFolderName('');
      setNewFolderPrompt(false);
      await refreshPane(activePane);
    } catch (err) {
      console.error('Lỗi tạo thư mục:', err);
    }
  };

  // Tạo file mới
  const handleCreateFile = async () => {
    if (!newFileName.trim()) return;
    const fullPath = joinPath(activeState.path, newFileName.trim());
    try {
      await enqueueJob('touch', fullPath);
      setNewFileName('');
      setNewFilePrompt(false);
      await refreshPane(activePane);
    } catch (err) {
      console.error('Lỗi tạo file:', err);
    }
  };

  // Xoá các mục được chọn
  const handleDeleteSelected = async () => {
    const selectedFiles = activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
    if (selectedFiles.length === 0) return;

    if (!confirm(`Bạn có chắc muốn xoá ${selectedFiles.length} mục đã chọn?`)) {
      return;
    }

    for (const item of selectedFiles) {
      const itemPath = joinPath(activeState.path, item.name);
      await enqueueJob('delete', itemPath);
    }
    await refreshPane(activePane);
  };

  // Copy sang Pane đối diện
  const handleTransferToOpposite = async (isMove: boolean) => {
    const selectedFiles = activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
    if (selectedFiles.length === 0 || !oppositeState.path) return;

    const srcPaths = selectedFiles.map((f) => joinPath(activeState.path, f.name));
    const destPath = oppositeState.path;

    try {
      const conflicts = await checkConflicts(srcPaths, destPath);
      if (conflicts.length > 0 && onShowConflicts) {
        onShowConflicts(conflicts, async (skipPaths) => {
          for (const src of srcPaths) {
            await enqueueJob(isMove ? 'move' : 'copy', src, destPath, skipPaths);
          }
          await refreshPane(oppositePane);
          if (isMove) await refreshPane(activePane);
        });
        return;
      }

      for (const src of srcPaths) {
        await enqueueJob(isMove ? 'move' : 'copy', src, destPath);
      }
      await refreshPane(oppositePane);
      if (isMove) await refreshPane(activePane);
    } catch (err) {
      console.error('Lỗi chuyển file:', err);
    }
  };

  // Mở terminal tại thư mục pane đang chọn
  const handleOpenTerminal = async () => {
    if (!activeState.path || activeState.path.includes('::')) return;
    try {
      await openInTerminal(activeState.path);
    } catch (err) {
      console.error('Lỗi mở terminal:', err);
    }
  };

  const renderPane = (pane: PaneType, state: typeof leftState) => {
    const isActive = activePane === pane;
    const selectedCount = state.selectedIds.size;
    const selectedSize = state.files
      .filter((f) => state.selectedIds.has(f.uuid) && !f.is_dir)
      .reduce((acc, f) => acc + f.size, 0);

    return (
      <div
        className={`explorer-pane ${isActive ? 'active' : ''}`}
        key={pane}
        onClick={() => setActivePane(pane)}
      >
        <div className="pane-header">
          {/* Navigation Bar */}
          <div className="pane-nav-bar">
            <button
              className="btn-icon"
              title="Quay lại"
              disabled={state.historyIndex <= 0}
              onClick={() => goBack(pane)}
            >
              <ArrowLeft size={14} />
            </button>
            <button
              className="btn-icon"
              title="Tiến lên"
              disabled={state.historyIndex >= state.history.length - 1}
              onClick={() => goForward(pane)}
            >
              <ArrowRight size={14} />
            </button>
            <button className="btn-icon" title="Lên thư mục cha" onClick={() => navigateUp(pane)}>
              <CornerDownRight size={14} style={{ transform: 'rotate(-90deg)' }} />
            </button>
            <button className="btn-icon" title="Làm mới" onClick={() => refreshPane(pane)}>
              <RefreshCw size={14} />
            </button>

            <Breadcrumbs pane={pane} />

            <button
              className="btn-icon"
              title={isCurrentBookmarked ? 'Bỏ ghim' : 'Ghim thư mục'}
              onClick={() => toggleBookmark(state.path.split('/').pop() || state.path, state.path)}
            >
              {isCurrentBookmarked ? (
                <BookmarkCheck size={14} color="#818cf8" />
              ) : (
                <Bookmark size={14} />
              )}
            </button>
          </div>

          {/* Quick Filter */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.35rem',
                background: 'rgba(0,0,0,0.25)',
                padding: '0.2rem 0.5rem',
                borderRadius: '6px',
                border: '1px solid var(--border)',
                flex: 1,
              }}
            >
              <Search size={12} color="var(--text-muted)" />
              <input
                type="text"
                placeholder="Lọc nhanh trong bảng..."
                style={{
                  background: 'transparent',
                  border: 'none',
                  outline: 'none',
                  color: 'var(--text-primary)',
                  fontSize: '0.78rem',
                  width: '100%',
                }}
                value={state.searchQuery}
                onChange={(e) => setSearchQuery(pane, e.target.value)}
              />
            </div>
          </div>
        </div>

        {/* File Table Content */}
        <FileTable pane={pane} />

        {/* Pane Status Bar */}
        <div className="pane-status-bar">
          <span>{state.files.length} mục</span>
          {selectedCount > 0 && (
            <span style={{ color: 'var(--primary-hover)' }}>
              Đã chọn: {selectedCount} ({formatBytes(selectedSize)})
            </span>
          )}
        </div>
      </div>
    );
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: '0.65rem' }}>
      {/* Global Toolbar for active pane */}
      <div
        className="glass-panel"
        style={{
          padding: '0.5rem 0.75rem',
          flexDirection: 'row',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', flexWrap: 'wrap' }}>
          <button className="btn btn-secondary btn-sm" onClick={() => setNewFolderPrompt(true)}>
            <FolderPlus size={14} />
            <span>Thư mục mới</span>
          </button>

          <button className="btn btn-secondary btn-sm" onClick={() => setNewFilePrompt(true)}>
            <Plus size={14} />
            <span>Tệp mới</span>
          </button>

          <div style={{ width: '1px', height: '18px', background: 'var(--border)' }} />

          <button
            className="btn btn-secondary btn-sm"
            disabled={activeState.selectedIds.size === 0}
            onClick={() => handleTransferToOpposite(false)}
            title="Sao chép các mục được chọn sang pane đối diện"
          >
            <Copy size={14} />
            <span>Chép sang {oppositePane === 'left' ? 'Trái' : 'Phải'}</span>
          </button>

          <button
            className="btn btn-secondary btn-sm"
            disabled={activeState.selectedIds.size === 0}
            onClick={() => handleTransferToOpposite(true)}
            title="Di chuyển các mục được chọn sang pane đối diện"
          >
            <MoveRight size={14} />
            <span>Chuyển sang {oppositePane === 'left' ? 'Trái' : 'Phải'}</span>
          </button>

          <button
            className="btn btn-danger btn-sm"
            disabled={activeState.selectedIds.size === 0}
            onClick={handleDeleteSelected}
          >
            <Trash2 size={14} />
            <span>Xoá ({activeState.selectedIds.size})</span>
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => onShowSearch?.(activeState.path)}
            title="Tìm kiếm đệ quy toàn cây thư mục"
          >
            <Search size={14} />
            <span>Tìm kiếm sâu</span>
          </button>

          {!activeState.path.includes('::') && (
            <button
              className="btn btn-secondary btn-sm"
              onClick={handleOpenTerminal}
              title="Mở ứng dụng dòng lệnh tại đây"
            >
              <Terminal size={14} />
              <span>Terminal</span>
            </button>
          )}
        </div>
      </div>

      {/* Dual Pane split view */}
      <div className="explorer-container">
        {renderPane('left', leftState)}
        {renderPane('right', rightState)}
      </div>

      {/* New Folder Modal */}
      {newFolderPrompt && (
        <div className="modal-overlay" onClick={() => setNewFolderPrompt(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 style={{ fontSize: '1rem', margin: 0 }}>Tạo thư mục mới</h3>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label">Tên thư mục:</label>
                <input
                  type="text"
                  className="input-text"
                  autoFocus
                  placeholder="Thư mục mới..."
                  value={newFolderName}
                  onChange={(e) => setNewFolderName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleCreateFolder();
                    else if (e.key === 'Escape') setNewFolderPrompt(false);
                  }}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-secondary btn-sm" onClick={() => setNewFolderPrompt(false)}>
                Huỷ
              </button>
              <button className="btn btn-primary btn-sm" onClick={handleCreateFolder}>
                Tạo
              </button>
            </div>
          </div>
        </div>
      )}

      {/* New File Modal */}
      {newFilePrompt && (
        <div className="modal-overlay" onClick={() => setNewFilePrompt(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 style={{ fontSize: '1rem', margin: 0 }}>Tạo tệp mới</h3>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label">Tên tệp tin:</label>
                <input
                  type="text"
                  className="input-text"
                  autoFocus
                  placeholder="tệp_mới.txt"
                  value={newFileName}
                  onChange={(e) => setNewFileName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleCreateFile();
                    else if (e.key === 'Escape') setNewFilePrompt(false);
                  }}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button className="btn btn-secondary btn-sm" onClick={() => setNewFilePrompt(false)}>
                Huỷ
              </button>
              <button className="btn btn-primary btn-sm" onClick={handleCreateFile}>
                Tạo
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
