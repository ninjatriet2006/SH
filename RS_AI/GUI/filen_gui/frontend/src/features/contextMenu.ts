// features/contextMenu.ts — Xây dựng context menu cho 1 file và cho vùng trống.
// Nhận FileItem + path + pane, gọi fileOps, refresh qua callback.
import type { FileItem } from '../store';
import { logActivity, isBookmarked, toggleBookmark } from '../store';
import { getPaneSelection, type Pane } from '../services/explorerStore';
import { ContextMenu, type ContextMenuItem } from '../components/ContextMenu';
import { OpenWithModal } from '../components/OpenWithModal';
import { OperationModal } from '../components/OperationModal';
import { PropertiesModal } from '../components/PropertiesModal';
import { BatchRenameModal, type BatchRenameItem } from '../components/BatchRenameModal';
import * as fileOps from '../services/fileOps';
import * as trashOps from '../services/trashOps';
import { setClipboard, pasteTo } from './clipboard';
import { actionStore } from '../services/actionStore';

import { appState } from '../store';

export interface ContextMenuOptions {
  file: FileItem;
  /** Đường dẫn thư mục chứa file (base path). */
  path: string;
  pane: Pane;
  /** Gọi lại sau khi thao tác làm thay đổi danh sách (rename/delete). */
  onRefresh: (pane: Pane, path: string) => Promise<void>;
  /** Select All — chọn toàn bộ file trong pane. */
  onSelectAll: () => void;
  /** Mở folder trong tab mới */
  onOpenInNewTab?: (path: string) => void;
}

export interface FolderContextMenuOptions {
  /** Đường dẫn thư mục hiện tại. */
  path: string;
  pane: Pane;
  onRefresh: (pane: Pane, path: string) => Promise<void>;
  /** Select All — chọn toàn bộ file trong pane. */
  onSelectAll: () => void;
}

function joinPath(dir: string, name: string): string {
  return dir.endsWith('/') ? dir + name : dir + '/' + name;
}

/** Dựng menu tại vị trí chuột; onClick nhận label mục được chọn. */
function showMenu(
  e: MouseEvent,
  items: (string | ContextMenuItem)[],
  onClick: (action: string) => void,
): void {
  // Dọn dẹp menu cũ nếu có
  document.querySelectorAll('.context-menu').forEach((el) => el.remove());

  const menu = new ContextMenu(items);
  document.body.appendChild(menu.getElement());
  menu.getElement().style.top = `${e.clientY}px`;
  menu.getElement().style.left = `${e.clientX}px`;

  // Lắng nghe click chọn item
  menu.getElement().addEventListener('click', (ev) => {
    const target = ev.target as HTMLElement;
    if (!target.classList.contains('item') || target.classList.contains('disabled')) return;
    onClick(target.textContent?.trim() ?? '');
    menu.getElement().remove();
  });

  // Đóng menu khi click chuột ra ngoài hoặc right-click chỗ khác
  const closeMenu = (ev: Event) => {
    if (!menu.getElement().contains(ev.target as Node)) {
      menu.getElement().remove();
      document.removeEventListener('pointerdown', closeMenu);
      document.removeEventListener('contextmenu', closeMenu);
    }
  };

  setTimeout(() => {
    document.addEventListener('pointerdown', closeMenu);
    document.addEventListener('contextmenu', closeMenu);
  }, 0);
}

/** Hiện context menu tại vị trí chuột cho file. */
export function MenuFile(e: MouseEvent, opts: ContextMenuOptions): void {
  const { file: f, path: basePath, pane, onRefresh } = opts;
  const fullPath = joinPath(basePath, f.name);

  const sels = getPaneSelection(pane);
  const isSelected = sels.some((s) => s.name === f.name);
  const batchMode = isSelected && sels.length > 1;

  const isTrash = basePath.startsWith('trash://');
  if (isTrash) {
    showMenu(e, ['Khôi phục', 'Xoá vĩnh viễn'], (action) => {
      handleTrashAction(action, f, basePath, pane, onRefresh);
    });
    return;
  }

  const menuItems: (string | ContextMenuItem)[] = [
    'Open',
    'Open With...',
    'New Folder',
    'New File',
    batchMode ? `Rename (${sels.length} mục)` : 'Rename',
    'Delete',
  ];
  if (f.is_dir && !batchMode) {
    if (opts.onOpenInNewTab) menuItems.push('Open in New Tab');
    menuItems.push(isBookmarked(fullPath) ? '❌ Bỏ ghim Sidebar' : '📌 Ghim vào Sidebar');
    if (pane === 'left') {
      menuItems.push('Open in Terminal');
    }
  }
  menuItems.push(
    'Copy',
    'Cut',
    'Paste',
    'Properties'
  );

  let selectedFiles: FileItem[] = [];
  if (batchMode) {
    selectedFiles = appState.explorer?.[`${pane}Files`]?.filter(ff => sels.some(s => s.name === ff.name)) || [f];
  } else {
    selectedFiles = [f];
  }
  
  const customActions = actionStore.getValidActionsForSelection(selectedFiles);
  if (customActions.length > 0) {
    menuItems.push({ separator: true });
    customActions.forEach(a => {
      menuItems.push(a.name);
    });
  }

  showMenu(
    e,
    menuItems,
    (action) => {
      switch (action) {
        case 'Open':
          fileOps.open(fullPath).catch((err) => console.warn('open fail:', err));
          break;
        case 'Open in New Tab':
          if (opts.onOpenInNewTab) opts.onOpenInNewTab(fullPath);
          break;
        case 'Open With...':
          new OpenWithModal(fullPath).open();
          break;
        case 'Open in Terminal':
          import('@tauri-apps/api/core').then(({ invoke }) => {
            invoke('open_in_terminal', { path: fullPath })
              .catch((err) => console.error('Open terminal failed:', err));
          });
          break;
        case 'New Folder':
          promptName('New Folder', 'folder name', (name) => pane === 'left' ? fileOps.mkdirLocal(joinPath(basePath, name)) : fileOps.mkdir(joinPath(basePath, name)), pane, basePath, onRefresh);
          break;
        case 'New File':
          promptName('New File', 'file name', (name) => fileOps.write(joinPath(basePath, name), ''), pane, basePath, onRefresh);
          break;
        case 'Rename':
          openRenameModal(f, fullPath, pane, basePath, onRefresh);
          break;
        case `Rename (${sels.length} mục)`:
          if (batchMode) {
            const renameItems: BatchRenameItem[] = sels.map(s => ({ pane: s.pane, path: s.path, name: s.name }));
            new BatchRenameModal(renameItems, basePath, onRefresh).open();
          }
          break;
        case 'Delete':
          openDeleteModal(f, fullPath, pane, basePath, onRefresh);
          break;
        case 'Copy':
          setClipboard('copy', [{ pane, path: fullPath }]);
          break;
        case 'Cut':
          setClipboard('cut', [{ pane, path: fullPath }]);
          break;
        case 'Paste':
          pasteTo(pane, basePath, onRefresh);
          break;
        case 'Properties':
          showPropertiesModal(f, fullPath, pane);
          break;
        case '📌 Ghim vào Sidebar':
        case '❌ Bỏ ghim Sidebar':
          toggleBookmark(f.name, fullPath);
          break;
        default:
          const customAction = customActions.find(a => a.name === action);
          if (customAction) {
            actionStore.executeAction(customAction, selectedFiles, basePath);
          }
          break;
      }
    },
  );
}

/** Hiện context menu trên vùng trống của pane (menu thư mục). */
export function MenuEmpty(e: MouseEvent, opts: FolderContextMenuOptions): void {
  const { path: basePath, pane, onRefresh, onSelectAll } = opts;

  const items: (string | ContextMenuItem)[] = [
    'New Folder',
    'New File',
    'Paste',
    'Select All',
  ];

  if (pane === 'left') {
    items.push('Open in Terminal');
  }

  showMenu(
    e,
    items,
    (action) => {
      switch (action) {
        case 'New Folder':
          promptName('New Folder', 'folder name', (name) => pane === 'left' ? fileOps.mkdirLocal(joinPath(basePath, name)) : fileOps.mkdir(joinPath(basePath, name)), pane, basePath, onRefresh);
          break;
        case 'New File':
          promptName('New File', 'file name', (name) => pane === 'left' ? fileOps.writeLocal(joinPath(basePath, name), '') : fileOps.write(joinPath(basePath, name), ''), pane, basePath, onRefresh);
          break;
        case 'Paste':
          pasteTo(pane, basePath, onRefresh);
          break;
        case 'Select All':
          onSelectAll();
          break;
        case 'Open in Terminal':
          import('@tauri-apps/api/core').then(({ invoke }) => {
            invoke('open_in_terminal', { path: basePath })
              .catch((err) => console.error('Open terminal failed:', err));
          });
          break;
        default:
          break;
      }
    },
  );
}

/** Hỏi tên (New Folder / New File) rồi chạy action, refresh sau khi xong. */
function promptName(
  title: string,
  placeholder: string,
  action: (name: string) => Promise<void>,
  pane: Pane,
  basePath: string,
  onRefresh: ContextMenuOptions['onRefresh'],
): void {
  const modal = new OperationModal(
    title,
    `<p>${title}:</p><input id="newName" type="text" placeholder="${placeholder}">`,
  );
  modal.open();
  const confirmBtn = modal.getElement().querySelector('.confirm') as HTMLButtonElement;
  const input = modal.getElement().querySelector('#newName') as HTMLInputElement;

  const handleConfirm = async () => {
    const name = input?.value?.trim();
    if (name) {
      try {
        await action(name);
        logActivity(title, `Thành công: ${name}`);
      } catch (err) {
        console.warn(`${title} fail:`, err);
        logActivity(`Lỗi ${title}`, `Chi tiết: ${err}`);
      }
    }
    modal.close();
    await onRefresh(pane, basePath);
  };

  confirmBtn?.addEventListener('click', handleConfirm);
  input?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    }
  });

  if (input) {
    setTimeout(() => input.focus(), 0);
  }
}

/** Modal Properties nâng cao: đếm đệ quy, phân quyền chmod */
async function showPropertiesModal(f: FileItem, fullPath: string, pane: Pane): Promise<void> {
  const modal = new PropertiesModal(f, fullPath, pane);
  await modal.open();
}

function openRenameModal(
  f: FileItem,
  fullPath: string,
  pane: Pane,
  basePath: string,
  onRefresh: ContextMenuOptions['onRefresh'],
): void {
  const modal = new OperationModal(
    'Rename',
    `<p>Rename ${f.name}:</p><input id="newName" type="text" value="${f.name}">`,
  );
  modal.open();
  const confirmBtn = modal.getElement().querySelector('.confirm') as HTMLButtonElement;
  const input = modal.getElement().querySelector('#newName') as HTMLInputElement;

  const handleConfirm = async () => {
    const newName = input?.value?.trim();
    if (newName) {
      try {
        if (pane === 'left') {
          await fileOps.renameLocal(fullPath, newName);
        } else {
          await fileOps.rename(fullPath, newName, appState.auth?.user);
        }
        logActivity('Đổi tên', `Từ ${f.name} thành ${newName}`);
      } catch (err) {
        console.warn('rename fail:', err);
        logActivity('Lỗi Đổi tên', `Chi tiết: ${err}`);
      }
    }
    modal.close();
    await onRefresh(pane, basePath);
  };

  confirmBtn?.addEventListener('click', handleConfirm);
  input?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    }
  });

  if (input) {
    setTimeout(() => {
      input.focus();
      input.select();
    }, 0);
  }
}

function openDeleteModal(
  f: FileItem,
  fullPath: string,
  pane: Pane,
  basePath: string,
  onRefresh: ContextMenuOptions['onRefresh'],
): void {
  const modal = new OperationModal('Delete', `<p>Delete ${f.name}?</p>`);
  modal.open();
  modal.getElement().querySelector('.confirm')?.addEventListener('click', async () => {
    try {
      if (pane === 'left') {
        await fileOps.rmLocal(fullPath);
      } else {
        await fileOps.remove(fullPath, appState.auth?.user);
      }
      logActivity('Xoá', `Đã xoá ${fullPath}`);
    } catch (err) {
      console.warn('delete fail:', err);
      logActivity('Lỗi Xoá', `Chi tiết: ${err}`);
    }
    modal.close();
    await onRefresh(pane, basePath);
  });
}

async function handleTrashAction(action: string, f: FileItem, basePath: string, pane: Pane, onRefresh: (pane: Pane, path: string) => Promise<void>) {
  try {
    if (action === 'Khôi phục') {
      if (basePath === 'trash://local') {
        const id = (f as any).path ? (f as any).path.split('/').pop() || '' : '';
        await trashOps.restoreLocalTrash(id);
      } else {
        const files = pane === 'left' ? appState.explorer?.leftFiles : appState.explorer?.rightFiles;
        const idx = (files?.findIndex(item => item.name === f.name) ?? -1) + 1;
        if (idx > 0) await trashOps.restoreRemoteTrash(idx);
      }
    } else if (action === 'Xoá vĩnh viễn') {
      if (basePath === 'trash://local') {
        alert('Tính năng xoá từng file cục bộ chưa hoàn thiện. Vui lòng làm trống toàn bộ thùng rác.');
        return;
      } else {
        const files = pane === 'left' ? appState.explorer?.leftFiles : appState.explorer?.rightFiles;
        const idx = (files?.findIndex(item => item.name === f.name) ?? -1) + 1;
        if (idx > 0) await trashOps.deleteRemoteTrash(idx);
      }
    }
    onRefresh(pane, basePath);
  } catch (e) {
    console.warn('Trash action error:', e);
    alert('Lỗi: ' + String(e));
  }
}