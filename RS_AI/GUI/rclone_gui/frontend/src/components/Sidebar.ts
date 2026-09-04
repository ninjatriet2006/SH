/*
[INTEGRITY NOTES]
- Mục đích: Sidebar bên trái — gộp "Truy cập nhanh" (thư mục XDG) và "Đã ghim"
  (bookmark) vào một chỗ, phía dưới là cây thư mục Local.
- Trách nhiệm: Render 3 khối, phản ứng với `rclonegui-bookmarks-changed`, và hỗ
  trợ chế độ thu gọn (chỉ hiện icon, giống sidebar dọc của Firefox).
- Tương tác: Nhận callback `onSelect` từ main.ts để điều hướng pane đang active.
  Không truy cập biến toàn cục.

Vì sao gộp: trước đây bookmark nằm trong dropdown 🔖 trên toolbar, tách rời khỏi
sidebar vốn chỉ có cây thư mục tĩnh. Gộp lại cho giống mô hình trình duyệt: chỗ
ghim và chỗ truy cập nhanh là một.
*/
import { appState, isBookmarked, toggleBookmark } from '../store';
import { getUserPlaces, type UserPlace } from '../../../bridge/explorer_api';
import { TreeView } from './TreeView';
import { escapeHtml } from '../features/format';

export interface SidebarOptions {
  /** Điều hướng pane đang active tới `path` (dạng `Local::/...`). */
  onSelect: (path: string) => void;
}

export class Sidebar {
  private element: HTMLElement;
  private opts: SidebarOptions;
  private places: UserPlace[] = [];
  private tree: TreeView | null = null;
  private collapsed = false;

  private onBookmarksChanged = () => this.renderBookmarks();

  constructor(opts: SidebarOptions) {
    this.opts = opts;
    this.element = document.createElement('div');
    this.element.className = 'sidebar-inner';
    window.addEventListener('rclonegui-bookmarks-changed', this.onBookmarksChanged);
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  /** Nạp dữ liệu và dựng toàn bộ sidebar. */
  public async init(): Promise<void> {
    try {
      this.places = await getUserPlaces();
    } catch (e) {
      console.warn('Không lấy được danh sách thư mục người dùng:', e);
      this.places = [];
    }
    this.render();
  }

  /** Bật/tắt chế độ thu gọn (chỉ còn dải icon). */
  public setCollapsed(collapsed: boolean): void {
    this.collapsed = collapsed;
    this.element.classList.toggle('is-collapsed', collapsed);
  }

  public isCollapsed(): boolean {
    return this.collapsed;
  }

  public destroy(): void {
    window.removeEventListener('rclonegui-bookmarks-changed', this.onBookmarksChanged);
  }

  // ── Dựng UI ───────────────────────────────────────────────────────────────

  private render(): void {
    this.element.innerHTML = '';
    this.element.appendChild(this.buildPlacesSection());
    this.element.appendChild(this.buildBookmarksSection());
    this.element.appendChild(this.buildTreeSection());
    this.renderBookmarks();
  }

  /**
   * Một dòng trong sidebar. Ở chế độ thu gọn chỉ icon hiển thị (CSS ẩn nhãn),
   * nên `title` luôn được đặt để tooltip thay thế nhãn.
   */
  private makeRow(icon: string, label: string, onClick: () => void, extraClass = ''): HTMLButtonElement {
    const row = document.createElement('button');
    row.className = `side-row ${extraClass}`.trim();
    row.title = label;
    row.innerHTML = `<span class="side-row-icon">${escapeHtml(icon)}</span><span class="side-row-label">${escapeHtml(label)}</span>`;
    row.addEventListener('click', onClick);
    return row;
  }

  private makeSection(id: string, heading: string): HTMLElement {
    const section = document.createElement('div');
    section.className = 'side-section';
    section.dataset.section = id;

    const title = document.createElement('div');
    title.className = 'side-heading';
    title.textContent = heading;
    section.appendChild(title);

    const body = document.createElement('div');
    body.className = 'side-section-body';
    section.appendChild(body);

    return section;
  }

  private sectionBody(section: HTMLElement): HTMLElement {
    return section.querySelector('.side-section-body') as HTMLElement;
  }

  private buildPlacesSection(): HTMLElement {
    const section = this.makeSection('places', 'Truy cập nhanh');
    const body = this.sectionBody(section);

    if (this.places.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'side-empty';
      empty.textContent = '(Không đọc được thư mục hệ thống)';
      body.appendChild(empty);
      return section;
    }

    for (const place of this.places) {
      const path = `Local::${place.path}`;
      body.appendChild(this.makeRow(place.icon, place.name, () => this.opts.onSelect(path)));
    }
    return section;
  }

  private buildBookmarksSection(): HTMLElement {
    return this.makeSection('bookmarks', 'Đã ghim');
  }

  /** Vẽ lại danh sách ghim (gọi khi có sự kiện bookmark đổi). */
  private renderBookmarks(): void {
    const section = this.element.querySelector('[data-section="bookmarks"]') as HTMLElement | null;
    if (!section) return;
    const body = this.sectionBody(section);
    body.innerHTML = '';

    const bookmarks = appState.bookmarks || [];
    if (bookmarks.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'side-empty';
      empty.textContent = 'Chưa ghim vị trí nào — bấm ☆ trên thanh công cụ';
      body.appendChild(empty);
      return;
    }

    for (const bm of bookmarks) {
      const row = this.makeRow('★', bm.name, () => this.opts.onSelect(bm.path), 'side-row-bookmark');
      row.title = `${bm.name}\n${bm.path}`;

      // Nút bỏ ghim ngay trên dòng, không cần mở modal quản lý.
      const remove = document.createElement('span');
      remove.className = 'side-row-remove';
      remove.textContent = '✕';
      remove.title = 'Bỏ ghim';
      remove.addEventListener('click', (e) => {
        e.stopPropagation();
        toggleBookmark(bm.name, bm.path); // Đã ghim → gọi lần nữa là bỏ ghim
      });
      row.appendChild(remove);

      body.appendChild(row);
    }
  }

  private buildTreeSection(): HTMLElement {
    const section = this.makeSection('tree', 'Thư mục');
    const body = this.sectionBody(section);

    // Gốc cây là $HOME (mục đầu trong danh sách XDG).
    const home = this.places.find((p) => p.kind === 'HOME');
    if (!home) {
      const empty = document.createElement('div');
      empty.className = 'side-empty';
      empty.textContent = '(Không xác định được thư mục gốc)';
      body.appendChild(empty);
      return section;
    }

    this.tree = new TreeView(`Local::${home.path}`, this.opts.onSelect, home.name);
    body.appendChild(this.tree.getElement());
    return section;
  }
}

/** Cho phép kiểm tra nhanh trạng thái ghim của một path (dùng bởi toolbar). */
export function isPathBookmarked(path: string): boolean {
  return isBookmarked(path);
}
