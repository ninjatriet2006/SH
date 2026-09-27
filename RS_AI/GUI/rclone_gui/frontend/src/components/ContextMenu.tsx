/*
[INTEGRITY NOTES]
- Mục đích: Bảng menu chuột phải (Context Menu) theo phong cách Nemo (Cinnamon File Manager).
- Trách nhiệm: Hỗ trợ 2 chế độ:
  1. File Context Menu (khi nhấp chuột phải lên file/thư mục đã chọn): Mở, Mở bằng, Cắt, Copy, Chép/Chuyển sang khung đối diện, Đổi tên, Xoá, Ghim, Terminal, Thuộc tính.
  2. Background Context Menu (khi nhấp chuột phải vào khoảng trống): Tạo thư mục, Tạo tệp, Dán, Chọn tất cả, Làm mới, Terminal, Thuộc tính.
- Tự động giới hạn vị trí không bị tràn khỏi màn hình (viewport collision detection).
*/

import {
  ArrowLeftRight,
  Bookmark,
  Copy,
  CornerDownRight,
  ExternalLink,
  FilePlus,
  FolderPlus,
  Info,
  MoveRight,
  Pencil,
  RefreshCw,
  Scissors,
  Terminal,
  Trash2,
} from 'lucide-react';
import React, { useEffect, useRef } from 'react';
import type { FileItem } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

export interface ContextMenuProps {
  x: number;
  y: number;
  type: 'file' | 'background';
  selectedItem?: FileItem | null;
  selectedCount?: number;
  oppositePaneName?: string;
  hasClipboard?: boolean;
  isBookmarked?: boolean;
  onClose: () => void;

  // File actions
  onOpen?: () => void;
  onOpenWith?: () => void;
  onCut?: () => void;
  onCopy?: () => void;
  onCopyToOpposite?: () => void;
  onMoveToOpposite?: () => void;
  onRename?: () => void;
  onDelete?: () => void;
  onBookmark?: () => void;
  onTerminal?: () => void;
  onProperties?: () => void;

  // Background actions
  onNewFolder?: () => void;
  onNewFile?: () => void;
  onPaste?: () => void;
  onSelectAll?: () => void;
  onReload?: () => void;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({
  x,
  y,
  type,
  selectedItem,
  selectedCount = 1,
  oppositePaneName = 'Khung đối diện',
  hasClipboard = false,
  isBookmarked = false,
  onClose,
  onOpen,
  onOpenWith,
  onCut,
  onCopy,
  onCopyToOpposite,
  onMoveToOpposite,
  onRename,
  onDelete,
  onBookmark,
  onTerminal,
  onProperties,
  onNewFolder,
  onNewFile,
  onPaste,
  onSelectAll,
  onReload,
}) => {
  const { t } = useTranslation();
  const menuRef = useRef<HTMLDivElement>(null);

  // Đóng khi click ngoài hoặc bấm phím Escape
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('mousedown', handleOutsideClick);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousedown', handleOutsideClick);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  // Giới hạn tọa độ trong viewport
  const menuWidth = 240;
  const menuEstimatedHeight = type === 'file' ? 380 : 250;
  const posX = Math.max(10, Math.min(x, window.innerWidth - menuWidth - 10));
  const posY = Math.max(10, Math.min(y, window.innerHeight - menuEstimatedHeight - 10));

  const handleAction = (callback?: () => void) => {
    if (callback) {
      callback();
    }
    onClose();
  };

  return (
    <div
      ref={menuRef}
      className="nemo-context-menu"
      style={{
        position: 'fixed',
        left: `${posX}px`,
        top: `${posY}px`,
        width: `${menuWidth}px`,
        zIndex: 9999,
        background: 'var(--bg-panel)',
        backdropFilter: 'blur(16px)',
        border: '1px solid var(--border)',
        borderRadius: '8px',
        boxShadow: '0 12px 32px rgba(0, 0, 0, 0.55), 0 0 1px 1px var(--border)',
        padding: '0.35rem',
        userSelect: 'none',
        animation: 'contextMenuFadeIn 0.12s cubic-bezier(0.16, 1, 0.3, 1)',
      }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {type === 'file' ? (
        // ===== BẢNG CHỌN 1: FILE / ITEM CONTEXT MENU =====
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          {/* Mở */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onOpen)}
            data-lang-id="ctx_open"
          >
            <div className="item-left">
              <CornerDownRight size={14} />
              <span>{t('ctx_open', 'Mở')}</span>
            </div>
            <span className="item-hotkey">Enter</span>
          </button>

          {/* Mở bằng ứng dụng khác */}
          {selectedItem && !selectedItem.is_dir && (
            <button
              className="context-menu-item"
              onClick={() => handleAction(onOpenWith)}
              data-lang-id="ctx_open_with"
            >
              <div className="item-left">
                <ExternalLink size={14} />
                <span>{t('ctx_open_with', 'Mở bằng ứng dụng khác...')}</span>
              </div>
            </button>
          )}

          <div className="context-menu-divider" />

          {/* Cắt */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onCut)}
            data-lang-id="ctx_cut"
          >
            <div className="item-left">
              <Scissors size={14} />
              <span>{t('ctx_cut', 'Cắt')}</span>
            </div>
            <span className="item-hotkey">Ctrl+X</span>
          </button>

          {/* Sao chép */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onCopy)}
            data-lang-id="ctx_copy"
          >
            <div className="item-left">
              <Copy size={14} />
              <span>{t('ctx_copy', 'Sao chép')}</span>
            </div>
            <span className="item-hotkey">Ctrl+C</span>
          </button>

          {/* Chép sang khung đối diện */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onCopyToOpposite)}
            data-lang-id="ctx_copy_to_opposite"
          >
            <div className="item-left">
              <ArrowLeftRight size={14} />
              <span>{t('ctx_copy_to_opposite', `Chép sang ${oppositePaneName}`)}</span>
            </div>
          </button>

          {/* Chuyển sang khung đối diện */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onMoveToOpposite)}
            data-lang-id="ctx_move_to_opposite"
          >
            <div className="item-left">
              <MoveRight size={14} />
              <span>{t('ctx_move_to_opposite', `Chuyển sang ${oppositePaneName}`)}</span>
            </div>
          </button>

          <div className="context-menu-divider" />

          {/* Đổi tên */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onRename)}
            data-lang-id="ctx_rename"
          >
            <div className="item-left">
              <Pencil size={14} />
              <span>{t('ctx_rename', 'Đổi tên')}</span>
            </div>
            <span className="item-hotkey">F2</span>
          </button>

          {/* Ghim thư mục */}
          {selectedItem?.is_dir && (
            <button
              className="context-menu-item"
              onClick={() => handleAction(onBookmark)}
              data-lang-id={isBookmarked ? 'ctx_unbookmark' : 'ctx_bookmark'}
            >
              <div className="item-left">
                <Bookmark size={14} color={isBookmarked ? 'var(--primary)' : 'currentColor'} />
                <span>
                  {isBookmarked
                    ? t('ctx_unbookmark', 'Bỏ ghim thư mục')
                    : t('ctx_bookmark', 'Ghim thư mục')}
                </span>
              </div>
            </button>
          )}

          {/* Xoá */}
          <button
            className="context-menu-item item-danger"
            onClick={() => handleAction(onDelete)}
            data-lang-id="ctx_delete"
          >
            <div className="item-left">
              <Trash2 size={14} />
              <span>
                {selectedCount > 1
                  ? `${t('ctx_delete', 'Xoá')} (${selectedCount})`
                  : t('ctx_delete', 'Xoá / Thùng rác')}
              </span>
            </div>
            <span className="item-hotkey">Del</span>
          </button>

          <div className="context-menu-divider" />

          {/* Mở trong Terminal */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onTerminal)}
            data-lang-id="ctx_open_terminal"
          >
            <div className="item-left">
              <Terminal size={14} />
              <span>{t('ctx_open_terminal', 'Mở trong Terminal')}</span>
            </div>
          </button>

          {/* Thuộc tính */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onProperties)}
            data-lang-id="ctx_properties"
          >
            <div className="item-left">
              <Info size={14} />
              <span>{t('ctx_properties', 'Thuộc tính')}</span>
            </div>
            <span className="item-hotkey">Alt+Enter</span>
          </button>
        </div>
      ) : (
        // ===== BẢNG CHỌN 2: BACKGROUND / BLANK SPACE CONTEXT MENU =====
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
          {/* Thư mục mới */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onNewFolder)}
            data-lang-id="ctx_new_folder"
          >
            <div className="item-left">
              <FolderPlus size={14} />
              <span>{t('ctx_new_folder', 'Thư mục mới')}</span>
            </div>
            <span className="item-hotkey">Ctrl+Shift+N</span>
          </button>

          {/* Tệp mới */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onNewFile)}
            data-lang-id="ctx_new_file"
          >
            <div className="item-left">
              <FilePlus size={14} />
              <span>{t('ctx_new_file', 'Tệp mới')}</span>
            </div>
          </button>

          <div className="context-menu-divider" />

          {/* Dán */}
          <button
            className="context-menu-item"
            disabled={!hasClipboard}
            onClick={() => handleAction(onPaste)}
            data-lang-id="ctx_paste"
          >
            <div className="item-left">
              <Copy size={14} style={{ opacity: hasClipboard ? 1 : 0.4 }} />
              <span style={{ opacity: hasClipboard ? 1 : 0.4 }}>{t('ctx_paste', 'Dán')}</span>
            </div>
            <span className="item-hotkey">Ctrl+V</span>
          </button>

          {/* Chọn tất cả */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onSelectAll)}
            data-lang-id="ctx_select_all"
          >
            <div className="item-left">
              <CornerDownRight size={14} />
              <span>{t('ctx_select_all', 'Chọn tất cả')}</span>
            </div>
            <span className="item-hotkey">Ctrl+A</span>
          </button>

          <div className="context-menu-divider" />

          {/* Nạp lại / Làm mới */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onReload)}
            data-lang-id="ctx_reload"
          >
            <div className="item-left">
              <RefreshCw size={14} />
              <span>{t('ctx_reload', 'Làm mới')}</span>
            </div>
            <span className="item-hotkey">F5</span>
          </button>

          {/* Mở trong Terminal */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onTerminal)}
            data-lang-id="ctx_open_terminal"
          >
            <div className="item-left">
              <Terminal size={14} />
              <span>{t('ctx_open_terminal', 'Mở trong Terminal')}</span>
            </div>
          </button>

          <div className="context-menu-divider" />

          {/* Thuộc tính thư mục */}
          <button
            className="context-menu-item"
            onClick={() => handleAction(onProperties)}
            data-lang-id="ctx_properties"
          >
            <div className="item-left">
              <Info size={14} />
              <span>{t('ctx_properties', 'Thuộc tính thư mục')}</span>
            </div>
          </button>
        </div>
      )}
    </div>
  );
};
