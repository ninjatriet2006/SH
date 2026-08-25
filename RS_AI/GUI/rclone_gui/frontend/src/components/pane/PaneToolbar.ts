
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
}

/**
 * Toolbar của 1 pane: các nút ⬅ ➡ ⬆ 🏠 ⟳ ✏️ và chuyển chế độ xem.
 */
export class PaneToolbar {
  element: HTMLDivElement;
  private backBtn: HTMLButtonElement;
  private forwardBtn: HTMLButtonElement;
  private state: 'remote-active' | 'path-active' | 'filter-active' = 'remote-active';

  constructor(opts: PaneToolbarOptions) {
    this.element = document.createElement('div');
    this.element.className = 'pane-toolbar-nemo state-remote-active';

    const mkBtn = (label: string, onClick?: () => void, extraClass?: string): HTMLButtonElement => {
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
    
    // View modes
    const mode = opts.currentViewMode || 'list';
    
    const btnGrid = mkBtn('🔲', () => {
      opts.onChangeViewMode?.('grid');
      updateViewBtns('grid');
    }, mode === 'grid' ? 'active' : '');
    
    const btnList = mkBtn('📄', () => {
      opts.onChangeViewMode?.('list');
      updateViewBtns('list');
    }, mode === 'list' ? 'active' : '');
    
    const btnCompact = mkBtn('☰', () => {
      opts.onChangeViewMode?.('compact');
      updateViewBtns('compact');
    }, mode === 'compact' ? 'active' : '');

    const updateViewBtns = (m: string) => {
      btnGrid.classList.toggle('active', m === 'grid');
      btnList.classList.toggle('active', m === 'list');
      btnCompact.classList.toggle('active', m === 'compact');
    };

    viewGroup.appendChild(btnGrid);
    viewGroup.appendChild(btnList);
    viewGroup.appendChild(btnCompact);
    
    viewGroup.appendChild(separator());
    
    viewGroup.appendChild(mkBtn('⟳', opts.onRefresh));
    
    this.element.appendChild(viewGroup);

    // Logic chuyển state tự động
    opts.labelElement.addEventListener('mousedown', () => this.setState('remote-active'));
    opts.breadcrumbElement.addEventListener('mousedown', () => this.setState('path-active'));
    opts.filterElement.addEventListener('focus', () => this.setState('filter-active'));

    // Bấm ra ngoài (nếu mất focus) sẽ quay về remote-active
    // Tuy nhiên, để cho trải nghiệm tự nhiên, ta dùng document click outside
    document.addEventListener('mousedown', (e) => {
      if (!this.element.contains(e.target as Node)) {
        this.setState('remote-active');
      }
    });
  }

  private setState(newState: 'remote-active' | 'path-active' | 'filter-active') {
    if (this.state !== newState) {
      this.state = newState;
      this.element.className = `pane-toolbar-nemo state-${newState}`;
    }
  }

  updateHistoryState(canBack: boolean, canForward: boolean): void {
    this.backBtn.disabled = !canBack;
    this.forwardBtn.disabled = !canForward;
  }

  getElement(): HTMLDivElement {
    return this.element;
  }
}