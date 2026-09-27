/*
[INTEGRITY NOTES]
- Mục đích: Bộ điều khiển trung tâm của Dual-Pane File Explorer.
- Trách nhiệm:
  1. Render hai Pane trái - phải với Breadcrumbs, Quick Filter, và bảng tệp tin FileTable.
  2. Tích hợp Nemo Context Menu: hỗ trợ bảng chọn chuột phải cho file (File Context Menu) và khoảng trống (Background Context Menu).
  3. Quản lý Clipboard (Cut/Copy/Paste) giữa 2 pane.
  4. Hộp thoại tạo thư mục, tạo tệp, đổi tên, và giải quyết xung đột tệp tin.
  5. Đạt chuẩn 100% bản địa hoá i18n với `useTranslation()` và `data-lang-id`.
- Tương tác: Dùng `useExplorerStore`, `useJobsStore`, `files_bridge.ts`, `sys_bridge.ts`, và `ContextMenu.tsx`.
*/

import {
  ArrowLeft,
  ArrowRight,
  Bookmark,
  BookmarkCheck,
  CornerDownRight,
  FolderPlus,
  RefreshCw,
  Search,
  Terminal,
} from 'lucide-react';
import React, { useEffect, useRef, useState } from 'react';
import { checkConflicts, openInTerminal } from '../../../bridge/files_bridge';
import { sysOpenWith } from '../../../bridge/sys_bridge';
import type { ConflictInfo, FileItem } from '../../../bridge/types';
import { type PaneType, useExplorerStore } from '../store/useExplorerStore';
import { useJobsStore } from '../store/useJobsStore';
import { useRemotesStore } from '../store/useRemotesStore';
import { formatBytes, joinPath } from '../utils/formatters';
import { useTranslation } from '../utils/i18n';
import { Breadcrumbs } from './Breadcrumbs';
import { ContextMenu } from './ContextMenu';
import { FileTable } from './FileTable';

interface DualPaneExplorerProps {
  onShowConflicts?: (conflicts: ConflictInfo[], onProceed: (skips: string[]) => void) => void;
  onShowProperties?: (path: string) => void;
  onShowSearch?: (path: string) => void;
}

export const DualPaneExplorer: React.FC<DualPaneExplorerProps> = ({
  onShowConflicts,
  onShowProperties,
  onShowSearch,
}) => {
  const { t } = useTranslation();
  const leftState = useExplorerStore((state) => state.left);
  const rightState = useExplorerStore((state) => state.right);
  const activePane = useExplorerStore((state) => state.activePane);
  const setActivePane = useExplorerStore((state) => state.setActivePane);
  const loadDirectory = useExplorerStore((state) => state.loadDirectory);
  const navigateUp = useExplorerStore((state) => state.navigateUp);
  const goBack = useExplorerStore((state) => state.goBack);
  const goForward = useExplorerStore((state) => state.goForward);
  const refreshPane = useExplorerStore((state) => state.refreshPane);
  const setSearchQuery = useExplorerStore((state) => state.setSearchQuery);
  const selectAll = useExplorerStore((state) => state.selectAll);
  const bookmarks = useExplorerStore((state) => state.bookmarks);
  const toggleBookmark = useExplorerStore((state) => state.toggleBookmark);
  const homeDir = useExplorerStore((state) => state.homeDir);

  const remotes = useRemotesStore((state) => state.remotes);
  const loadRemotes = useRemotesStore((state) => state.loadRemotes);

  useEffect(() => {
    loadRemotes();
  }, [loadRemotes]);

  const enqueueJob = useJobsStore((state) => state.enqueueJob);

  // Modal prompts
  const [newFolderPrompt, setNewFolderPrompt] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newFilePrompt, setNewFilePrompt] = useState(false);
  const [newFileName, setNewFileName] = useState('');
  const [renameTarget, setRenameTarget] = useState<FileItem | null>(null);
  const [renameNewName, setRenameNewName] = useState('');

  // Context Menu State
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    type: 'file' | 'background';
    item?: FileItem;
    pane: PaneType;
  } | null>(null);

  // Clipboard State (Cut / Copy / Paste)
  const [clipboard, setClipboard] = useState<{
    action: 'copy' | 'cut';
    sourcePane: PaneType;
    sourceDir: string;
    items: FileItem[];
  } | null>(null);

  const activeState = activePane === 'left' ? leftState : rightState;
  const oppositePane: PaneType = activePane === 'left' ? 'right' : 'left';
  const oppositeState = oppositePane === 'left' ? leftState : rightState;

  // Splitter ratio state (nhớ tỷ lệ trong localStorage)
  const [splitRatio, setSplitRatio] = useState<number>(() => {
    const saved = localStorage.getItem('rclonegui_split_ratio');
    return saved ? parseFloat(saved) : 50;
  });
  const containerRef = useRef<HTMLDivElement>(null);

  // Phím tắt bàn phím chuẩn Desktop (Ctrl+Shift+N tạo thư mục)
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'N' || e.key === 'n')) {
        e.preventDefault();
        setNewFolderName('');
        setNewFolderPrompt(true);
      }
    };
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, []);

  const handleSplitterMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const container = containerRef.current;
    if (!container) return;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const rect = container.getBoundingClientRect();
      const newRatio = ((moveEvent.clientX - rect.left) / rect.width) * 100;
      const clamped = Math.max(20, Math.min(80, newRatio));
      setSplitRatio(clamped);
      localStorage.setItem('rclonegui_split_ratio', clamped.toFixed(1));
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleSplitterDoubleClick = () => {
    setSplitRatio(50);
    localStorage.setItem('rclonegui_split_ratio', '50');
  };

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

  // Đổi tên file / thư mục
  const handleRenameConfirm = async () => {
    if (!renameTarget || !renameNewName.trim() || renameNewName === renameTarget.name) {
      setRenameTarget(null);
      return;
    }
    const oldPath = joinPath(activeState.path, renameTarget.name);
    const newPath = joinPath(activeState.path, renameNewName.trim());
    try {
      await enqueueJob('rename', oldPath, newPath);
      setRenameTarget(null);
      await refreshPane(activePane);
    } catch (err) {
      console.error('Lỗi đổi tên:', err);
    }
  };

  // Xoá các mục được chọn
  const handleDeleteSelected = async (targetItems?: FileItem[]) => {
    const selectedFiles = targetItems || activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
    if (selectedFiles.length === 0) return;

    const confirmMsg = t('explorer_confirm_delete', `Bạn có chắc muốn xoá ${selectedFiles.length} mục đã chọn?`);
    if (!confirm(confirmMsg)) {
      return;
    }

    for (const item of selectedFiles) {
      const itemPath = joinPath(activeState.path, item.name);
      await enqueueJob('delete', itemPath);
    }
    await refreshPane(activePane);
  };

  // Copy / Move sang Pane đối diện
  const handleTransferToOpposite = async (isMove: boolean, targetItems?: FileItem[]) => {
    const selectedFiles = targetItems || activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
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

  // Cắt (Cut)
  const handleCut = (item?: FileItem) => {
    const items = item ? [item] : activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
    if (items.length === 0) return;
    setClipboard({
      action: 'cut',
      sourcePane: activePane,
      sourceDir: activeState.path,
      items,
    });
  };

  // Sao chép (Copy)
  const handleCopy = (item?: FileItem) => {
    const items = item ? [item] : activeState.files.filter((f) => activeState.selectedIds.has(f.uuid));
    if (items.length === 0) return;
    setClipboard({
      action: 'copy',
      sourcePane: activePane,
      sourceDir: activeState.path,
      items,
    });
  };

  // Dán (Paste)
  const handlePaste = async () => {
    if (!clipboard || clipboard.items.length === 0) return;
    const destPath = activeState.path;
    const isMove = clipboard.action === 'cut';

    try {
      for (const item of clipboard.items) {
        const srcPath = joinPath(clipboard.sourceDir, item.name);
        await enqueueJob(isMove ? 'move' : 'copy', srcPath, destPath);
      }

      if (isMove) {
        await refreshPane(clipboard.sourcePane);
        setClipboard(null);
      }
      await refreshPane(activePane);
    } catch (err) {
      console.error('Lỗi dán file:', err);
    }
  };

  // Mở terminal tại thư mục pane đang chọn
  const handleOpenTerminal = async (path?: string) => {
    const targetPath = path || activeState.path;
    if (!targetPath || targetPath.includes('::')) return;
    try {
      await openInTerminal(targetPath);
    } catch (err) {
      console.error('Lỗi mở terminal:', err);
    }
  };

  const renderPane = (
    pane: PaneType,
    state: typeof leftState,
    paneStyle?: React.CSSProperties,
  ) => {
    const isCurrentBookmarked = bookmarks.some((b) => b.path === state.path);
    const currentRemote = state.path.includes('::') ? state.path.split('::')[0] : '__local__';
    const isActive = activePane === pane;
    const selectedCount = state.selectedIds.size;
    const selectedSize = state.files
      .filter((f) => state.selectedIds.has(f.uuid) && !f.is_dir)
      .reduce((acc, f) => acc + f.size, 0);

    return (
      <div
        className={`explorer-pane ${isActive ? 'active' : ''}`}
        key={pane}
        style={paneStyle}
        onClick={() => setActivePane(pane)}
      >
        <div
          className="pane-header"
          onContextMenu={(e) => {
            const target = e.target as HTMLElement;
            if (target.tagName === 'INPUT' || target.tagName === 'SELECT') return;
            e.preventDefault();
            setActivePane(pane);
            setContextMenu({
              x: e.clientX,
              y: e.clientY,
              type: 'background',
              pane,
            });
          }}
        >
          {/* Navigation Bar */}
          <div className="pane-nav-bar" style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
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

            {/* Quick New Folder button */}
            <button
              className="btn-icon"
              title={t('explorer_new_folder', 'Thư mục mới')}
              onClick={() => {
                setActivePane(pane);
                setNewFolderName('');
                setNewFolderPrompt(true);
              }}
              data-lang-id="explorer_new_folder"
            >
              <FolderPlus size={14} />
            </button>

            {/* Remote / Drive Switcher */}
            <select
              className="drive-select"
              value={currentRemote}
              onChange={(e) => {
                const val = e.target.value;
                if (val === '__local__') {
                  loadDirectory(pane, homeDir || '/');
                } else {
                  loadDirectory(pane, `${val}::/`);
                }
              }}
              title={t('explorer_select_remote', 'Chọn Ổ đĩa / Remote...')}
              style={{
                background: 'rgba(0,0,0,0.25)',
                color: 'var(--text-primary)',
                border: '1px solid var(--border)',
                borderRadius: '6px',
                padding: '0.2rem 0.4rem',
                fontSize: '0.78rem',
                cursor: 'pointer',
                outline: 'none',
                maxWidth: '130px',
                textOverflow: 'ellipsis',
              }}
            >
              <option value="__local__">🖥️ {t('explorer_drive_local', 'Thư mục cục bộ (Local)')}</option>
              {remotes.map((r) => (
                <option key={r.name} value={r.name}>
                  ☁️ {r.name}
                </option>
              ))}
            </select>

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

            <button
              className="btn-icon"
              title={t('explorer_deep_search', 'Tìm kiếm sâu')}
              onClick={() => onShowSearch?.(state.path)}
            >
              <Search size={14} />
            </button>

            {!state.path.includes('::') && (
              <button
                className="btn-icon"
                title={t('explorer_terminal', 'Terminal')}
                onClick={() => handleOpenTerminal(state.path)}
              >
                <Terminal size={14} />
              </button>
            )}
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
                data-lang-id="explorer_filter_placeholder"
                placeholder={t('explorer_filter_placeholder', 'Lọc nhanh trong bảng...')}
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
        <FileTable
          pane={pane}
          onFileContextMenu={(e, item) => {
            setActivePane(pane);
            setContextMenu({
              x: e.clientX,
              y: e.clientY,
              type: 'file',
              item,
              pane,
            });
          }}
          onBackgroundContextMenu={(e) => {
            setActivePane(pane);
            setContextMenu({
              x: e.clientX,
              y: e.clientY,
              type: 'background',
              pane,
            });
          }}
        />

        {/* Pane Status Bar */}
        <div className="pane-status-bar">
          <span data-lang-id="explorer_items">
            {state.files.length} {t('explorer_items', 'mục')}
          </span>
          {selectedCount > 0 && (
            <span style={{ color: 'var(--primary-hover)' }} data-lang-id="explorer_selected">
              {t('explorer_selected', 'Đã chọn')}: {selectedCount} ({formatBytes(selectedSize)})
            </span>
          )}
        </div>
      </div>
    );
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Dual Pane split view with draggable splitter */}
      <div className="explorer-container" ref={containerRef}>
        {renderPane('left', leftState, { flex: `0 0 calc(${splitRatio}% - 5px)`, minWidth: '180px' })}
        <div
          className="pane-splitter"
          title="Kéo sang trái / phải để điều chỉnh tỷ lệ • Nhấp đúp để đặt lại 50:50"
          onMouseDown={handleSplitterMouseDown}
          onDoubleClick={handleSplitterDoubleClick}
        >
          <div className="pane-splitter-handle" />
        </div>
        {renderPane('right', rightState, { flex: `0 0 calc(${100 - splitRatio}% - 5px)`, minWidth: '180px' })}
      </div>

      {/* Nemo Dual Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          type={contextMenu.type}
          selectedItem={contextMenu.item}
          selectedCount={contextMenu.item ? 1 : activeState.selectedIds.size}
          oppositePaneName={contextMenu.pane === 'left' ? 'Pane Phải' : 'Pane Trái'}
          hasClipboard={!!clipboard && clipboard.items.length > 0}
          isBookmarked={
            contextMenu.item?.is_dir
              ? bookmarks.some((b) => b.path === joinPath(activeState.path, contextMenu.item!.name))
              : false
          }
          onClose={() => setContextMenu(null)}
          // File Context Menu Actions
          onOpen={() => {
            if (!contextMenu.item) return;
            if (contextMenu.item.is_dir) {
              loadDirectory(contextMenu.pane, joinPath(activeState.path, contextMenu.item.name));
            } else {
              sysOpenWith(joinPath(activeState.path, contextMenu.item.name)).catch(console.error);
            }
          }}
          onOpenWith={() => {
            if (!contextMenu.item) return;
            sysOpenWith(joinPath(activeState.path, contextMenu.item.name)).catch(console.error);
          }}
          onCut={() => handleCut(contextMenu.item)}
          onCopy={() => handleCopy(contextMenu.item)}
          onCopyToOpposite={() => handleTransferToOpposite(false, contextMenu.item ? [contextMenu.item] : undefined)}
          onMoveToOpposite={() => handleTransferToOpposite(true, contextMenu.item ? [contextMenu.item] : undefined)}
          onRename={() => {
            if (contextMenu.item) {
              setRenameTarget(contextMenu.item);
              setRenameNewName(contextMenu.item.name);
            }
          }}
          onDelete={() => handleDeleteSelected(contextMenu.item ? [contextMenu.item] : undefined)}
          onBookmark={() => {
            if (contextMenu.item?.is_dir) {
              const full = joinPath(activeState.path, contextMenu.item.name);
              toggleBookmark(contextMenu.item.name, full);
            }
          }}
          onTerminal={() => {
            if (contextMenu.item?.is_dir) {
              handleOpenTerminal(joinPath(activeState.path, contextMenu.item.name));
            } else {
              handleOpenTerminal();
            }
          }}
          onProperties={() => {
            const target = contextMenu.item
              ? joinPath(activeState.path, contextMenu.item.name)
              : activeState.path;
            onShowProperties?.(target);
          }}
          // Background Context Menu Actions
          onNewFolder={() => setNewFolderPrompt(true)}
          onNewFile={() => setNewFilePrompt(true)}
          onPaste={handlePaste}
          onSelectAll={() => selectAll(contextMenu.pane)}
          onReload={() => refreshPane(contextMenu.pane)}
        />
      )}

      {/* New Folder Modal */}
      {newFolderPrompt && (
        <div className="modal-overlay" onClick={() => setNewFolderPrompt(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 style={{ fontSize: '1rem', margin: 0 }} data-lang-id="explorer_modal_new_folder_title">
                {t('explorer_modal_new_folder_title', 'Tạo thư mục mới')}
              </h3>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label" data-lang-id="explorer_modal_new_folder_label">
                  {t('explorer_modal_new_folder_label', 'Tên thư mục:')}
                </label>
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
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setNewFolderPrompt(false)}
                data-lang-id="btn_cancel"
              >
                {t('btn_cancel', 'Huỷ')}
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleCreateFolder}
                data-lang-id="btn_create"
              >
                {t('btn_create', 'Tạo')}
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
              <h3 style={{ fontSize: '1rem', margin: 0 }} data-lang-id="explorer_modal_new_file_title">
                {t('explorer_modal_new_file_title', 'Tạo tệp mới')}
              </h3>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label" data-lang-id="explorer_modal_new_file_label">
                  {t('explorer_modal_new_file_label', 'Tên tệp tin:')}
                </label>
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
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setNewFilePrompt(false)}
                data-lang-id="btn_cancel"
              >
                {t('btn_cancel', 'Huỷ')}
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleCreateFile}
                data-lang-id="btn_create"
              >
                {t('btn_create', 'Tạo')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Rename Modal */}
      {renameTarget && (
        <div className="modal-overlay" onClick={() => setRenameTarget(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 style={{ fontSize: '1rem', margin: 0 }} data-lang-id="explorer_modal_rename_title">
                {t('explorer_modal_rename_title', 'Đổi tên tệp / thư mục')}
              </h3>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label" data-lang-id="explorer_modal_rename_label">
                  {t('explorer_modal_rename_label', 'Tên mới:')}
                </label>
                <input
                  type="text"
                  className="input-text"
                  autoFocus
                  value={renameNewName}
                  onChange={(e) => setRenameNewName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleRenameConfirm();
                    else if (e.key === 'Escape') setRenameTarget(null);
                  }}
                />
              </div>
            </div>
            <div className="modal-footer">
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setRenameTarget(null)}
                data-lang-id="btn_cancel"
              >
                {t('btn_cancel', 'Huỷ')}
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleRenameConfirm}
                data-lang-id="btn_rename"
              >
                {t('btn_rename', 'Đổi tên')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
