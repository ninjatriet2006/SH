import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { listFiles } from '../../../bridge/explorer_api.ts';
import { logActivity } from '../store';
import type { FileItem } from '../store';
import { OperationModal } from './OperationModal';
import { PaneContainer } from './pane/PaneContainer';
import { MenuFile, MenuEmpty } from '../features/contextMenu';
import { sortFiles } from '../features/sort';
import { escapeHtml } from '../features/format';
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
  setPanePath,
  setPaneFiles,
  getPaneFiles,
  pushPaneHistory,
  popPaneBack,
  popPaneForward,
  canPaneGoBack,
  canPaneGoForward,
} from '../services/explorerStore';
import { setClipboard, hasClipboard, pasteTo } from '../features/clipboard';
import * as fileOps from '../services/fileOps';
import { isTrashPath, listTrash } from '../services/trashOps';
import { undoManager } from '../services/undoManager';
import { baseName, joinPath, generateUniqueName, type DragPayload } from '../features/dragDrop';
import { ConflictModal, type ConflictResult } from './ConflictModal';
import { transferManager } from '../features/transferManager';
import { listRemotes } from '../../../bridge/remote_api.ts';

// Cache ngắn hạn cho nội dung thư mục (TTL: 1 phút)
const dirCache = new Map<string, { files: FileItem[], timestamp: number }>();
const DIR_CACHE_TTL = 60 * 1000;

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
      onRefresh: () => this.refresh('left', true),
      onBookmarkSelect: (path) => this.navigate('left', path),
      onRemoteChange: (remote) => this.handleRemoteChange('left', remote)
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
      onRefresh: () => this.refresh('right', true),
      onBookmarkSelect: (path) => this.navigate('right', path),
      onRemoteChange: (remote) => this.handleRemoteChange('right', remote)
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
      this.container.style.setProperty('--pane-left-width', `${newLeftWidth}%`);
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
      const leftPath = getPanePath('left');
      if (leftPath.startsWith('Local::')) {
        // Reload without touching the selection
        this.loadPane('left', leftPath);
      }
      const rightPath = getPanePath('right');
      if (rightPath.startsWith('Local::')) {
        this.loadPane('right', rightPath);
      }
    });

    // Lắng nghe khi TransferManager xử lý xong một chuỗi tác vụ
    transferManager.addQueueEmptyListener(() => {
       const leftPath = getPanePath('left');
       if (leftPath) this.loadPane('left', leftPath, true, true);

       const rightPath = getPanePath('right');
       if (rightPath) this.loadPane('right', rightPath, true, true);
    });

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
        const files = getPaneFiles(pane);
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
    
    let leftPath = getPanePath('left');
    if (!leftPath || leftPath === '/') {
      leftPath = ''; // Không ép Local nữa, để trống cho người dùng tự chọn
    }

    let rightPath = getPanePath('right');
    if (!rightPath || rightPath === '/') {
      rightPath = '';
    }
    
    await Promise.all([
      this.loadPane('left', leftPath),
      this.loadPane('right', rightPath),
    ]);
  }

  async loadPane(pane: 'left' | 'right', path: string, silent = false, forceRefresh = false) {
    if (pane === 'left') {
        this.leftLoadRequestId++;
    } else {
        this.rightLoadRequestId++;
    }
    const currentReqId = pane === 'left' ? this.leftLoadRequestId : this.rightLoadRequestId;

    const currentPath = getPanePath(pane);
    
    // Cập nhật path ngay từ đầu để getPanePath luôn trả về đúng vị trí hiện tại
    // (kể cả path rỗng). Nếu chưa chọn remote thì xoá luôn danh sách file cũ.
    setPanePath(pane, path);
    if (!path || (!path.includes('::') && !isTrashPath(path))) {
      setPaneFiles(pane, []);
    }

    if (currentPath !== path && !silent) {
      clearPaneSelection(pane);
    }

    // ── Nhánh Thùng rác (đường dẫn ảo `trash://...`) ───────────────────────
    // Không đi qua rclone lsjson nên xử lý riêng, cũng không dùng dirCache vì
    // nội dung thùng rác đổi ngay sau mỗi lần khôi phục/xoá.
    if (isTrashPath(path)) {
      try {
        const trashFiles = await listTrash(path);
        if (pane === 'left' && this.leftLoadRequestId !== currentReqId) return;
        if (pane === 'right' && this.rightLoadRequestId !== currentReqId) return;

        setPaneFiles(pane, trashFiles);
        if (trashFiles.length === 0) {
          const view = pane === 'left' ? this.leftPane : this.rightPane;
          view.renderPlaceholder('🗑️ Thùng rác trống', path);
        } else {
          this.renderPane(pane);
        }
      } catch (e) {
        if (pane === 'left' && this.leftLoadRequestId !== currentReqId) return;
        if (pane === 'right' && this.rightLoadRequestId !== currentReqId) return;
        const view = pane === 'left' ? this.leftPane : this.rightPane;
        view.renderPlaceholder(`⚠️ Không đọc được thùng rác: ${String(e)}`, path);
      }
      return;
    }

    let remote = '';
    
    if (path.includes('::')) {
        const parts = path.split('::');
        remote = parts[0];
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

    // Check cache
    if (!forceRefresh) {
        const cached = dirCache.get(path);
        if (cached && Date.now() - cached.timestamp < DIR_CACHE_TTL) {
            files = [...cached.files];
        }
    }

    if (files.length === 0) {
      try {
        // Truyền `pane` để backend gắn inotify watcher đúng thư mục pane này xem.
        files = await listFiles(path, pane);
        dirCache.set(path, { files: [...files], timestamp: Date.now() });
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
    }
    
    if (pane === 'left' && this.leftLoadRequestId !== currentReqId) return;
    if (pane === 'right' && this.rightLoadRequestId !== currentReqId) return;

    files = sortFiles(files, getPaneSortKey(pane), true, getPaneSortDir(pane));
    
    setPaneFiles(pane, files);
    setPanePath(pane, path);
    this.renderPane(pane);
  }

  renderPane(pane: 'left' | 'right') {
    const view = pane === 'left' ? this.leftPane : this.rightPane;
    const files = getPaneFiles(pane);
    const path = getPanePath(pane);
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
    const path = getPanePath(pane) || '/';
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
      `<p>Bạn có chắc muốn ${actionText} ${srcs.length} mục vào <br><strong>${escapeHtml(destPath)}</strong>?</p>`
    );
    modal.open();
    
    modal.getElement().querySelector('.confirm')?.addEventListener('click', async () => {
      modal.close();
      try {
        const destFiles = getPaneFiles(destPane);
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
              await transferManager.enqueue('move', baseName(item.src), item.src, item.dest);
            }
          } else {
            for (const item of finalSrcsToProcess) {
               await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest);
            }
          }
        } else {
          if (move) {
            for (const item of finalSrcsToProcess) {
              await transferManager.enqueue('move', baseName(item.src), item.src, item.dest);
            }
          } else {
            for (const item of finalSrcsToProcess) {
              await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest);
            }
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
    const basePath = getPanePath(pane) || '/';
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
    const files = getPaneFiles(pane);
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
    const basePath = getPanePath(pane) || '/';
    MenuEmpty(e, {
      path: basePath,
      pane,
      onRefresh: (p, pth) => this.loadPane(p, pth),
      onSelectAll: () => this.selectAll(pane),
    });
  }

  private handleContextMenu(pane: Pane, e: MouseEvent, f: FileItem) {
    const basePath = getPanePath(pane) || '/';
    
    // Nếu file được click chưa có trong danh sách đang chọn, chọn duy nhất file này
    const currentSelection = getPaneSelection(pane);
    const isAlreadySelected = currentSelection.some(s => s.name === f.name);
    if (!isAlreadySelected) {
      this.handleSelectRow(pane, f);
    }
    
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

  /**
   * Xử lý khi người dùng đổi nguồn ở dropdown của pane.
   * Nhận cả remote thường, `Local`, và đường dẫn ảo `trash://...`.
   */
  private async handleRemoteChange(pane: Pane, remote: string): Promise<void> {
    if (!remote) {
      this.navigate(pane, '');
      return;
    }
    // Thùng rác: value chính là đường dẫn ảo, dùng nguyên.
    if (isTrashPath(remote)) {
      this.navigate(pane, remote);
      return;
    }
    if (remote === 'Local') {
      try {
        const homeDir = await invoke<string>('get_home_dir');
        this.navigate(pane, `Local::${homeDir}`);
      } catch {
        this.navigate(pane, 'Local::/');
      }
      return;
    }
    this.navigate(pane, `${remote}::/`);
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
    // Thùng rác là danh sách phẳng, không có cấp trên.
    if (isTrashPath(current)) return;
    const parent = current.substring(0, current.lastIndexOf('/')) || '/';
    this.navigate(pane, parent);
  }

  private goHome(pane: Pane) {
    this.navigate(pane, '/');
  }

  private refresh(pane: Pane, forceRefresh = false) {
    this.loadPane(pane, getPanePath(pane), false, forceRefresh);
  }

  getElement(): HTMLDivElement {
    return this.container;
  }

  /**
   * API hẹp dành cho các thành phần ngoài (MenuBar, TreeView) tác động lên
   * explorer, thay vì để chúng chạm trực tiếp vào nội bộ của DualPaneExplorer.
   */
  public get commands() {
    return {
      selectAllActive: () => this.selectAll(getActivePane()),
      goHomeActive: () => this.goHome(getActivePane()),
      refreshActive: () => this.refresh(getActivePane(), true),
      /** Điều hướng pane đang active tới `path` (có ghi lịch sử back/forward). */
      navigateActive: (path: string) => this.navigate(getActivePane(), path),
    };
  }
}