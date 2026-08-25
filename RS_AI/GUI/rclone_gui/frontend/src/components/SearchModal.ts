import { OperationModal } from './OperationModal';
import * as fileOps from '../services/fileOps';

export class SearchModal {
  private modal: OperationModal;
  private currentPath: string;
  private debounceTimer: number | null = null;
  private onSelect: (path: string) => void;

  constructor(currentPath: string, onSelect: (path: string) => void) {
    this.currentPath = currentPath;
    this.onSelect = onSelect;
    this.modal = new OperationModal('Advanced Search (Local)', this.getHtml());
  }

  private getHtml(): string {
    return `
      <div class="search-modal-container" style="display: flex; flex-direction: column; gap: 10px; height: 450px;">
        <input type="text" id="advanced-search-input" placeholder="Search files recursively..." style="padding: 10px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--surface-bg); color: var(--text-color); font-size: 14px;">
        
        <div id="advanced-search-toggle" style="cursor: pointer; font-size: 12px; color: var(--primary-color); display: flex; align-items: center; gap: 4px;">
          <span>▶</span> Hiển thị tuỳ chọn nâng cao
        </div>
        
        <div id="advanced-search-options" style="display: none; flex-direction: column; gap: 8px; padding: 10px; background: var(--hover-bg); border-radius: 4px; border: 1px solid var(--border-color);">
          <label style="display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-color);">
            <input type="checkbox" id="search-opt-fuzzy"> Sử dụng thuật toán Fuzzy Match
          </label>
          
          <div style="display: flex; flex-direction: column; gap: 4px;">
            <label style="font-size: 12px; color: var(--text-muted);">Nội dung file chứa (Text/Log... <10MB):</label>
            <input type="text" id="search-opt-content" placeholder="Ví dụ: main() { ..." style="padding: 6px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--bg-color); color: var(--text-color); font-size: 13px;">
          </div>
          
          <div style="display: flex; gap: 10px;">
            <div style="display: flex; flex-direction: column; gap: 4px; flex: 1;">
              <label style="font-size: 12px; color: var(--text-muted);">Dung lượng tối thiểu (MB):</label>
              <input type="number" id="search-opt-min-size" min="0" placeholder="0" style="padding: 6px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--bg-color); color: var(--text-color); font-size: 13px;">
            </div>
            <div style="display: flex; flex-direction: column; gap: 4px; flex: 1;">
              <label style="font-size: 12px; color: var(--text-muted);">Dung lượng tối đa (MB):</label>
              <input type="number" id="search-opt-max-size" min="0" placeholder="0" style="padding: 6px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--bg-color); color: var(--text-color); font-size: 13px;">
            </div>
          </div>
        </div>

        <div id="advanced-search-results" style="flex: 1; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px; background: var(--surface-bg);">
          <div style="padding: 10px; text-align: center; color: var(--text-muted);">Type to search...</div>
        </div>
      </div>
    `;
  }

  public open(): void {
    this.modal.open();
    
    const confirmBtn = this.modal.getElement().querySelector('.confirm') as HTMLButtonElement;
    if (confirmBtn) {
      confirmBtn.style.display = 'none'; // Only need Close button
    }
    
    const cancelBtn = this.modal.getElement().querySelector('.cancel') as HTMLButtonElement;
    if (cancelBtn) {
      cancelBtn.textContent = 'Close';
    }

    const input = this.modal.getElement().querySelector('#advanced-search-input') as HTMLInputElement;
    const toggleBtn = this.modal.getElement().querySelector('#advanced-search-toggle') as HTMLDivElement;
    const optionsDiv = this.modal.getElement().querySelector('#advanced-search-options') as HTMLDivElement;
    
    if (toggleBtn && optionsDiv) {
      toggleBtn.addEventListener('click', () => {
        const isHidden = optionsDiv.style.display === 'none';
        optionsDiv.style.display = isHidden ? 'flex' : 'none';
        toggleBtn.innerHTML = isHidden ? '<span>▼</span> Ẩn tuỳ chọn nâng cao' : '<span>▶</span> Hiển thị tuỳ chọn nâng cao';
      });
    }

    const triggerSearch = () => {
      const query = input?.value || '';
      if (this.debounceTimer) clearTimeout(this.debounceTimer);
      this.debounceTimer = window.setTimeout(() => this.performSearch(query), 400);
    };

    if (input) {
      input.focus();
      input.addEventListener('input', triggerSearch);
    }
    
    // Bind inputs to trigger search automatically
    ['#search-opt-fuzzy', '#search-opt-content', '#search-opt-min-size', '#search-opt-max-size'].forEach(selector => {
      const el = this.modal.getElement().querySelector(selector);
      if (el) {
        el.addEventListener('input', triggerSearch);
        el.addEventListener('change', triggerSearch);
      }
    });
  }

  private async performSearch(query: string) {
    const listEl = this.modal.getElement().querySelector('#advanced-search-results');
    if (!listEl) return;

    if (!query.trim()) {
      listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--text-muted);">Type to search...</div>`;
      return;
    }

    listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--text-muted);">Searching...</div>`;

    try {
      const fuzzyCb = this.modal.getElement().querySelector('#search-opt-fuzzy') as HTMLInputElement;
      const contentInp = this.modal.getElement().querySelector('#search-opt-content') as HTMLInputElement;
      const minSizeInp = this.modal.getElement().querySelector('#search-opt-min-size') as HTMLInputElement;
      const maxSizeInp = this.modal.getElement().querySelector('#search-opt-max-size') as HTMLInputElement;

      const minMB = parseFloat(minSizeInp?.value);
      const maxMB = parseFloat(maxSizeInp?.value);

      const options: fileOps.SearchOptions = {
        fuzzy: fuzzyCb?.checked || false,
        content_query: contentInp?.value?.trim() || null,
        min_size: !isNaN(minMB) ? minMB * 1024 * 1024 : null,
        max_size: !isNaN(maxMB) ? maxMB * 1024 * 1024 : null,
      };

      const results = await fileOps.searchLocal(this.currentPath, query, options);
      if (results.length === 0) {
        listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--text-muted);">No results found.</div>`;
        return;
      }

      listEl.innerHTML = '';
      results.forEach(res => {
        const item = document.createElement('div');
        item.style.padding = '8px 12px';
        item.style.cursor = 'pointer';
        item.style.borderBottom = '1px solid var(--border-color)';
        item.style.display = 'flex';
        item.style.flexDirection = 'column';
        item.style.gap = '2px';
        
        item.innerHTML = `
          <div style="font-size: 14px; color: var(--text-color); font-weight: 500; display: flex; justify-content: space-between;">
            <span>${res.item.is_dir ? '📁' : '📄'} ${res.item.name}</span>
            ${res.score !== undefined && res.score > 0 ? `<span style="font-size: 11px; color: var(--success-color);">Score: ${res.score}</span>` : ''}
          </div>
          <div style="font-size: 11px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; direction: rtl; text-align: left;">
            &lrm;${res.path}&lrm;
          </div>
        `;

        item.addEventListener('mouseover', () => item.style.background = 'var(--hover-bg)');
        item.addEventListener('mouseout', () => item.style.background = 'transparent');
        item.addEventListener('click', () => {
          this.modal.close();
          // Extract directory from path
          const lastSlash = res.path.lastIndexOf('/');
          const dirPath = lastSlash >= 0 ? res.path.substring(0, lastSlash) : res.path;
          this.onSelect(dirPath || '/');
        });

        listEl.appendChild(item);
      });
    } catch (e) {
      console.warn('Search failed:', e);
      listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: red;">Search failed.</div>`;
    }
  }
}
