import { appState } from '../../store';
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
  onBookmarkSelect?: (path: string) => void;
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

    // Bookmark menu button
    const bmkBtn = mkBtn('🔖', undefined, 'nemo-btn-bookmark');
    bmkBtn.addEventListener('click', (e) => {
        const bookmarks = appState.bookmarks || [];
        if (bookmarks.length === 0) {
            showMenu(e, [{ label: '(Chưa có ghim nào)', disabled: true }], () => {});
            return;
        }
        const items = bookmarks.map(b => b.name);
        showMenu(e, items, (action) => {
            const b = bookmarks.find(x => x.name === action);
            if (b && opts.onBookmarkSelect) {
                opts.onBookmarkSelect(b.path);
            }
        });
    });
    navGroup.appendChild(bmkBtn);

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