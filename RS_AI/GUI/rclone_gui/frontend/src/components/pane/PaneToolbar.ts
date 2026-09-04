import { appState, saveSettings } from '../../store';
import { showMenu } from '../../features/contextMenu';

export interface PaneToolbarOptions {
  labelElement: HTMLElement;
  breadcrumbElement: HTMLElement;
  filterElement: HTMLElement;
  onBack?: () => void;
  onForward?: () => void;
  onUp?: () => void;
  onHome?: () => void;
  onRefresh?: () => void;
  onEditPath?: () => void;
  onAdvancedSearch?: () => void;
  currentViewMode?: 'list' | 'grid' | 'compact';
  onChangeViewMode?: (mode: 'list' | 'grid' | 'compact') => void;
  /** Bấm dấu sao → ghim/bỏ ghim vị trí hiện tại. */
  onToggleBookmark?: (path: string) => void;
}

/**
 * Toolbar của 1 pane: các nút ⬅ ➡ ⬆ 🏠 ⟳ ✏️ và chuyển chế độ xem.
 */
export class PaneToolbar {
  element: HTMLDivElement;
  private backBtn: HTMLButtonElement;
  private forwardBtn: HTMLButtonElement;
  private state: 'remote-active' | 'path-active' | 'filter-active' = 'remote-active';
  private onSettingsChanged?: () => void;
  private bookmarkBtn!: HTMLButtonElement;
  private currentPath = '';

  private onDocMouseDown = (e: MouseEvent) => {
    if (!this.element.contains(e.target as Node)) {
      this.setState('remote-active');
    }
  };

  constructor(opts: PaneToolbarOptions) {
    this.element = document.createElement('div');
    this.element.className = 'pane-toolbar-nemo state-remote-active';

    const mkBtn = (label: string, onClick?: (e: MouseEvent) => void, extraClass?: string): HTMLButtonElement => {
      const b = document.createElement('button');
      b.className = `nemo-btn ${extraClass || ''}`;
      b.textContent = label;
      if (onClick) b.addEventListener('click', onClick);
      return b;
    };

    const separator = () => {
      const s = document.createElement('div');
      s.className = 'nemo-separator';
      return s;
    };

    // --- 1. Left Group (Navigation) ---
    const navGroup = document.createElement('div');
    navGroup.className = 'nemo-nav-group';
    this.backBtn = mkBtn('⬅', opts.onBack);
    this.forwardBtn = mkBtn('➡', opts.onForward);
    navGroup.appendChild(this.backBtn);
    navGroup.appendChild(this.forwardBtn);
    navGroup.appendChild(mkBtn('⬆', opts.onUp));
    navGroup.appendChild(mkBtn('🏠', opts.onHome));

    navGroup.appendChild(opts.labelElement);
    this.element.appendChild(navGroup);

    // --- 2. Center Group (Path) ---
    const pathGroup = document.createElement('div');
    pathGroup.className = 'nemo-path-group';
    
    // Nút Edit path đóng vai trò bung address bar
    const editBtn = mkBtn('✏️', () => {
      this.setState('path-active');
      opts.onEditPath?.();
    });
    pathGroup.appendChild(editBtn);
    pathGroup.appendChild(opts.breadcrumbElement);
    this.element.appendChild(pathGroup);

    // --- 3. Right Group (View & Search) ---
    const viewGroup = document.createElement('div');
    viewGroup.className = 'nemo-view-group';
    
    // Search toggle focuses filter input or opens advanced search
    const searchBtn = mkBtn('🔍', () => {
      this.setState('filter-active');
      if (opts.onAdvancedSearch) {
        opts.onAdvancedSearch();
      } else {
        opts.filterElement.focus();
      }
    });
    viewGroup.appendChild(searchBtn);
    viewGroup.appendChild(opts.filterElement);
    
    viewGroup.appendChild(separator());
    
    // View modes Dropdown
    let mode = opts.currentViewMode || 'list';
    
    const getModeIcon = (m: string) => {
      if (m === 'grid') return '🔲';
      if (m === 'compact') return '☰';
      return '📄';
    };

    const btnViewMode = mkBtn(getModeIcon(mode), (e: MouseEvent) => {
      const items = ['🔲 Lưới (Grid)', '📄 Chi tiết (List)', '☰ Thu gọn (Compact)'];
      showMenu(e, items, (label) => {
        let newMode: 'list' | 'grid' | 'compact' = 'list';
        if (label.includes('Lưới')) newMode = 'grid';
        else if (label.includes('Thu gọn')) newMode = 'compact';
        
        mode = newMode;
        btnViewMode.textContent = getModeIcon(newMode);
        if (opts.onChangeViewMode) opts.onChangeViewMode(newMode);
      });
    }, 'active');
    
    viewGroup.appendChild(btnViewMode);
    
    viewGroup.appendChild(separator());

    // Toggle hiện/ẩn file bắt đầu bằng dấu chấm (áp dụng cho cả 2 pane).
    const hiddenIcon = () => (appState.settings?.showHiddenFiles ? '👁️' : '🚫');
    const hiddenTitle = () =>
      appState.settings?.showHiddenFiles
        ? 'Đang hiện file ẩn — bấm để ẩn'
        : 'Đang ẩn file ẩn — bấm để hiện';
    const btnHidden = mkBtn(hiddenIcon(), () => {
      if (!appState.settings) return;
      appState.settings.showHiddenFiles = !appState.settings.showHiddenFiles;
      saveSettings(); // Tự phát sự kiện 'rclonegui-settings-changed'
    });
    btnHidden.title = hiddenTitle();
    // Đồng bộ nhãn khi chính pane này hoặc pane kia toggle.
    this.onSettingsChanged = () => {
      btnHidden.textContent = hiddenIcon();
      btnHidden.title = hiddenTitle();
    };
    window.addEventListener('rclonegui-settings-changed', this.onSettingsChanged);
    viewGroup.appendChild(btnHidden);

    viewGroup.appendChild(separator());

    // Ghim vị trí hiện tại — hành vi như dấu sao trên thanh địa chỉ trình duyệt.
    // ☆ chưa ghim, ★ đã ghim. Đặt giữa nút ẩn/hiện và reload.
    this.bookmarkBtn = mkBtn('☆', () => {
      const path = this.currentPath;
      if (!path) return;
      opts.onToggleBookmark?.(path);
    }, 'nemo-btn-bookmark');
    this.bookmarkBtn.title = 'Ghim vị trí này';
    viewGroup.appendChild(this.bookmarkBtn);

    viewGroup.appendChild(separator());

    viewGroup.appendChild(mkBtn('⟳', opts.onRefresh));
    
    this.element.appendChild(viewGroup);

    // Logic chuyển state tự động
    opts.labelElement.addEventListener('mousedown', () => this.setState('remote-active'));
    opts.breadcrumbElement.addEventListener('mousedown', () => this.setState('path-active'));
    opts.filterElement.addEventListener('focus', () => this.setState('filter-active'));

    // Bấm ra ngoài (nếu mất focus) sẽ quay về remote-active
    document.addEventListener('mousedown', this.onDocMouseDown);
  }

  /** Giải phóng các listener gắn ngoài phạm vi element này. */
  public destroy(): void {
    document.removeEventListener('mousedown', this.onDocMouseDown);
    if (this.onSettingsChanged) {
      window.removeEventListener('rclonegui-settings-changed', this.onSettingsChanged);
    }
  }

  private setState(newState: 'remote-active' | 'path-active' | 'filter-active') {
    if (this.state !== newState) {
      this.state = newState;
      this.element.className = `pane-toolbar-nemo state-${newState}`;
    }
  }

  /**
   * Cập nhật dấu sao theo path đang xem.
   * Gọi mỗi lần pane render để icon phản ánh đúng trạng thái ghim.
   */
  updateBookmarkState(path: string, bookmarked: boolean): void {
    this.currentPath = path;
    if (!this.bookmarkBtn) return;
    const enabled = Boolean(path);
    this.bookmarkBtn.disabled = !enabled;
    this.bookmarkBtn.textContent = bookmarked ? '★' : '☆';
    this.bookmarkBtn.classList.toggle('is-bookmarked', bookmarked);
    this.bookmarkBtn.title = !enabled
      ? 'Chưa chọn vị trí'
      : bookmarked
        ? 'Bỏ ghim vị trí này'
        : 'Ghim vị trí này';
  }

  updateHistoryState(canBack: boolean, canForward: boolean): void {
    this.backBtn.disabled = !canBack;
    this.forwardBtn.disabled = !canForward;
  }

  getElement(): HTMLDivElement {
    return this.element;
  }
}