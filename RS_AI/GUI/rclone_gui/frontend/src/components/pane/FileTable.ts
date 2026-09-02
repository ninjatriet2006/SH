import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../../store';
import { formatSize, formatDate, escapeHtml } from '../../features/format';
import type { SortKey, SortDir } from '../../features/sort';
import type {
  PaneColKey,
  PaneColWidths,
  Pane,
  ExplorerSelection,
} from '../../services/explorerStore';
import { getPaneSelection, setPaneSelection, getPaneVisibleCols, setPaneVisibleCols } from '../../services/explorerStore';
import { serializeDrag, parseDrag, type DragPayload } from '../../features/dragDrop';
import { ContextMenu } from '../ContextMenu';
import { floatingStatusBar } from '../FloatingStatusBar';
import { emblemStore } from '../../services/emblemStore';

/** Lấy loại hiển thị cho cột Type (dự phòng khi backend chưa gửi file_type). */
function typeOf(f: FileItem): string {
  if (f.file_type) return f.file_type;
  if (f.is_dir) return 'Folder';
  const idx = f.name.lastIndexOf('.');
  return idx > 0 && idx + 1 < f.name.length ? f.name.slice(idx + 1).toUpperCase() : '';
}

function getIconForFile(f: FileItem): string {
  if (f.is_dir) return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="var(--colors-blue)" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg folder"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>`;
  
  const ext = f.name.split('.').pop()?.toLowerCase();
  
  switch (ext) {
    case 'jpg': case 'jpeg': case 'png': case 'gif': case 'webp': case 'svg':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="var(--colors-green)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg image"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>`;
    case 'mp4': case 'mkv': case 'avi': case 'mov':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="var(--colors-purple)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg video"><path d="m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5"/><rect x="2" y="6" width="14" height="12" rx="2"/></svg>`;
    case 'mp3': case 'wav': case 'flac': case 'm4a':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="var(--colors-yellow)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg audio"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>`;
    case 'zip': case 'rar': case '7z': case 'tar': case 'gz':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="var(--colors-red)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg archive"><rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/></svg>`;
    case 'js': case 'ts': case 'html': case 'css': case 'json': case 'rs': case 'py': case 'c': case 'cpp': case 'java':
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="var(--colors-orange)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg code"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>`;
    default:
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="file-icon-svg file"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>`;
  }
}

function renderRowHTML(f: FileItem, activeCols: any[], basePath: string): string {
  const icon = getIconForFile(f);
  
  let fullPath = '';
  if (basePath.startsWith('trash://')) {
    fullPath = `${basePath}/${f.name}`;
  } else {
    fullPath = basePath === '/' ? `/${f.name}` : `${basePath}/${f.name}`;
  }
  
  const emblems = emblemStore.getEmblems(fullPath);
  let emblemHtml = '';
  if (emblems.length > 0) {
    emblemHtml = `<div class="emblem-overlay">${emblems.join('')}</div>`;
  }
  
  let html = '';
  activeCols.forEach(col => {
    switch(col.key) {
      case 'name':
        html += `<td class="name-col"><div class="icon-container"><span class="file-icon">${icon}</span>${emblemHtml}</div><span class="file-name">${escapeHtml(f.name)}</span></td>`;
        break;
      case 'type':
        html += `<td>${escapeHtml(typeOf(f))}</td>`;
        break;
      case 'size':
        html += `<td>${f.is_dir ? '-' : formatSize(f.size)}</td>`;
        break;
      case 'date':
        html += `<td>${escapeHtml(formatDate(f.mod_time))}</td>`;
        break;
      case 'permissions':
        html += `<td>${escapeHtml(f.permissions)}</td>`;
        break;
      case 'owner':
        html += `<td>${escapeHtml(f.owner)}</td>`;
        break;
      case 'group':
        html += `<td>${escapeHtml(f.group)}</td>`;
        break;
    }
  });
  return html;
}

const ALL_COLUMNS: { key: SortKey; label: string; resize?: PaneColKey }[] = [
  { key: 'name', label: 'Name', resize: 'name' },
  { key: 'type', label: 'Type', resize: 'type' },
  { key: 'size', label: 'Size', resize: 'size' },
  { key: 'date', label: 'Modified', resize: 'date' },
];

export interface FileTableOptions {
  files: FileItem[];
  sortKey: SortKey;
  sortDir: SortDir;
  colWidths: PaneColWidths;
  /** Pane đang render — dùng để đọc/ghi selection trong explorerStore. */
  pane: Pane;
  /** Path đầy đủ của thư mục đang xem — dùng để dựng selection path. */
  basePath: string;
  /** Tham chiếu anchor (giữ qua các lần render bảng) — do PaneView sở hữu. */
  anchor: { index: number };
  /** Click vào tiêu đề cột → sort theo cột đó (toggle hướng do orchestrator xử lý). */
  onSort: (key: SortKey) => void;
  /** Kéo resize handle → cập nhật width cột (lưu vào store). */
  onColResize: (key: PaneColKey, width: number) => void;
  /** Click vào hàng là thư mục → trả tên thư mục để PaneView tính path đầy đủ. */
  onOpenDir: (dirName: string) => void;
  /** Middle-click vào hàng là thư mục → mở thư mục đó trong tab mới. */
  onOpenInNewTab?: (dirName: string) => void;
  /** Context menu trên 1 hàng → trả event + file để orchestrator xử lý. */
  onContextMenu: (e: MouseEvent, file: FileItem) => void;
  /** Drop file vào thư mục này (hoặc vào thư mục hiện tại qua PaneView). */
  onDrop: (payload: DragPayload, destPath: string, move: boolean) => void;
  /** Callback khi selection thay đổi. */
  onSelectionChange?: (selectedFiles: FileItem[]) => void;
}

interface RowRec {
  row: HTMLTableRowElement;
  file: FileItem;
  path: string;
  thumbnailLoaded?: boolean;
}

interface RubberDrag {
  active: boolean;
  moved: boolean;
  startX: number;
  startY: number;
  base: ExplorerSelection[];
  ctrl: boolean;
  /** Index hàng bắt đầu kéo; -1 = bắt đầu trên vùng trống. */
  rowIndex: number;
}

/**
 * Render bảng file 4 cột Name | Type | Size | Modified.
 * - Click tiêu đề → sort (▲▼ hiển thị trên cột đang sort).
 * - Kéo handle bên phải th → đổi width cột; double-click handle → auto-fit.
 * - Click hàng → chọn 1 (Ctrl → toggle, Shift → chọn khoảng từ anchor).
 * - Kéo chuột (trên hàng hoặc vùng trống) → rubber-band chọn nhiều.
 * - Click vùng trống → clear selection.
 * Selection lưu qua explorerStore (setPaneSelection với danh sách path).
 */
export class FileTable {
  element: HTMLTableElement;

  private opts: FileTableOptions;
  private rows: RowRec[] = [];
  private selected: ExplorerSelection[] = [];
  private drag: RubberDrag | null = null;
  private rubberBandEl: HTMLDivElement | null = null;
  private rubberContainer: HTMLElement | null = null;
  
  private tbody: HTMLTableSectionElement;
  private spacerTop: HTMLTableRowElement;
  private spacerBottom: HTMLTableRowElement;
  private rowHeight = 33;
  private currentStartIndex = -1;
  private currentEndIndex = -1;
  private observer: IntersectionObserver;

  constructor(opts: FileTableOptions) {
    this.opts = opts;
    this.element = document.createElement('table');
    this.element.className = 'file-table';

    this.observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          const row = entry.target as HTMLTableRowElement;
          const index = parseInt(row.dataset.index || '-1');
          if (index >= 0) {
            const rec = this.rows[index];
            if (rec.path.startsWith('Local::') && !rec.thumbnailLoaded) {
              this.loadThumbnail(rec, row);
            }
          }
        }
      });
    }, {
      rootMargin: '100px 0px', // Fetch slightly before they appear
    });

    const thead = this.element.createTHead();
    const header = thead.insertRow();
    
    // Header Context Menu cho Column Chooser
    thead.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      const visibleCols = getPaneVisibleCols(opts.pane);
      const items = ALL_COLUMNS.map(col => {
        const isVisible = visibleCols.includes(col.key);
        return {
          label: `${isVisible ? '✓' : ' '} ${col.label}`,
          key: col.key,
        };
      });

      document.querySelectorAll('.context-menu').forEach((el) => el.remove());
      const menu = new ContextMenu(items.map(i => i.label));
      document.body.appendChild(menu.getElement());
      menu.getElement().style.top = `${e.clientY}px`;
      menu.getElement().style.left = `${e.clientX}px`;

      menu.getElement().addEventListener('click', (ev) => {
        const target = ev.target as HTMLElement;
        if (!target.classList.contains('item') || target.classList.contains('disabled')) return;
        const text = target.textContent || '';
        const clickedLabel = text.substring(2).trim();
        const clickedCol = ALL_COLUMNS.find(c => c.label === clickedLabel);
        if (clickedCol) {
          let newVisible = [...visibleCols];
          if (visibleCols.includes(clickedCol.key)) {
            newVisible = newVisible.filter(k => k !== clickedCol.key);
          } else {
            newVisible.push(clickedCol.key);
          }
          setPaneVisibleCols(opts.pane, newVisible);
        }
        menu.getElement().remove();
      });

      const closeMenu = (ce: MouseEvent) => {
        if (!menu.getElement().contains(ce.target as Node)) {
          menu.getElement().remove();
          document.removeEventListener('mousedown', closeMenu);
        }
      };
      document.addEventListener('mousedown', closeMenu);
    });

    const activeCols = ALL_COLUMNS.filter(c => getPaneVisibleCols(opts.pane).includes(c.key));

    activeCols.forEach((col) => {
      const th = document.createElement('th');
      th.dataset.col = col.key;

      // Sort UI: label + mũi tên ▲▼ trên cột đang sort.
      const label = document.createElement('span');
      label.className = 'sort-label';
      label.textContent = col.label;
      th.appendChild(label);
      const arrow = document.createElement('span');
      arrow.className = 'sort-arrow';
      if (col.key === opts.sortKey) {
        th.classList.add(opts.sortDir === 'asc' ? 'sorted-asc' : 'sorted-desc');
        arrow.textContent = opts.sortDir === 'asc' ? '▲' : '▼';
      }
      th.appendChild(arrow);
      th.classList.add('sortable');
      th.addEventListener('click', () => opts.onSort(col.key));

      // Resize handle (trừ cột Modified cuối).
      if (col.resize) {
        th.style.width = `${opts.colWidths[col.resize]}px`;
        this.bindResize(th, col.resize, opts);
      }
      header.appendChild(th);
    });

    window.addEventListener('filen-emblems-changed', this.onEmblemsChanged);

    this.tbody = this.element.createTBody();
    
    this.spacerTop = document.createElement('tr');
    this.spacerTop.className = 'virtual-spacer';
    this.spacerTop.style.border = 'none';
    const tdTop = document.createElement('td');
    tdTop.colSpan = activeCols.length;
    tdTop.style.padding = '0';
    tdTop.style.border = 'none';
    this.spacerTop.appendChild(tdTop);

    this.spacerBottom = document.createElement('tr');
    this.spacerBottom.className = 'virtual-spacer';
    this.spacerBottom.style.border = 'none';
    const tdBottom = document.createElement('td');
    tdBottom.colSpan = activeCols.length;
    tdBottom.style.padding = '0';
    tdBottom.style.border = 'none';
    this.spacerBottom.appendChild(tdBottom);

    opts.files.forEach((f, i) => {
      const row = document.createElement('tr');
      row.innerHTML = renderRowHTML(f, activeCols, this.opts.basePath);
      const path = opts.basePath.endsWith('/') ? opts.basePath + f.name : opts.basePath + '/' + f.name;
      const rec: RowRec = { row, file: f, path };
      this.rows.push(rec);
      row.addEventListener('pointerdown', (e) => {
        this.beginPointer(e, i);
        e.stopPropagation();
      });
      row.addEventListener('auxclick', (e) => {
        if (e.button === 1 && f.is_dir && this.opts.onOpenInNewTab) { // Middle click
          e.preventDefault();
          e.stopPropagation();
          this.opts.onOpenInNewTab(f.name);
        }
      });
      row.addEventListener('dblclick', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (f.is_dir) {
          opts.onOpenDir(f.name);
        } else {
          // Mở file (File Execution)
          if (rec.path.startsWith('Local::')) {
            try {
              const { open } = await import('@tauri-apps/plugin-shell');
              const localPath = rec.path.replace(/^Local::/, '');
              await open(localPath);
            } catch (err) {
              console.error("Lỗi khi mở file Local:", err);
            }
          } else {
            alert('File này đang ở trên Cloud. Bạn cần copy về Local để mở!');
          }
        }
      });
      row.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        opts.onContextMenu(e, f);
      });
      this.bindRowDrag(row, rec);
      this.bindHoverEvents(row, rec);

      row.dataset.index = i.toString();
      if (rec.path.startsWith('Local::') && this.isImageFile(f)) {
        this.observer.observe(row);
      }
    });

    // Initial render
    this.handleScroll(0, 800); // 800px is a reasonable initial height assumption

    // Khôi phục selection từ store sau mỗi lần render (sort/load).
    this.selected = getPaneSelection(opts.pane);
    this.applyRowClasses();
  }

  private onEmblemsChanged = () => {
    // Re-render rows to update emblems
    const activeCols = ALL_COLUMNS.filter(c => getPaneVisibleCols(this.opts.pane).includes(c.key));
    this.rows.forEach(r => {
      r.row.innerHTML = renderRowHTML(r.file, activeCols, this.opts.basePath);
    });
  };

  public destroy() {
    this.observer.disconnect();
    window.removeEventListener('pointermove', this.onMove);
    window.removeEventListener('pointerup', this.onUp);
    window.removeEventListener('pointercancel', this.onUp);
    window.removeEventListener('filen-emblems-changed', this.onEmblemsChanged);
  }

  public appendFiles(chunk: FileItem[]) {
    const startIndex = this.opts.files.length;
    
    chunk.forEach((f, i) => {
      const globalIndex = startIndex + i;
      this.opts.files.push(f);
      
      const activeCols = ALL_COLUMNS.filter(c => getPaneVisibleCols(this.opts.pane).includes(c.key));
      const row = document.createElement('tr');
      row.innerHTML = renderRowHTML(f, activeCols, this.opts.basePath);
      const path = this.opts.basePath.endsWith('/') ? this.opts.basePath + f.name : this.opts.basePath + '/' + f.name;
      const rec: RowRec = { row, file: f, path };
      this.rows.push(rec);
      
      row.addEventListener('pointerdown', (e) => {
        this.beginPointer(e, globalIndex);
        e.stopPropagation();
      });
      row.addEventListener('auxclick', (e) => {
        if (e.button === 1 && f.is_dir && this.opts.onOpenInNewTab) { // Middle click
          e.preventDefault();
          e.stopPropagation();
          this.opts.onOpenInNewTab(f.name);
        }
      });
      row.addEventListener('dblclick', async (e) => {
        e.preventDefault();
        e.stopPropagation();
        if (f.is_dir) {
          this.opts.onOpenDir(f.name);
        } else {
          // Mở file
          if (rec.path.startsWith('Local::')) {
            try {
              const { open } = await import('@tauri-apps/plugin-shell');
              const localPath = rec.path.replace(/^Local::/, '');
              await open(localPath);
            } catch (err) {
              console.error("Lỗi khi mở file Local:", err);
            }
          } else {
            alert('File này đang ở trên Cloud. Bạn cần copy về Local để mở!');
          }
        }
      });
      row.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.opts.onContextMenu(e, f);
      });
      this.bindRowDrag(row, rec);
      this.bindHoverEvents(row, rec);
      
      row.dataset.index = globalIndex.toString();
      if (rec.path.startsWith('Local::') && this.isImageFile(f)) {
        this.observer.observe(row);
      }
    });

    // Force re-render to update spacers if we are at the bottom, or just let scroll handle it
    const paneBody = this.element.parentElement;
    if (paneBody) {
      this.handleScroll(paneBody.scrollTop, paneBody.clientHeight, true);
    }
  }

  /** Cập nhật dữ liệu vào bảng hiện tại để tránh nháy màn hình khi auto-reload */
  public updateData(files: FileItem[], sortKey: SortKey, sortDir: SortDir, colWidths: PaneColWidths) {
    const isSameFiles = this.opts.files.length === files.length && 
        this.opts.files.every((f, i) => f.name === files[i].name && f.size === files[i].size && f.mod_time === files[i].mod_time);

    this.opts.sortKey = sortKey;
    this.opts.sortDir = sortDir;
    this.opts.colWidths = colWidths;

    if (isSameFiles) {
        // Cập nhật header nếu sort đổi
        this.updateHeaderUI();
        this.selected = getPaneSelection(this.opts.pane);
        this.applyRowClasses();
        return;
    }

    this.opts.files = files;

    // Unobserve old rows
    this.rows.forEach(r => {
      if (r.path.startsWith('Local::') && r.thumbnailLoaded !== true) {
        this.observer.unobserve(r.row);
      }
    });
    this.rows = [];

    const activeCols = ALL_COLUMNS.filter(c => getPaneVisibleCols(this.opts.pane).includes(c.key));

    this.opts.files.forEach((f, i) => {
      const row = document.createElement('tr');
      row.innerHTML = renderRowHTML(f, activeCols, this.opts.basePath);
      const path = this.opts.basePath.endsWith('/') ? this.opts.basePath + f.name : this.opts.basePath + '/' + f.name;
      const rec: RowRec = { row, file: f, path };
      this.rows.push(rec);

      row.addEventListener('pointerdown', (e) => { this.beginPointer(e, i); e.stopPropagation(); });
      row.addEventListener('auxclick', (e) => {
        if (e.button === 1 && f.is_dir && this.opts.onOpenInNewTab) {
          e.preventDefault(); e.stopPropagation();
          this.opts.onOpenInNewTab(f.name);
        }
      });
      row.addEventListener('dblclick', async (e) => {
        e.preventDefault(); e.stopPropagation();
        if (f.is_dir) {
          this.opts.onOpenDir(f.name);
        } else {
          if (rec.path.startsWith('Local::')) {
            try {
              const { open } = await import('@tauri-apps/plugin-shell');
              const localPath = rec.path.replace(/^Local::/, '');
              await open(localPath);
            } catch (err) { console.error("Lỗi khi mở file Local:", err); }
          } else {
            alert('File này đang ở trên Cloud. Bạn cần copy về Local để mở!');
          }
        }
      });
      row.addEventListener('contextmenu', (e) => {
        e.preventDefault(); e.stopPropagation();
        this.opts.onContextMenu(e, f);
      });
      this.bindRowDrag(row, rec);
      this.bindHoverEvents(row, rec);

      row.dataset.index = i.toString();
      if (rec.path.startsWith('Local::') && this.isImageFile(f)) {
        this.observer.observe(row);
      }
    });

    this.updateHeaderUI();

    this.selected = getPaneSelection(this.opts.pane);
    this.applyRowClasses();

    const parent = this.element.parentElement;
    const scrollTop = parent ? parent.scrollTop : 0;
    const clientHeight = parent ? parent.clientHeight : 800;
    this.handleScroll(scrollTop, clientHeight, true);
  }

  private updateHeaderUI() {
    const headers = this.element.querySelectorAll('thead th');
    headers.forEach((th) => {
      const colKey = (th as HTMLElement).dataset.col as SortKey;
      if (!colKey) return;
      th.classList.remove('sorted-asc', 'sorted-desc');
      const arrow = th.querySelector('.sort-arrow');
      if (arrow) arrow.textContent = '';
      if (colKey === this.opts.sortKey) {
        th.classList.add(this.opts.sortDir === 'asc' ? 'sorted-asc' : 'sorted-desc');
        if (arrow) arrow.textContent = this.opts.sortDir === 'asc' ? '▲' : '▼';
      }
      if (colKey !== 'date' && this.opts.colWidths[colKey as PaneColKey]) {
        (th as HTMLElement).style.width = `${this.opts.colWidths[colKey as PaneColKey]}px`;
      }
    });
  }

  public handleScroll(scrollTop: number, clientHeight: number, forceRender = false) {
    const startIndex = Math.min(this.opts.files.length, Math.max(0, Math.floor(scrollTop / this.rowHeight) - 5));
    const endIndex = Math.min(this.opts.files.length, Math.ceil((scrollTop + clientHeight) / this.rowHeight) + 5);

    if (!forceRender && this.currentStartIndex === startIndex && this.currentEndIndex === endIndex) return;

    this.currentStartIndex = startIndex;
    this.currentEndIndex = endIndex;

    // Clear tbody efficiently
    this.tbody.innerHTML = '';

    // Calculate heights
    const topHeight = startIndex * this.rowHeight;
    const bottomHeight = Math.max(0, (this.opts.files.length - endIndex) * this.rowHeight);

    this.spacerTop.style.height = `${topHeight}px`;
    this.spacerBottom.style.height = `${bottomHeight}px`;

    if (topHeight > 0) this.tbody.appendChild(this.spacerTop);
    
    for (let i = startIndex; i < endIndex; i++) {
      this.tbody.appendChild(this.rows[i].row);
    }
    
    if (bottomHeight > 0) this.tbody.appendChild(this.spacerBottom);
  }

  public scrollToRow(index: number) {
    if (index < 0 || index >= this.opts.files.length) return;
    
    // Tìm pane-body parent để cuộn
    const paneBody = this.element.closest('.pane-body') as HTMLElement;
    if (!paneBody) return;
    
    const rowTop = index * this.rowHeight;
    const rowBottom = rowTop + this.rowHeight;
    const viewTop = paneBody.scrollTop;
    const viewBottom = viewTop + paneBody.clientHeight;
    
    if (rowTop < viewTop) {
      paneBody.scrollTop = rowTop;
    } else if (rowBottom > viewBottom) {
      paneBody.scrollTop = rowBottom - paneBody.clientHeight;
    }
  }

  private isImageFile(f: FileItem): boolean {
    if (f.is_dir) return false;
    const ext = f.name.split('.').pop()?.toLowerCase();
    return ['jpg', 'jpeg', 'png', 'webp', 'gif'].includes(ext || '');
  }

  private async loadThumbnail(rec: RowRec, row: HTMLTableRowElement) {
    rec.thumbnailLoaded = true; // prevent duplicate calls
    try {
      const base64 = await invoke('fs_get_thumbnail', { path: rec.path }) as string;
      const iconSpan = row.querySelector('.file-icon');
      if (iconSpan) {
        iconSpan.innerHTML = `<img src="${base64}" class="thumb-img" style="width: 24px; height: 24px; object-fit: cover; border-radius: 4px;" />`;
      }
    } catch (e) {
      console.warn('Failed to load thumbnail for', rec.path, e);
    }
  }

  /** Bắt đầu rubber-band / clear khi nhấn trên vùng trống của pane-body. */
  beginEmptyArea(e: PointerEvent): void {
    this.beginPointer(e, -1);
  }

  // ── Selection ────────────────────────────────────────────────────────────
  private toSel(r: RowRec): ExplorerSelection {
    return { pane: this.opts.pane, name: r.file.name, path: r.path, is_dir: r.file.is_dir };
  }

  private setSelection(newSels: ExplorerSelection[]): void {
    this.selected = newSels;
    setPaneSelection(this.opts.pane, newSels);
    this.applyRowClasses();
    
    if (this.opts.onSelectionChange) {
      const selectedFiles = this.opts.files.filter(f => newSels.some(s => s.name === f.name));
      this.opts.onSelectionChange(selectedFiles);
    }
  }

  private applyRowClasses(): void {
    const set = new Set(this.selected.map((s) => s.path));
    for (const r of this.rows) {
      r.row.classList.toggle('selected', set.has(r.path));
    }
  }

  private unionByPath(a: ExplorerSelection[], b: ExplorerSelection[]): ExplorerSelection[] {
    const seen = new Set<string>();
    const out: ExplorerSelection[] = [];
    for (const s of [...a, ...b]) {
      if (!seen.has(s.path)) {
        seen.add(s.path);
        out.push(s);
      }
    }
    return out;
  }

  private bindHoverEvents(row: HTMLTableRowElement, rec: RowRec) {
    let springTimeout: ReturnType<typeof setTimeout> | null = null;
    const f = rec.file;
    
    row.addEventListener('mouseenter', () => {
      let info = rec.path;
      if (!f.is_dir) {
        info += ` (${formatSize(f.size)})`;
      }
      floatingStatusBar.show(info);
    });
    
    row.addEventListener('mouseleave', () => {
      floatingStatusBar.hide();
    });
    
    row.addEventListener('dragenter', () => {
      if (f.is_dir) {
        springTimeout = setTimeout(() => {
          this.opts.onOpenDir(f.name);
        }, 1200);
      }
    });
    
    row.addEventListener('dragleave', () => {
      if (springTimeout) {
        clearTimeout(springTimeout);
        springTimeout = null;
      }
    });
    
    row.addEventListener('drop', () => {
      if (springTimeout) {
        clearTimeout(springTimeout);
        springTimeout = null;
      }
    });
  }

  // ── Drag & drop (kéo thả file giữa các pane) ────────────────────────────
  /** Gắn drag source (hàng kéo được) + drop target (chỉ hàng thư mục). */
  private bindRowDrag(row: HTMLTableRowElement, r: RowRec): void {
    const nameCol = row.querySelector('.name-col');
    if (nameCol) {
      // Chỉ cho phép kéo trực tiếp từ text hoặc icon để vùng trống dành cho rubber-band
      const icon = nameCol.querySelector('.file-icon') as HTMLElement;
      const name = nameCol.querySelector('.file-name') as HTMLElement;
      if (icon) icon.draggable = true;
      if (name) name.draggable = true;
    }

    // Nguồn kéo: nếu hàng chưa chọn → chọn riêng nó; đẩy toàn bộ selection vào payload.
    row.addEventListener('dragstart', (e) => {
      if (!this.selected.some((s) => s.path === r.path)) {
        this.setSelection([this.toSel(r)]);
      }
      const payload: DragPayload = {
        pane: this.opts.pane,
        paths: this.selected.map((s) => s.path),
      };
      e.dataTransfer!.setData('text/plain', serializeDrag(payload));
      e.dataTransfer!.effectAllowed = 'copyMove';
      e.dataTransfer!.setDragImage(row, 20, 20);
      row.classList.add('dragging');

      // Bắt đầu native drag-out đồng thời với HTML5 drag
      import('../../features/dragDrop').then(m => m.startOSDrag(payload.paths));
    });

    row.addEventListener('dragend', () => {
      row.classList.remove('dragging');
    });

    // Đích thả: chỉ thư mục nhận drop → copy/move vào thư mục đó.
    row.addEventListener('dragover', (e) => {
      if (!r.file.is_dir) return;
      e.preventDefault();
      e.stopPropagation();
      e.dataTransfer!.dropEffect = e.ctrlKey ? 'move' : 'copy';
      row.classList.add('drop-target');
    });

    row.addEventListener('dragleave', () => {
      row.classList.remove('drop-target');
    });

    row.addEventListener('drop', (e) => {
      if (!r.file.is_dir) return;
      e.preventDefault();
      e.stopPropagation();
      row.classList.remove('drop-target');
      const payload = parseDrag(e.dataTransfer!.getData('text/plain'));
      if (payload) this.opts.onDrop(payload, r.path, e.ctrlKey);
    });
  }

  // ── Pointer / rubber-band ───────────────────────────────────────────────
  private beginPointer(e: PointerEvent, rowIndex: number): void {
    if (e.button !== 0) return;
    this.drag = {
      active: true,
      moved: false,
      startX: e.clientX,
      startY: e.clientY,
      base: [...this.selected],
      ctrl: e.ctrlKey || e.metaKey,
      rowIndex,
    };
    window.addEventListener('pointermove', this.onMove);
    window.addEventListener('pointerup', this.onUp);
    window.addEventListener('pointercancel', this.onUp);
  }

  private onMove = (e: PointerEvent): void => {
    const d = this.drag;
    if (!d || !d.active) return;
    if (!d.moved) {
      if (Math.hypot(e.clientX - d.startX, e.clientY - d.startY) < 4) return;
      d.moved = true;
      this.showRubberBand();
    }
    this.updateRubberBand(e.clientX, e.clientY);
    this.applyRubberSelection();
  };

  private onUp = (e: PointerEvent): void => {
    const d = this.drag;
    if (!d) return;
    window.removeEventListener('pointermove', this.onMove);
    window.removeEventListener('pointerup', this.onUp);
    window.removeEventListener('pointercancel', this.onUp);
    this.drag = null;
    if (d.moved) {
      this.finishRubberBand();
    } else {
      this.handleClick(d.rowIndex, e);
    }
  };

  private showRubberBand(): void {
    const container = this.element.parentElement;
    if (!container) return;
    this.rubberContainer = container;
    const el = document.createElement('div');
    el.className = 'rubber-band';
    el.style.left = '0px';
    el.style.top = '0px';
    el.style.width = '0px';
    el.style.height = '0px';
    container.appendChild(el);
    this.rubberBandEl = el;
  }

  /** Gốc toạ độ nội dung của container (pane-body) trong viewport. */
  private containerOrigin(): { ox: number; oy: number; el: HTMLElement } {
    const el = this.rubberContainer ?? this.element.parentElement ?? this.element;
    const rect = el.getBoundingClientRect();
    return { ox: rect.left + el.clientLeft, oy: rect.top + el.clientTop, el };
  }

  private updateRubberBand(x: number, y: number): void {
    const el = this.rubberBandEl;
    const d = this.drag;
    if (!el || !d) return;
    const { ox, oy, el: container } = this.containerOrigin();
    const sx = d.startX - ox + container.scrollLeft;
    const sy = d.startY - oy + container.scrollTop;
    const cx = x - ox + container.scrollLeft;
    const cy = y - oy + container.scrollTop;
    const left = Math.min(sx, cx);
    const top = Math.min(sy, cy);
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
    el.style.width = `${Math.abs(cx - sx)}px`;
    el.style.height = `${Math.abs(cy - sy)}px`;
  }

  private rubberContentRect(): { left: number; top: number; right: number; bottom: number } {
    const el = this.rubberBandEl!;
    const left = parseFloat(el.style.left);
    const top = parseFloat(el.style.top);
    return {
      left,
      top,
      right: left + parseFloat(el.style.width),
      bottom: top + parseFloat(el.style.height),
    };
  }

  private applyRubberSelection(): void {
    const d = this.drag;
    if (!d || !this.rubberBandEl) return;
    const rb = this.rubberContentRect();
    const { ox, oy, el: container } = this.containerOrigin();
    const picked: ExplorerSelection[] = [];
    for (const r of this.rows) {
      const cell = r.row.cells[0] ?? r.row;
      const rr = cell.getBoundingClientRect();
      const left = rr.left - ox + container.scrollLeft;
      const top = rr.top - oy + container.scrollTop;
      const right = left + rr.width;
      const bottom = top + rr.height;
      if (left <= rb.right && right >= rb.left && top <= rb.bottom && bottom >= rb.top) {
        picked.push(this.toSel(r));
      }
    }
    this.selected = d.ctrl ? this.unionByPath(d.base, picked) : picked;
    this.applyRowClasses();
  }

  private finishRubberBand(): void {
    this.setSelection(this.selected);
    if (this.rubberBandEl) {
      this.rubberBandEl.remove();
      this.rubberBandEl = null;
    }
    this.rubberContainer = null;
    const first = this.rows.findIndex((r) => this.selected.some((s) => s.path === r.path));
    this.opts.anchor.index = first >= 0 ? first : this.opts.anchor.index;
  }

  private handleClick(rowIndex: number, e: PointerEvent): void {
    if (rowIndex < 0) {
      // Click vào vùng trống → clear selection.
      this.setSelection([]);
      return;
    }
    const rec = this.rows[rowIndex];
    if (e.shiftKey) {
      const anchor = this.opts.anchor.index >= 0 ? this.opts.anchor.index : rowIndex;
      const lo = Math.min(anchor, rowIndex);
      const hi = Math.max(anchor, rowIndex);
      this.setSelection(this.rows.slice(lo, hi + 1).map((r) => this.toSel(r)));
      // Anchor giữ nguyên để shift-click liên tiếp mở rộng khoảng.
    } else if (e.ctrlKey || e.metaKey) {
      const exists = this.selected.some((s) => s.path === rec.path);
      this.setSelection(
        exists ? this.selected.filter((s) => s.path !== rec.path) : [...this.selected, this.toSel(rec)],
      );
      this.opts.anchor.index = rowIndex;
    } else {
      this.setSelection([this.toSel(rec)]);
      this.opts.anchor.index = rowIndex;
    }
  }

  // ── Column resize ───────────────────────────────────────────────────────
  /** Gắn drag handle cho 1 cột; double-click → auto-fit theo nội dung. */
  private bindResize(
    th: HTMLTableCellElement,
    key: PaneColKey,
    opts: FileTableOptions,
  ): void {
    const handle = document.createElement('span');
    handle.className = 'resize-handle';
    th.appendChild(handle);
    handle.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
    });

    handle.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      handle.setPointerCapture(e.pointerId);
      const startX = e.clientX;
      const startW = th.getBoundingClientRect().width;
      const onMove = (ev: PointerEvent) => {
        this.setColWidth(th, key, startW + ev.clientX - startX, opts);
      };
      const onUp = () => {
        handle.removeEventListener('pointermove', onMove);
        handle.removeEventListener('pointerup', onUp);
      };
      handle.addEventListener('pointermove', onMove);
      handle.addEventListener('pointerup', onUp);
    });

    handle.addEventListener('dblclick', (e) => {
      e.preventDefault();
      e.stopPropagation();
      this.autoFit(th, key, opts);
    });
  }

  private setColWidth(
    th: HTMLTableCellElement,
    col: PaneColKey,
    width: number,
    opts: FileTableOptions,
  ): void {
    const w = Math.round(width);
    th.style.width = `${w}px`;
    opts.onColResize(col, w);
  }

  /** Auto-fit: width = max(content width của các ô trong cột + header). */
  private autoFit(
    th: HTMLTableCellElement,
    col: PaneColKey,
    opts: FileTableOptions,
  ): void {
    const activeCols = ALL_COLUMNS.filter(c => getPaneVisibleCols(opts.pane).includes(c.key));
    const idx = activeCols.findIndex((c) => c.key === col) + 1;
    const cells = Array.from(
      this.element.querySelectorAll<HTMLTableCellElement>(`tbody td:nth-child(${idx})`),
    );
    let max = th.scrollWidth;
    for (const cell of cells) {
      if (cell.scrollWidth > max) max = cell.scrollWidth;
    }
    this.setColWidth(th, col, max, opts);
  }

  getElement(): HTMLTableElement {
    return this.element;
  }
}