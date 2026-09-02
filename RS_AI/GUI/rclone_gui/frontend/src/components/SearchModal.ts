/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại tìm kiếm đệ quy trong thư mục đang xem (Local hoặc Remote).
- Trách nhiệm: Nhận từ khoá, gọi `fileOps.searchLocal` (backend `fs_search`), hiển thị
  kết quả; chọn một kết quả sẽ trả về thư mục chứa nó cho pane điều hướng tới.
- Tương tác: Mở từ nút 🔍 trên PaneToolbar thông qua PaneView.onAdvancedSearch.
*/
import { OperationModal } from './OperationModal';
import * as fileOps from '../services/fileOps';
import { escapeHtml } from '../features/format';

export class SearchModal {
  private modal: OperationModal;
  private currentPath: string;
  private debounceTimer: number | null = null;
  private onSelect: (path: string) => void;

  constructor(currentPath: string, onSelect: (path: string) => void) {
    this.currentPath = currentPath;
    this.onSelect = onSelect;
    this.modal = new OperationModal('Tìm kiếm đệ quy', this.getHtml());
  }

  private getHtml(): string {
    return `
      <div class="search-modal-container" style="display: flex; flex-direction: column; gap: 10px; height: 450px; width: 520px; max-width: 80vw;">
        <div style="font-size: 12px; color: var(--colors-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
          Trong: <strong>${escapeHtml(this.currentPath || '/')}</strong>
        </div>
        <input type="text" id="advanced-search-input" placeholder="Nhập tên tệp cần tìm…" style="padding: 10px; border-radius: 4px; border: 1px solid var(--colors-border-muted); background: var(--colors-surface-input); color: var(--colors-text-primary); font-size: 14px;">

        <div id="advanced-search-results" style="flex: 1; overflow-y: auto; border: 1px solid var(--colors-border-muted); border-radius: 4px; background: var(--colors-surface-input);">
          <div style="padding: 10px; text-align: center; color: var(--colors-text-muted);">Nhập từ khoá để tìm…</div>
        </div>
      </div>
    `;
  }

  public open(): void {
    this.modal.open();

    const confirmBtn = this.modal.getElement().querySelector('.confirm') as HTMLButtonElement;
    if (confirmBtn) {
      confirmBtn.style.display = 'none'; // Chỉ cần nút Đóng
    }

    const cancelBtn = this.modal.getElement().querySelector('.cancel') as HTMLButtonElement;
    if (cancelBtn) {
      cancelBtn.textContent = 'Đóng';
    }

    const input = this.modal.getElement().querySelector('#advanced-search-input') as HTMLInputElement;
    if (input) {
      input.focus();
      input.addEventListener('input', () => {
        const query = input.value;
        if (this.debounceTimer) clearTimeout(this.debounceTimer);
        this.debounceTimer = window.setTimeout(() => this.performSearch(query), 400);
      });
    }
  }

  private async performSearch(query: string) {
    const listEl = this.modal.getElement().querySelector('#advanced-search-results');
    if (!listEl) return;

    if (!query.trim()) {
      listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--colors-text-muted);">Nhập từ khoá để tìm…</div>`;
      return;
    }

    listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--colors-text-muted);">Đang tìm…</div>`;

    try {
      const results = await fileOps.searchLocal(this.currentPath, query);
      if (results.length === 0) {
        listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--colors-text-muted);">Không tìm thấy kết quả.</div>`;
        return;
      }

      listEl.innerHTML = '';
      results.forEach(res => {
        const item = document.createElement('div');
        item.style.padding = '8px 12px';
        item.style.cursor = 'pointer';
        item.style.borderBottom = '1px solid var(--colors-border-muted)';
        item.style.display = 'flex';
        item.style.flexDirection = 'column';
        item.style.gap = '2px';

        item.innerHTML = `
          <div style="font-size: 14px; color: var(--colors-text-primary); font-weight: 500;">
            ${res.item.is_dir ? '📁' : '📄'} ${escapeHtml(res.item.name)}
          </div>
          <div style="font-size: 11px; color: var(--colors-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
            ${escapeHtml(res.path)}
          </div>
        `;

        item.addEventListener('mouseover', () => item.style.background = 'var(--colors-surface-overlay)');
        item.addEventListener('mouseout', () => item.style.background = 'transparent');
        item.addEventListener('click', () => {
          this.modal.close();
          // Điều hướng tới thư mục chứa kết quả (bỏ phần tên file cuối).
          const lastSlash = res.path.lastIndexOf('/');
          const dirPath = lastSlash >= 0 ? res.path.substring(0, lastSlash) : res.path;
          this.onSelect(dirPath || '/');
        });

        listEl.appendChild(item);
      });
    } catch (e) {
      console.warn('Search failed:', e);
      listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--colors-neon-coral, red);">Tìm kiếm thất bại: ${escapeHtml(String(e))}</div>`;
    }
  }
}
