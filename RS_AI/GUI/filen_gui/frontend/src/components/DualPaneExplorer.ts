import { invoke, Channel } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { appState, logActivity } from '../store';
import type { FileItem } from '../store';
import { OperationModal } from './OperationModal';
import { PaneContainer } from './pane/PaneContainer';
import { MenuFile, MenuEmpty } from '../features/contextMenu';
import { sortFiles } from '../features/sort';
import type { SortKey } from '../features/sort';
import type { Pane, PaneColKey } from '../services/explorerStore';
import {
  getPaneColWidths,
  getPaneSortDir,
  getPaneSortKey,
  setPaneColWidth,
  setPaneSort,
} from '../services/explorerStore';
import {
  setActivePane,
  setPaneSelection,
  clearPaneSelection,
  getActivePane,
  getPaneSelection,
  getPanePath,
  pushPaneHistory,
  popPaneBack,
  popPaneForward,
  canPaneGoBack,
  canPaneGoForward,
} from '../services/explorerStore';
import { setClipboard, hasClipboard, pasteTo } from '../features/clipboard';
import * as fileOps from '../services/fileOps';
import { undoManager } from '../services/undoManager';
import { baseName, joinPath, generateUniqueName, type DragPayload } from '../features/dragDrop';
import { ConflictModal, type ConflictResult } from './ConflictModal';

/**
 * Orchestrator: tạo 2 PaneView + divider, load dữ liệu, xử lý command.
 * Mọi render UI (header/breadcrumb/table/toolbar) nằm trong pane/*.
 */
export class DualPaneExplorer {
  container: HTMLDivElement;
  private leftPane: PaneContainer;
  private rightPane: PaneContainer;

  constructor() {
    this.container = document.createElement('div');
    this.container.className = 'dual-pane-explorer';

    this.leftPane = new PaneContainer({
      side: 'left',
      sideLabel: '🖥️ Cục bộ',
      onOpenDir: (path) => this.navigate('left', path),
      onOpenInNewTab: (path) => this.leftPane.addTab(path),
      onSelectRow: (file) => this.handleSelectRow('left', file),
      onActivate: () => setActivePane('left'),
      onContextMenu: (e, file) => this.handleContextMenu('left', e, file),
      onContextMenuEmpty: (e) => this.handleFolderContextMenu('left', e),
      onFilter: () => { /* filter chưa active — giữ hành vi hiện tại */ },
      onMkdir: (name) => this.handleMkdir('left', name),
      onSort: (key) => this.handleSort('left', key),
      onColResize: (key, width) => this.handleColResize('left', key, width),
      onDrop: (payload, destPath, move) => this.handleDrop('left', payload, destPath, move),
      onBack: () => this.goBack('left'),
      onForward: () => this.goForward('left'),
      onUp: () => this.goUp('left'),
      onHome: () => this.goHome('left'),
      onRefresh: () => this.refresh('left'),
    });
    this.leftPane.onTabSwitch = (path) => this.loadPane('left', path);

    this.rightPane = new PaneContainer({
      side: 'right',
      sideLabel: '☁️ Đám mây',
      onOpenDir: (path) => this.navigate('right', path),
      onOpenInNewTab: (path) => this.rightPane.addTab(path),
      onSelectRow: (file) => this.handleSelectRow('right', file),
      onActivate: () => setActivePane('right'),
      onContextMenu: (e, file) => this.handleContextMenu('right', e, file),
      onContextMenuEmpty: (e) => this.handleFolderContextMenu('right', e),
      onFilter: () => { /* filter chưa active — giữ hành vi hiện tại */ },
      onMkdir: (name) => this.handleMkdir('right', name),
      onSort: (key) => this.handleSort('right', key),
      onColResize: (key, width) => this.handleColResize('right', key, width),
      onDrop: (payload, destPath, move) => this.handleDrop('right', payload, destPath, move),
      onBack: () => this.goBack('right'),
      onForward: () => this.goForward('right'),
      onUp: () => this.goUp('right'),
      onHome: () => this.goHome('right'),
      onRefresh: () => this.refresh('right'),
    });
    this.rightPane.onTabSwitch = (path) => this.loadPane('right', path);

    const divider = document.createElement('div');
    divider.className = 'pane-divider';

    // Drag to resize logic
    let isResizing = false;
    divider.addEventListener('mousedown', (e) => {
      isResizing = true;
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      e.preventDefault();
    });

    document.addEventListener('mousemove', (e) => {
      if (!isResizing) return;
      const containerRect = this.container.getBoundingClientRect();
      const leftWidth = e.clientX - containerRect.left;
      
      // Set bounds (min 200px per pane)
      const minWidth = 200;
      const maxWidth = containerRect.width - minWidth;
      
      if (leftWidth > minWidth && leftWidth < maxWidth) {
        this.container.style.gridTemplateColumns = `${leftWidth}px 6px 1fr`;
      }
    });

    document.addEventListener('mouseup', () => {
      if (isResizing) {
        isResizing = false;
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }
    });

    this.container.appendChild(this.leftPane.getElement());
    this.container.appendChild(divider);
    this.container.appendChild(this.rightPane.getElement());

    // Listen for local directory changes (inotify)
    listen('local-dir-changed', () => {
      const leftPath = appState.explorer?.leftPath;
      if (leftPath) {
        // Reload without touching the selection
        this.loadPane('left', leftPath);
      }
    });

    this.init();
  }

  /** Ctrl+C / Ctrl+X / Ctrl+V trên pane active. */
  private handleKeyDown = (e: KeyboardEvent) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    const pane = getActivePane();

    if (key === 'c' || key === 'x') {
      const sels = getPaneSelection(pane);
      if (sels.length === 0) return;
      if (pane === 'right') {
        console.warn('Copy/Cut chưa hỗ trợ trên cloud pane');
        return;
      }
      e.preventDefault();
      setClipboard(key === 'c' ? 'copy' : 'cut', sels.map((s) => ({ pane, path: s.path })));
    } else if (key === 'v') {
      if (!hasClipboard()) return;
      e.preventDefault();
      const destPath = getPanePath(pane);
      pasteTo(pane, destPath, (p, pth) => this.loadPane(p, pth));
    }      // Ctrl+T: New Tab
      if (key === 't') {
        e.preventDefault();
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        activeContainer.addTab(pane === 'left' ? '/' : 'trash://remote');
        return;
      }
      
      // Ctrl+W: Close Tab
      if (key === 'w') {
        e.preventDefault();
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        if (activeContainer.activeTabId) {
          activeContainer.closeTab(activeContainer.activeTabId);
        }
        return;
      }

      if (key === 'a') {
      e.preventDefault();
      this.selectAll(pane);
    } else if (key === 'z') {
      e.preventDefault();
      if (e.shiftKey) {
        undoManager.redo();
      } else {
        undoManager.undo();
      }
    } else if (key === 'y') {
      e.preventDefault();
      undoManager.redo();
    }
  };

  async init() {
    window.addEventListener('keydown', this.handleKeyDown);
    
    let leftPath = appState.explorer?.leftPath;
    if (!leftPath || leftPath === '/') {
      try {
        const { desktopDir } = await import('@tauri-apps/api/path');
        leftPath = await desktopDir();
      } catch (e) {
        console.warn('Could not get desktop dir', e);
        try {
          const { homeDir } = await import('@tauri-apps/api/path');
          leftPath = await homeDir();
        } catch (e2) {
          leftPath = '/';
        }
      }
    }
    
    const rightPath = appState.explorer?.rightPath ?? '/';
    await Promise.all([
      this.loadPane('left', leftPath),
      this.loadPane('right', rightPath),
    ]);
  }

  async loadPane(pane: 'left' | 'right', path: string) {
    // Cloud pane khi chưa đăng nhập → hiện placeholder ngay, không gọi command
    // (fs_list_remote trả [] với exit 0 khi chưa đăng nhập → tránh render bảng rỗng).
    if (pane === 'right' && !appState.auth?.user) {
      this.rightPane.renderPlaceholder(`🔑 Chưa đăng nhập — bấm "Đăng nhập mới" ở sidebar để xem Filen Cloud`, path);
      return;
    }


    
    // Path thay đổi → bỏ selection cũ của pane này.
    const currentPath = pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath;
    if (currentPath !== path) {
      clearPaneSelection(pane);
    }
    
    // Hiện thông báo đang tải trước khi gọi lệnh
    if (pane === 'right') {
      this.rightPane.renderPlaceholder(`⏳ Đang tải dữ liệu từ Filen Cloud...`, path);
    } else {
      this.leftPane.renderPlaceholder(`⏳ Đang đọc thư mục...`, path);
    }

    let files: FileItem[] = [];
    try {
      if (path === 'trash://remote') {
        files = await invoke('fs_trash_list_remote_terminal', { account: undefined }) as any;
      } else if (path === 'trash://local') {
        files = await invoke('fs_trash_list_local') as any;
      } else if (pane === 'right') {
        const onChunk = new Channel<FileItem[]>();
        
        // Cờ đánh dấu đã nhận chunk đầu tiên chưa để chuyển từ "Loading..." sang bảng
        let firstChunk = true;
        
        onChunk.onmessage = (chunk: FileItem[]) => {
          files = files.concat(chunk);
          
          if (!appState.explorer) appState.explorer = {};
          appState.explorer.rightPath = path;
          appState.explorer.rightFiles = files;
          
          if (firstChunk) {
            firstChunk = false;
            this.renderPane('right');
          } else {
            if (this.rightPane.table) {
               this.rightPane.table.appendFiles(chunk);
            }
          }
        };
        
        await invoke('fs_list_remote_stream_terminal', { account: undefined, path, onChunk });
        console.log('fs_list_remote_stream finished for', path, 'total:', files.length);
      } else {
        files = await invoke('fs_list_local', { path }) as any;
        console.log('fs_list_local result for', path, files);
      }
    } catch (e) {
      console.warn(`fs_list ${pane} fail (${path}):`, e);
      // Cloud pane khi chưa đăng nhập → hiện placeholder thay vì crash
      if (pane === 'right') {
        this.rightPane.renderPlaceholder(`🔑 Chưa đăng nhập — bấm "Đăng nhập mới" ở sidebar để xem Filen Cloud`, path);
        return;
      }
      this.leftPane.renderPlaceholder(`⚠️ Không đọc được thư mục: ${String(e)}`, path, {
        label: '⬅ Quay lại',
        onClick: () => this.goBack('left')
      });
      this.leftPane.toolbar?.updateHistoryState(canPaneGoBack('left'), canPaneGoForward('left'));
      return;
    }

    sortFiles(files, getPaneSortKey(pane), true, getPaneSortDir(pane));
    
    if (pane === 'left') {
      if (!appState.explorer) appState.explorer = {};
      appState.explorer.leftPath = path;
      appState.explorer.leftFiles = files;
    } else {
      if (!appState.explorer) appState.explorer = {};
      appState.explorer.rightPath = path;
      appState.explorer.rightFiles = files;
    }
    this.renderPane(pane);
  }

  renderPane(pane: 'left' | 'right') {
    const view = pane === 'left' ? this.leftPane : this.rightPane;
    const files = pane === 'left' ? appState.explorer?.leftFiles : appState.explorer?.rightFiles;
    const path = pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath;
    const sortKey = getPaneSortKey(pane);
    const sortDir = getPaneSortDir(pane);
    const colWidths = getPaneColWidths(pane);
    view.render(
      sortFiles(files ?? [], sortKey, true, sortDir),
      path ?? '/',
      { key: sortKey, dir: sortDir },
      colWidths
    );
    view.toolbar?.updateHistoryState(canPaneGoBack(pane), canPaneGoForward(pane));
  }

  /** Click tiêu đề cột → sort; cùng cột thì toggle asc/desc. */
  private handleSort(pane: Pane, key: SortKey) {
    const curKey = getPaneSortKey(pane);
    const curDir = getPaneSortDir(pane);
    const nextDir = curKey === key ? (curDir === 'asc' ? 'desc' : 'asc') : 'asc';
    setPaneSort(pane, key, nextDir);
    this.renderPane(pane);
  }

  /** Kéo resize handle → lưu width cột cho pane. */
  private handleColResize(pane: Pane, key: PaneColKey, width: number) {
    setPaneColWidth(pane, key, width);
  }

  private async handleMkdir(pane: 'left' | 'right', name: string) {
    const path = (pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath) ?? '/';
    await invoke('fs_mkdir_terminal', { path: path + '/' + name });
    await this.loadPane(pane, path);
  }

  /**
   * Drop file vào pane đích. Mặc định COPY; giữ Ctrl → MOVE.
   * - Đích local (left): copy → fs_cp_batch; move → fs_mv_local từng item.
   * - Đích cloud (right): fs_upload từng item.
   * Sau khi xong refresh pane đích (+ pane nguồn nếu move).
   */
  private async handleDrop(
    destPane: Pane,
    payload: DragPayload,
    destPath: string,
    move: boolean,
  ): Promise<void> {
    const srcs = payload.paths;
    if (srcs.length === 0) return;
    
    const actionText = move ? 'Di chuyển' : 'Sao chép';
    const modal = new OperationModal(
      'Xác nhận Kéo thả',
      `<p>Bạn có chắc muốn ${actionText} ${srcs.length} mục vào <br><strong>${destPath}</strong>?</p>`
    );
    modal.open();
    
    modal.getElement().querySelector('.confirm')?.addEventListener('click', async () => {
      modal.close();
      try {
        const destFiles = destPane === 'left' ? appState.explorer?.leftFiles || [] : appState.explorer?.rightFiles || [];
        const destNames = destFiles.map(f => f.name);
        
        let applyToAllRes: ConflictResult | null = null;
        let finalSrcsToProcess: { src: string, dest: string, action: 'replace' | 'skip' | 'keep_both' }[] = [];

        for (const src of srcs) {
          const name = baseName(src);
          const existing = destNames.includes(name);
          let action: 'replace' | 'skip' | 'keep_both' = 'replace';

          if (existing) {
            if (applyToAllRes) {
              action = applyToAllRes.resolution;
            } else {
              const conflictModal = new ConflictModal(name, srcs.length > 1);
              conflictModal.open();
              const res = await conflictModal.waitForResolution();
              action = res.resolution;
              if (res.applyToAll) applyToAllRes = res;
            }
          }
          
          if (action === 'skip') continue;

          let targetName = name;
          if (action === 'keep_both') {
             targetName = generateUniqueName(name, destNames);
             destNames.push(targetName);
          }

          finalSrcsToProcess.push({ src, dest: joinPath(destPath, targetName), action });
        }

        if (finalSrcsToProcess.length === 0) {
          logActivity('Bỏ qua', 'Tất cả các mục kéo thả đã bị bỏ qua');
          return;
        }

        if (destPane === 'left') {
          if (move) {
            for (const item of finalSrcsToProcess) {
              await fileOps.moveLocal(item.src, item.dest);
            }
          } else {
            for (const item of finalSrcsToProcess) {
               await fileOps.cpLocal(item.src, item.dest, true);
            }
          }
        } else {
          for (const item of finalSrcsToProcess) {
            await fileOps.upload(item.src, item.dest);
          }
        }
        
        logActivity(actionText, `${finalSrcsToProcess.length} mục tới ${destPath}`);
        
        await this.loadPane(destPane, destPath);
        // Move → pane nguồn cũng thay đổi (nếu khác pane đích).
        if (move && payload.pane !== destPane) {
          await this.loadPane(payload.pane, getPanePath(payload.pane));
        }
      } catch (e) {
        console.warn('drop fail:', e);
        logActivity(`Lỗi ${actionText}`, `Chi tiết: ${e}`);
      }
    });
  }

  private handleSelectRow(pane: Pane, f: FileItem) {
    const basePath = (pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath) ?? '/';
    const fullPath = basePath.endsWith('/') ? basePath + f.name : basePath + '/' + f.name;
    setPaneSelection(pane, [{
      pane,
      name: f.name,
      path: fullPath,
      is_dir: f.is_dir,
    }]);
  }

  /** Select All: chọn toàn bộ file trong pane active. */
  private selectAll(pane: Pane) {
    const files = pane === 'left' ? appState.explorer?.leftFiles : appState.explorer?.rightFiles;
    const basePath = getPanePath(pane);
    const sels = (files ?? []).map((f) => ({
      pane,
      name: f.name,
      path: basePath.endsWith('/') ? basePath + f.name : basePath + '/' + f.name,
      is_dir: f.is_dir,
    }));
    setPaneSelection(pane, sels);
    this.renderPane(pane);
  }

  private handleFolderContextMenu(pane: Pane, e: MouseEvent) {
    const basePath = (pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath) ?? '/';
    MenuEmpty(e, {
      path: basePath,
      pane,
      onRefresh: (p, pth) => this.loadPane(p, pth),
      onSelectAll: () => this.selectAll(pane),
    });
  }

  private handleContextMenu(pane: Pane, e: MouseEvent, f: FileItem) {
    const basePath = (pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath) ?? '/';
    this.handleSelectRow(pane, f);
    MenuFile(e, {
      file: f,
      path: basePath,
      pane,
      onRefresh: (p, pth) => this.loadPane(p, pth),
      onSelectAll: () => this.selectAll(pane),
      onOpenInNewTab: (path) => {
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        activeContainer.addTab(path);
      },
    });
  }

  private navigate(pane: Pane, path: string) {
    pushPaneHistory(pane, path);
    this.loadPane(pane, path);
  }

  private goBack(pane: Pane) {
    const p = popPaneBack(pane);
    if (p) this.loadPane(pane, p);
  }

  private goForward(pane: Pane) {
    const p = popPaneForward(pane);
    if (p) this.loadPane(pane, p);
  }

  private goUp(pane: Pane) {
    const current = getPanePath(pane);
    if (current === '/') return;
    const parent = current.substring(0, current.lastIndexOf('/')) || '/';
    this.navigate(pane, parent);
  }

  private goHome(pane: Pane) {
    this.navigate(pane, '/');
  }

  private refresh(pane: Pane) {
    this.loadPane(pane, getPanePath(pane));
  }

  getElement(): HTMLDivElement {
    return this.container;
  }
}