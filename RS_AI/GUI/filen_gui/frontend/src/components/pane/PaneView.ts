import { appState, type FileItem } from '../../store';
import { FileTable } from './FileTable';
import { PaneToolbar } from './PaneToolbar';
import type { SortKey, SortDir } from '../../features/sort';
import type { PaneColKey, PaneColWidths } from '../../services/explorerStore';
import { parseDrag, type DragPayload } from '../../features/dragDrop';
import { SearchModal } from '../SearchModal';
import { PaneStatusBar } from './PaneStatusBar';
import { getFreeSpace } from '../../services/fileOps';

export interface PaneViewOptions {
  side: 'left' | 'right';
  sideLabel: string;
  /** Điều hướng tới path (breadcrumb click hoặc click thư mục). */
  onOpenDir: (path: string) => void;
  onOpenInNewTab?: (path: string) => void;
  /** Context menu trên 1 file. */
  onContextMenu: (e: MouseEvent, file: FileItem) => void;
  /** Context menu trên vùng trống của body (không phải hàng). */
  onContextMenuEmpty: (e: MouseEvent) => void;
  /** Gõ filter input. */
  onFilter: (query: string) => void;
  /** Tạo thư mục mới. */
  onMkdir: (name: string) => Promise<void>;
  /** Click tiêu đề cột → sort. */
  onSort: (key: SortKey) => void;
  /** Kéo resize handle → cập nhật width cột. */
  onColResize: (key: PaneColKey, width: number) => void;
  /** Click hàng → chọn file (phase 5). */
  onSelectRow: (file: FileItem) => void;
  /** Click bất kỳ đâu trong pane → pane active. */
  onActivate: () => void;
  /** Drop file vào vùng trống pane → copy/move vào thư mục hiện tại. */
  onDrop: (payload: DragPayload, destPath: string, move: boolean) => void;
  /** Lịch sử & Điều hướng */
  onBack?: () => void;
  onForward?: () => void;
  onUp?: () => void;
  onHome?: () => void;
  onRefresh?: () => void;
}

export interface PaneSort {
  key: SortKey;
  dir: SortDir;
}

/**
 * Đảm nhận 1 pane: render header (label + toolbar + filter) + breadcrumb + body table.
 * Không tự load dữ liệu — nhận files/path qua render() và báo sự kiện qua callbacks.
 */
export class PaneView {
  element: HTMLDivElement;
  toolbar: PaneToolbar;
  statusBar: PaneStatusBar;

  private opts: PaneViewOptions;
  private breadcrumbDiv: HTMLDivElement;
  private body: HTMLDivElement;
  private path = '/';
  public table: FileTable | null = null;
  private selectionAnchor: { index: number } = { index: -1 };
  private isEditingPath: boolean = false;
  public viewMode: 'list' | 'grid' | 'compact' = 'list';

  constructor(opts: PaneViewOptions) {
    this.opts = opts;
    this.element = document.createElement('div');
    this.element.className = `pane ${opts.side}`;

    window.addEventListener('filen-settings-changed', () => {
      if (this.path) {
        opts.onOpenDir(this.path);
      }
    });

    const label = document.createElement('span');
    label.className = 'pane-side-label';
    label.dataset.langId = opts.side === 'left' ? 'pane_mode_local' : 'pane_mode_cloud';
    label.textContent = opts.sideLabel;

    const filter = document.createElement('input');
    filter.className = 'pane-filter-nemo';
    filter.type = 'text';
    filter.placeholder = 'Lọc…';
    filter.dataset.langId = 'pane_filter_placeholder';
    filter.addEventListener('input', () => this.opts.onFilter(filter.value));

    this.breadcrumbDiv = document.createElement('div');
    this.breadcrumbDiv.className = 'pane-breadcrumb';

    this.toolbar = new PaneToolbar({
      labelElement: label,
      breadcrumbElement: this.breadcrumbDiv,
      filterElement: filter,
      onBack: opts.onBack,
      onForward: opts.onForward,
      onUp: opts.onUp,
      onHome: opts.onHome,
      onRefresh: opts.onRefresh,
      onEditPath: () => this.toggleEditPath(),
      onAdvancedSearch: opts.side === 'left' ? () => {
        new SearchModal(this.path, (p) => this.opts.onOpenDir(p)).open();
      } : undefined,
      currentViewMode: this.viewMode,
      onChangeViewMode: (mode) => {
        this.viewMode = mode;
        if (this.table) {
          this.table.element.dataset.viewMode = mode;
        }
      }
    });

    this.body = document.createElement('div');
    this.body.className = 'pane-body';

    this.statusBar = new PaneStatusBar();

    // Right-click trên vùng trống (không phải hàng) → menu thư mục.
    this.body.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      this.opts.onContextMenuEmpty(e);
    });
    // Click/kéo trên vùng trống (không phải hàng) → clear selection hoặc rubber-band.
    this.body.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      if ((e.target as HTMLElement).closest('.file-table')) return;
      this.table?.beginEmptyArea(e);
    });

    // Drop vào vùng trống pane → copy/move vào thư mục hiện tại.
    this.body.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer!.dropEffect = e.ctrlKey ? 'move' : 'copy';
      this.body.classList.add('drop-target');
    });
    this.body.addEventListener('dragleave', (e) => {
      if (!this.body.contains(e.relatedTarget as Node)) {
        this.body.classList.remove('drop-target');
      }
    });
    this.body.addEventListener('drop', (e) => {
      e.preventDefault();
      this.body.classList.remove('drop-target');
      const payload = parseDrag(e.dataTransfer!.getData('text/plain'));
      if (payload) this.opts.onDrop(payload, this.path, e.ctrlKey);
    });

    this.body.addEventListener('scroll', () => {
      if (this.table) {
        this.table.handleScroll(this.body.scrollTop, this.body.clientHeight);
      }
    });

    this.element.appendChild(this.toolbar.getElement());
    this.element.appendChild(this.body);
    this.element.appendChild(this.statusBar.element);

    // Click vào bất kỳ đâu trong pane (kể cả body/header) → pane active.
    this.element.addEventListener('click', () => opts.onActivate());
  }

  /** Render toàn bộ pane (breadcrumb + bảng file) cho path hiện tại. */
  render(
    files: FileItem[],
    path: string,
    sort: PaneSort,
    colWidths: PaneColWidths,
  ): void {
    if (path !== this.path) this.selectionAnchor.index = -1;
    this.path = path ?? '/';
    this.renderBreadcrumb();
    
    // Filter hidden files
    const showHidden = appState.settings?.showHiddenFiles ?? true;
    const visibleFiles = showHidden ? files : files.filter(f => !f.name.startsWith('.'));
    
    this.renderBody(visibleFiles, sort, colWidths);

    const totalSize = visibleFiles.reduce((acc, f) => acc + (f.size || 0), 0);
    this.statusBar.updateTotal(visibleFiles.length, totalSize);
    
    // Fetch free space
    getFreeSpace(this.path).then(free => {
      this.statusBar.updateFreeSpace(free);
    }).catch(err => {
      console.warn('Failed to get free space for', this.path, err);
      this.statusBar.updateFreeSpace(0);
    });
  }

  /** Hiện placeholder (lỗi / chưa đăng nhập) thay cho bảng file. */
  renderPlaceholder(html: string, path: string, action?: { label: string; onClick: () => void }): void {
    if (path !== this.path) {
      this.path = path;
      this.selectionAnchor.index = -1;
      this.renderBreadcrumb();
    }
    this.body.innerHTML = '';
    const div = document.createElement('div');
    div.className = 'pane-placeholder';
    div.innerHTML = html;
    if (action) {
      const btn = document.createElement('button');
      btn.className = 'btn';
      btn.style.marginTop = '15px';
      btn.textContent = action.label;
      btn.addEventListener('click', action.onClick);
      div.appendChild(btn);
    }
    this.body.appendChild(div);
  }

  private normalizePath(p: string): string {
    const parts = p.split(/[/\\]/); // Hỗ trợ cả / và \
    const stack: string[] = [];
    for (const part of parts) {
      if (part === '' || part === '.') continue;
      if (part === '..') {
        if (stack.length > 0) stack.pop();
      } else {
        stack.push(part);
      }
    }
    const isWindowsAbsolute = p.match(/^[a-zA-Z]:/);
    return isWindowsAbsolute ? stack.join('/') || '/' : '/' + stack.join('/');
  }

  private renderBreadcrumb(): void {
    this.breadcrumbDiv.innerHTML = '';

    if (this.isEditingPath) {
      const input = document.createElement('input');
      input.type = 'text';
      input.className = 'path-input';
      input.value = this.path;
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          const newPath = input.value.trim();
          if (newPath) this.opts.onOpenDir(this.normalizePath(newPath));
          this.isEditingPath = false;
          this.renderBreadcrumb();
        } else if (e.key === 'Escape') {
          this.isEditingPath = false;
          this.renderBreadcrumb();
        }
      });
      input.addEventListener('blur', () => {
        this.isEditingPath = false;
        this.renderBreadcrumb();
      });
      this.breadcrumbDiv.appendChild(input);
      input.focus();
      input.select();
    } else {
      const parts = this.path.split(/[/\\]/).filter((p) => p !== '' && p !== '/');
      let acc = '';
      
      const rootChip = document.createElement('span');
      rootChip.className = 'crumb';
      rootChip.textContent = '/';
      rootChip.addEventListener('click', () => this.opts.onOpenDir('/'));
      this.breadcrumbDiv.appendChild(rootChip);

      parts.forEach((part, index) => {
        acc += '/' + part;
        
        // Visual separator if not first element after root
        if (index >= 0) {
          const sep = document.createElement('span');
          sep.className = 'crumb-separator';
          sep.textContent = ' ⟩ ';
          this.breadcrumbDiv.appendChild(sep);
        }

        const chip = document.createElement('span');
        chip.className = 'crumb';
        chip.textContent = part;
        chip.addEventListener('click', () => this.opts.onOpenDir(acc));
        this.breadcrumbDiv.appendChild(chip);
      });
    }
  }

  private toggleEditPath(): void {
    this.isEditingPath = !this.isEditingPath;
    this.renderBreadcrumb();
  }

private renderBody(
    files: FileItem[],
    sort: PaneSort,
    colWidths: PaneColWidths,
  ): void {
    this.body.innerHTML = '';
    const table = new FileTable({
      files,
      sortKey: sort.key,
      sortDir: sort.dir,
      colWidths,
      pane: this.opts.side,
      basePath: this.path,
      anchor: this.selectionAnchor,
      onSort: (key) => this.opts.onSort(key),
      onColResize: (key, width) => this.opts.onColResize(key, width),
      onOpenDir: (dirName) => {
        const currentPath = this.path;
        let newPath = '';
        if (currentPath === '/') {
          newPath = `/${dirName}`;
        } else if (currentPath.endsWith('/')) {
          newPath = `${currentPath}${dirName}`;
        } else {
          newPath = `${currentPath}/${dirName}`;
        }
        this.opts.onOpenDir(newPath);
      },
      onOpenInNewTab: (dirName) => {
        if (!this.opts.onOpenInNewTab) return;
        const currentPath = this.path;
        let newPath = '';
        if (currentPath === '/') {
          newPath = `/${dirName}`;
        } else if (currentPath.endsWith('/')) {
          newPath = `${currentPath}${dirName}`;
        } else {
          newPath = `${currentPath}/${dirName}`;
        }
        this.opts.onOpenInNewTab(newPath);
      },
      onContextMenu: (e, file) => this.opts.onContextMenu(e, file),
      onDrop: (payload, destPath, move) => this.opts.onDrop(payload, destPath, move),
    });
    this.table = table;
    this.table.element.dataset.viewMode = this.viewMode;
    this.body.appendChild(table.getElement());
  }

  getElement(): HTMLDivElement {
    return this.element;
  }
}