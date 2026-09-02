import { appState, type FileItem } from '../../store';
import { FileTable } from './FileTable';
import { PaneToolbar } from './PaneToolbar';
import type { SortKey, SortDir } from '../../features/sort';
import { upgradeSelectToCustomDropdown } from '../../features/customDropdown';
import type { PaneColKey, PaneColWidths } from '../../services/explorerStore';
import { parseDrag, type DragPayload } from '../../features/dragDrop';
import { SearchModal } from '../SearchModal';
import { PaneStatusBar } from './PaneStatusBar';
import { getAboutSpace } from '../../services/fileOps';
import { escapeHtml } from '../../features/format';

export interface PaneViewOptions {
  side: 'left' | 'right';
  sideLabel: string;
  onOpenDir: (path: string) => void;
  onOpenInNewTab?: (path: string) => void;
  onContextMenu: (e: MouseEvent, file: FileItem) => void;
  onContextMenuEmpty: (e: MouseEvent) => void;
  onFilter: (query: string) => void;
  onMkdir: (name: string) => Promise<void>;
  onSort: (key: SortKey) => void;
  onColResize: (key: PaneColKey, width: number) => void;
  onSelectRow: (file: FileItem) => void;
  onActivate: () => void;
  onDrop: (payload: DragPayload, destPath: string, move: boolean) => void;
  onBack?: () => void;
  onForward?: () => void;
  onUp?: () => void;
  onHome?: () => void;
  onRefresh?: () => void;
  onRemoteChange?: (remote: string) => void;
  onBookmarkSelect?: (path: string) => void;
}

export interface PaneSort {
  key: SortKey;
  dir: SortDir;
}

export class PaneView {
  element: HTMLDivElement;
  toolbar: PaneToolbar;
  statusBar: PaneStatusBar;
  
  private remoteSelect: HTMLSelectElement;
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

    // Cài đặt đổi (vd: bật/tắt hiện file ẩn) → nạp lại danh sách hiện tại.
    // Dùng onRefresh chứ không phải onOpenDir để không đẩy thêm bản ghi vào
    // lịch sử điều hướng back/forward.
    window.addEventListener('rclonegui-settings-changed', this.onSettingsChanged);

    this.remoteSelect = document.createElement('select');
    this.remoteSelect.className = 'select-dropdown pane-side-label';
    this.remoteSelect.innerHTML = '<option value="">-- Đang tải --</option>';
    this.remoteSelect.addEventListener('change', () => {
      if (this.remoteSelect.value && opts.onRemoteChange) {
        opts.onRemoteChange(this.remoteSelect.value);
      }
    });

    const filter = document.createElement('input');
    filter.className = 'pane-filter-nemo';
    filter.type = 'text';
    filter.placeholder = 'Lọc…';
    filter.dataset.langId = 'pane_filter_placeholder';
    filter.addEventListener('input', () => this.opts.onFilter(filter.value));

    this.breadcrumbDiv = document.createElement('div');
    this.breadcrumbDiv.className = 'pane-breadcrumb';

    this.toolbar = new PaneToolbar({
      labelElement: this.remoteSelect,
      breadcrumbElement: this.breadcrumbDiv,
      filterElement: filter,
      onBack: opts.onBack,
      onForward: opts.onForward,
      onUp: opts.onUp,
      onHome: opts.onHome,
      onRefresh: opts.onRefresh,
      onEditPath: () => this.toggleEditPath(),
      onAdvancedSearch: () => {
        // Tìm kiếm đệ quy do backend `fs_search` đảm nhiệm; chọn kết quả sẽ
        // điều hướng pane này tới thư mục chứa file đó.
        new SearchModal(this.path, (dirPath) => this.opts.onOpenDir(dirPath)).open();
      },
      onBookmarkSelect: opts.onBookmarkSelect,
      currentViewMode: this.viewMode,
      onChangeViewMode: (mode) => {
        this.viewMode = mode;
        if (this.table) {
          this.table.element.dataset.viewMode = mode;
        }
      }
    });
    
    // Nâng cấp select SAU KHI nó đã được append vào PaneToolbar (PaneToolbar gọi appendChild ở trong constructor)
    upgradeSelectToCustomDropdown(this.remoteSelect, true);
    
    // Đảm bảo click vào custom dropdown input vẫn trigger remote-active
    const wrapper = this.remoteSelect.closest('.custom-dropdown-wrapper');
    if (wrapper) {
      wrapper.addEventListener('mousedown', () => {
         this.remoteSelect.dispatchEvent(new Event('mousedown'));
      });
    }

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
    
    // Fetch about space
    getAboutSpace(this.path).then(about => {
      this.statusBar.updateSpace(about);
    }).catch(err => {
      console.warn('Failed to get about space for', this.path, err);
      this.statusBar.updateSpace({});
    });
  }

  /** Hiện placeholder (lỗi / chưa đăng nhập) thay cho bảng file. */
  renderPlaceholder(html: string, path: string, action?: { label: string; onClick: () => void }): void {
    if (path !== this.path) {
      this.path = path;
      this.selectionAnchor.index = -1;
      this.renderBreadcrumb();
    }
    // Placeholder thay bảng → phải destroy table cũ
    if (this.table) {
      this.table.destroy();
      this.table = null;
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
      let remote = this.path.includes('::') ? this.path.split('::')[0] : 'Local';
      let realPath = this.path.includes('::') ? this.path.split('::').slice(1).join('::') : this.path;
      if (!this.path || this.path === '/') remote = '';
      if (this.remoteSelect) {
        this.remoteSelect.value = remote;
        // Đồng bộ lên UI của custom dropdown
        (this.remoteSelect as any)._syncCustomDropdown?.();
      }
      const parts = realPath.split(/[/\\]/).filter((p: string) => p !== '' && p !== '/');
      let acc = remote ? `${remote}::` : '';
      
      if (remote) {
        const rootChip = document.createElement('span');
        rootChip.className = 'crumb';
        rootChip.textContent = `${remote}:/`;
        rootChip.addEventListener('click', () => this.opts.onOpenDir(`${remote}::/`));
        this.breadcrumbDiv.appendChild(rootChip);
      }

      parts.forEach((part: string, index: number) => {
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
        const targetPath = acc;
        chip.addEventListener('click', () => this.opts.onOpenDir(targetPath));
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
    if (this.table && this.table['opts'] && this.table['opts'].basePath === this.path && this.table.element.parentElement === this.body) {
      this.table.updateData(files, sort.key, sort.dir, colWidths);
      return;
    }

    // Bảng cũ bị thay thế: phải destroy để nhả IntersectionObserver và các
    // window listener (pointermove/up, rclonegui-emblems-changed) mà nó đã đăng ký.
    if (this.table) {
      this.table.destroy();
      this.table = null;
    }
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
        // Tái sử dụng logic normalizePath
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

  private onSettingsChanged = () => {
    if (this.path) this.opts.onRefresh?.();
  };

  /** Giải phóng listener + bảng con. Gọi khi tab bị đóng. */
  public destroy(): void {
    window.removeEventListener('rclonegui-settings-changed', this.onSettingsChanged);
    this.toolbar.destroy();
    this.table?.destroy();
    this.table = null;
  }

  public setRemotes(remotes: any[]) {
    if (!this.remoteSelect) return;
    let optionsHtml = '<option value="">☁️ Chọn Remote...</option>';
    
    // Luôn luôn chèn Local vào đầu danh sách (nếu mảng remotes không có)
    const hasLocal = remotes.some(r => r.name === 'Local');
    if (!hasLocal) {
      optionsHtml += `<option value="Local">💻 Local (Máy tính)</option>`;
    }

    remotes.forEach(remote => {
      // Fix visual name for Local
      if (remote.name === 'Local') {
        optionsHtml += `<option value="Local">💻 Local (Máy tính)</option>`;
      } else {
        optionsHtml += `<option value="${escapeHtml(remote.name)}">☁️ ${escapeHtml(remote.name)} (${escapeHtml(remote.type)})</option>`;
      }
    });
    this.remoteSelect.innerHTML = optionsHtml;
    // Báo cho custom dropdown biết options đã đổi
    (this.remoteSelect as any)._updateCustomDropdown?.();
    
    // Sync the select value with the current path
    let remote = this.path.includes('::') ? this.path.split('::')[0] : 'Local';
    if (!this.path || this.path === '/') remote = '';
    if (remote) {
      this.remoteSelect.value = remote;
      (this.remoteSelect as any)._syncCustomDropdown?.();
    }
  }
}
