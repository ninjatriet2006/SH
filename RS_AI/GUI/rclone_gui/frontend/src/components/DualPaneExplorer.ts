// Removed invoke import
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { listFiles } from '../../../bridge/explorer_api.ts';
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
import { transferManager } from '../features/transferManager';
import { listRemotes } from '../../../bridge/remote_api.ts';

/**
 * Orchestrator: tạo 2 PaneView + divider, load dữ liệu, xử lý command.
 * Mọi render UI (header/breadcrumb/table/toolbar) nằm trong pane/*.
 */
export class DualPaneExplorer {
  container: HTMLDivElement;
  private leftPane: PaneContainer;
  private rightPane: PaneContainer;
  private leftLoadRequestId = 0;
  private rightLoadRequestId = 0;

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
      onBookmarkSelect: (path) => this.navigate('left', path),
      onRemoteChange: async (remote) => {
        if (remote === 'Local') {
          try {
            const homeDir = await invoke<string>('get_home_dir');
            this.navigate('left', `Local::${homeDir}`);
          } catch (e) {
            this.navigate('left', `Local::/`);
          }
        } else if (remote) {
          this.navigate('left', `${remote}::/`);
        } else {
          this.navigate('left', '');
        }
      }
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
      onBookmarkSelect: (path) => this.navigate('right', path),
      onRemoteChange: async (remote) => {
        if (remote === 'Local') {
          try {
            const homeDir = await invoke<string>('get_home_dir');
            this.navigate('right', `Local::${homeDir}`);
          } catch (e) {
            this.navigate('right', `Local::/`);
          }
        } else if (remote) {
          this.navigate('right', `${remote}::/`);
        } else {
          this.navigate('right', '');
        }
      }
    });
    this.rightPane.onTabSwitch = (path) => this.loadPane('right', path);

    const divider = document.createElement('div');
    divider.className = 'pane-divider';

    // Drag to resize logic
    let isResizing = false;
    divider.addEventListener('mousedown', () => {
      isResizing = true;
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    });
    window.addEventListener('mousemove', (e) => {
      if (!isResizing) return;
      const containerRect = this.container.getBoundingClientRect();
      let newLeftWidth = ((e.clientX - containerRect.left) / containerRect.width) * 100;
      newLeftWidth = Math.max(10, Math.min(newLeftWidth, 90));
      this.container.style.gridTemplateColumns = `${newLeftWidth}% 4px 1fr`;
    });
    window.addEventListener('mouseup', () => {
      if (isResizing) {
        isResizing = false;
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }
    });

    this.container.appendChild(this.leftPane.getElement());
    this.container.appendChild(divider);
    this.container.appendChild(this.rightPane.getElement());

    this.setupListeners();
    this.initRemotes();
  }

  private async initRemotes() {
    try {
        const remotes = await listRemotes();
        this.leftPane.setRemotes(remotes);
        this.rightPane.setRemotes(remotes);
    } catch (e) {
        console.error("Failed to load remotes", e);
    }
  }

  // Listen for local directory changes (inotify)
  private setupListeners() {
    listen('local-dir-changed', () => {
      const leftPath = appState.explorer?.leftPath;
      if (leftPath && leftPath.startsWith('Local::')) {
        // Reload without touching the selection
        this.loadPane('left', leftPath);
      }
      const rightPath = appState.explorer?.rightPath;
      if (rightPath && rightPath.startsWith('Local::')) {
        this.loadPane('right', rightPath);
      }
    });

    // Lắng nghe khi TransferManager xử lý xong một chuỗi tác vụ
    transferManager.addQueueEmptyListener(() => {
       const leftPath = appState.explorer?.leftPath;
       if (leftPath) this.loadPane('left', leftPath, true);
       
       const rightPath = appState.explorer?.rightPath;
       if (rightPath) this.loadPane('right', rightPath, true);
    });
    
    // Tự động làm mới ngầm (auto reload) mỗi 5 giây cho các Cloud Remote
    // Riêng Local đã có inotify (local-dir-changed) ở trên nên không cần tốn tài nguyên quét
    setInterval(() => {
       const leftPath = appState.explorer?.leftPath;
       if (leftPath && !leftPath.startsWith('Local::')) {
           this.loadPane('left', leftPath, true);
       }
       
       const rightPath = appState.explorer?.rightPath;
       if (rightPath && !rightPath.startsWith('Local::')) {
           this.loadPane('right', rightPath, true);
       }
    }, 5000);

    this.init();
  }

  private handleKeyDown = (e: KeyboardEvent) => {
    const key = e.key;
    const pane = getActivePane();

    if (e.ctrlKey || e.metaKey) {
      const k = key.toLowerCase();
      if (k === 'c' || k === 'x') {
        const sels = getPaneSelection(pane);
        if (sels.length === 0) return;
        if (pane === 'right') {
          console.warn('Copy/Cut chưa hỗ trợ trên cloud pane');
          return;
        }
        e.preventDefault();
        setClipboard(k === 'c' ? 'copy' : 'cut', sels.map((s) => ({ pane, path: s.path })));
      } else if (k === 'v') {
        if (!hasClipboard()) return;
        e.preventDefault();
        const destPath = getPanePath(pane);
        pasteTo(pane, destPath, (p, pth) => this.loadPane(p, pth));
      } else if (k === 't') {
        e.preventDefault();
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        if (activeContainer) {
          activeContainer.addTab('');
        }
      } else if (k === 'w') {
        e.preventDefault();
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        if (activeContainer.activeTabId) {
          activeContainer.closeTab(activeContainer.activeTabId);
        }
      } else if (k === 'a') {
        e.preventDefault();
        this.selectAll(pane);
      } else if (k === 'z') {
        e.preventDefault();
        if (e.shiftKey) {
          undoManager.redo();
        } else {
          undoManager.undo();
        }
      } else if (k === 'y') {
        e.preventDefault();
        undoManager.redo();
      }
    } else {
      if (key === 'ArrowUp' || key === 'ArrowDown') {
        e.preventDefault();
        const files = pane === 'left' ? appState.explorer?.leftFiles : appState.explorer?.rightFiles;
        if (!files || files.length === 0) return;
        
        const selection = getPaneSelection(pane);
        let currentIndex = -1;
        if (selection.length > 0) {
          const lastSelected = selection[selection.length - 1];
          currentIndex = files.findIndex(f => f.name === lastSelected.name);
        }
        
        let nextIndex = key === 'ArrowDown' ? currentIndex + 1 : currentIndex - 1;
        if (nextIndex < 0) nextIndex = 0;
        if (nextIndex >= files.length) nextIndex = files.length - 1;
        
        const nextFile = files[nextIndex];
        const basePath = getPanePath(pane);
        const newPath = basePath.endsWith('/') ? basePath + nextFile.name : basePath + '/' + nextFile.name;
        
        // Single selection
        setPaneSelection(pane, [{
          pane: pane,
          name: nextFile.name,
          path: newPath,
          is_dir: nextFile.is_dir
        }]);
        
        // Cập nhật UI
        this.renderPane(pane);
        const activeContainer = pane === 'left' ? this.leftPane : this.rightPane;
        if (activeContainer && activeContainer.activeTabId) {
          const tab = activeContainer.tabs.find(t => t.id === activeContainer.activeTabId);
          if (tab && tab.view.table) {
            tab.view.table.scrollToRow(nextIndex);
          }
        }
      } else if (key === 'Enter') {
        e.preventDefault();
        const selection = getPaneSelection(pane);
        if (selection.length === 1) {
          const f = selection[0];
          if (f.is_dir) {
            this.navigate(pane, f.path);
          }
        }
      }
    }
  };

  async init() {
    window.addEventListener('keydown', this.handleKeyDown);
    
    let leftPath = appState.explorer?.leftPath;
    if (!leftPath || leftPath === '/') {
      leftPath = ''; // Không ép Local nữa, để trống cho người dùng tự chọn
    }
    
    let rightPath = appState.explorer?.rightPath;
    if (!rightPath || rightPath === '/') {
      rightPath = '';
    }
    
    await Promise.all([
      this.loadPane('left', leftPath),
      this.loadPane('right', rightPath),
    ]);
  }

  async loadPane(pane: 'left' | 'right', path: string, silent = false) {
    if (pane === 'left') {
        this.leftLoadRequestId++;
    } else {
        this.rightLoadRequestId++;
    }
    const currentReqId = pane === 'left' ? this.leftLoadRequestId : this.rightLoadRequestId;

    const currentPath = pane === 'left' ? appState.explorer?.leftPath : appState.explorer?.rightPath;
    
    // Cập nhật appState ngay từ đầu để getPanePath luôn trả về đúng path hiện tại (kể cả path rỗng)
    if (appState.explorer) {
      if (pane === 'left') {
        appState.explorer.leftPath = path;
        if (!path || !path.includes('::')) appState.explorer.leftFiles = [];
      } else {
        appState.explorer.rightPath = path;
        if (!path || !path.includes('::')) appState.explorer.rightFiles = [];
      }
    }

    if (currentPath !== path && !silent) {
      clearPaneSelection(pane);
    }

    let remote = '';
    let realPath = path;
    
    if (path.includes('::')) {
        const parts = path.split('::');
        remote = parts[0];
        realPath = parts.slice(1).join('::');
    }
    
    if (!remote) {
        if (pane === 'left') {
            this.leftPane.renderPlaceholder('', path);
        } else {
            this.rightPane.renderPlaceholder('', path);
        }
        return;
    }

    if (!silent) {
      if (pane === 'right') {
        this.rightPane.renderPlaceholder(`⏳ Đang tải dữ liệu từ ${remote}...`, path);
      } else {
        this.leftPane.renderPlaceholder(`⏳ Đang đọc thư mục ${remote}...`, path);
      }
    }

    let files: FileItem[] = [];
    try {
      files = await listFiles(remote, realPath);
    } catch (e) {
      console.warn(`fs_list ${pane} fail (${path}):`, e);
      if (pane === 'left' && this.leftLoadRequestId !== currentReqId) return;
      if (pane === 'right' && this.rightLoadRequestId !== currentReqId) return;
      
      if (pane === 'right') {
        this.rightPane.renderPlaceholder(`⚠️ Lỗi: ${String(e)}`, path);
        return;
      }
      this.leftPane.renderPlaceholder(`⚠️ Không đọc được thư mục: ${String(e)}`, path, {
        label: '⬅ Quay lại',
        onClick: () => this.goBack('left')
      });
      return;
    }
    
    if (pane === 'left' && this.leftLoadRequestId !== currentReqId) return;
    if (pane === 'right' && this.rightLoadRequestId !== currentReqId) return;

    files = sortFiles(files, getPaneSortKey(pane), true, getPaneSortDir(pane));
    
    if (pane === 'left') {
      if (!appState.explorer) appState.explorer = {} as any;
      appState.explorer!.leftPath = path;
      appState.explorer!.leftFiles = files;
    } else {
      if (!appState.explorer) appState.explorer = {} as any;
      appState.explorer!.rightPath = path;
      appState.explorer!.rightFiles = files;
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
    await fileOps.mkdir(path + '/' + name);
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
              await transferManager.enqueue('move', baseName(item.src), item.src, item.dest, payload.pane === 'left', destPane === 'left');
            }
          } else {
            for (const item of finalSrcsToProcess) {
               await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest, payload.pane === 'left', destPane === 'left');
            }
          }
        } else {
          for (const item of finalSrcsToProcess) {
            await transferManager.enqueue('upload', baseName(item.src), item.src, item.dest, payload.pane === 'left', false);
          }
        }
        
        logActivity(actionText, `${finalSrcsToProcess.length} mục tới ${destPath}`);
        
        // Không đợi TransferManager chạy xong, chỉ tải lại pane gốc (vì transfer chạy ngầm)
        // Việc refresh thư mục đích sẽ do user tự làm hoặc ta có callback.
        // Tạm thời refresh luôn.
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