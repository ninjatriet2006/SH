import * as fileOps from '../services/fileOps';
import { upgradeSelectToCustomDropdown } from '../features/customDropdown';
import { logActivity } from '../store';
import type { Pane } from '../services/explorerStore';

export interface BatchRenameItem {
  pane: Pane;
  path: string;
  name: string;
}

export class BatchRenameModal {
  private items: BatchRenameItem[];
  private onRefresh: (pane: Pane, basePath: string) => Promise<void>;
  private basePath: string;
  private element: HTMLDivElement;

  constructor(items: BatchRenameItem[], basePath: string, onRefresh: (pane: Pane, basePath: string) => Promise<void>) {
    this.items = items;
    this.basePath = basePath;
    this.onRefresh = onRefresh;

    this.element = document.createElement('div');
    this.element.className = 'operation-modal';
    this.element.innerHTML = `
      <h2>Đổi tên hàng loạt</h2>
      <div style="display: flex; flex-direction: column; gap: 12px; min-width: 350px;">
        <p>Đổi tên <strong>${items.length}</strong> mục được chọn.</p>
        
        <div>
          <label style="display: block; margin-bottom: 4px;">Chế độ:</label>
          <select id="rename-mode" class="neon-input" style="width: 100%;">
            <option value="numbering">Đánh số thứ tự</option>
            <option value="replace">Tìm & Thay thế</option>
          </select>
        </div>

        <div id="numbering-options" style="display: flex; flex-direction: column; gap: 8px;">
          <div>
            <label style="display: block; margin-bottom: 4px;">Tên cơ sở (Base Name):</label>
            <input id="base-name" type="text" class="neon-input" style="width: 100%;" placeholder="Ví dụ: HinhAnh_" />
          </div>
          <div style="display: flex; gap: 8px;">
            <div style="flex: 1;">
              <label style="display: block; margin-bottom: 4px;">Bắt đầu từ:</label>
              <input id="start-num" type="number" class="neon-input" style="width: 100%;" value="1" min="0" />
            </div>
            <div style="flex: 1;">
              <label style="display: block; margin-bottom: 4px;">Số chữ số (Padding):</label>
              <input id="padding" type="number" class="neon-input" style="width: 100%;" value="3" min="1" max="10" />
            </div>
          </div>
        </div>

        <div id="replace-options" style="display: none; flex-direction: column; gap: 8px;">
          <div>
            <label style="display: block; margin-bottom: 4px;">Tìm chuỗi:</label>
            <input id="search-str" type="text" class="neon-input" style="width: 100%;" placeholder="Chuỗi cần tìm..." />
          </div>
          <div>
            <label style="display: block; margin-bottom: 4px;">Thay bằng:</label>
            <input id="replace-str" type="text" class="neon-input" style="width: 100%;" placeholder="Chuỗi thay thế..." />
          </div>
        </div>

        <div style="margin-top: 8px;">
          <label style="display: block; margin-bottom: 4px; color: var(--colors-text-muted);">Xem trước (File đầu tiên):</label>
          <div id="preview-text" style="font-family: monospace; background: var(--colors-bg-overlay); padding: 8px; border-radius: 4px;">...</div>
        </div>
        
        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px;">
          <button id="btn-cancel" class="neon-button secondary">Huỷ</button>
          <button id="btn-confirm" class="neon-button primary">Xác nhận</button>
        </div>
      </div>
    `;
    
    upgradeSelectToCustomDropdown(this.element.querySelector('#rename-mode') as HTMLSelectElement, false);
    this.bindEvents();
    this.updatePreview();
  }

  public open(): void {
    document.body.appendChild(this.element);
    const rect = this.element.getBoundingClientRect();
    this.element.style.position = 'fixed';
    this.element.style.top = `calc(50% - ${rect.height/2}px)`;
    this.element.style.left = `calc(50% - ${rect.width/2}px)`;
  }

  public close(): void {
    this.element.remove();
  }

  public getElement(): HTMLDivElement {
    return this.element;
  }


  private bindEvents() {
    const el = this.getElement();
    const modeSelect = el.querySelector('#rename-mode') as HTMLSelectElement;
    const numOpts = el.querySelector('#numbering-options') as HTMLElement;
    const repOpts = el.querySelector('#replace-options') as HTMLElement;
    
    modeSelect.addEventListener('change', () => {
      if (modeSelect.value === 'numbering') {
        numOpts.style.display = 'flex';
        repOpts.style.display = 'none';
      } else {
        numOpts.style.display = 'none';
        repOpts.style.display = 'flex';
      }
      this.updatePreview();
    });

    const inputs = el.querySelectorAll('input');
    inputs.forEach(input => input.addEventListener('input', () => this.updatePreview()));

    el.querySelector('#btn-cancel')?.addEventListener('click', () => this.close());
    el.querySelector('#btn-confirm')?.addEventListener('click', () => this.executeRename());
  }

  private calculateNewName(originalName: string, index: number): string {
    const el = this.getElement();
    const mode = (el.querySelector('#rename-mode') as HTMLSelectElement).value;
    
    // Tách phần tên và đuôi mở rộng (extension)
    const dotIndex = originalName.lastIndexOf('.');
    const hasExt = dotIndex > 0 && dotIndex < originalName.length - 1;
    const base = hasExt ? originalName.substring(0, dotIndex) : originalName;
    const ext = hasExt ? originalName.substring(dotIndex) : '';

    if (mode === 'numbering') {
      const baseName = (el.querySelector('#base-name') as HTMLInputElement).value;
      const startNum = parseInt((el.querySelector('#start-num') as HTMLInputElement).value) || 1;
      const padding = parseInt((el.querySelector('#padding') as HTMLInputElement).value) || 1;
      
      const numStr = (startNum + index).toString().padStart(padding, '0');
      return `${baseName}${numStr}${ext}`;
    } else {
      const searchStr = (el.querySelector('#search-str') as HTMLInputElement).value;
      const replaceStr = (el.querySelector('#replace-str') as HTMLInputElement).value;
      
      if (!searchStr) return originalName;
      // Replace all occurrences in the base name
      const newBase = base.split(searchStr).join(replaceStr);
      return `${newBase}${ext}`;
    }
  }

  private updatePreview() {
    if (this.items.length === 0) return;
    const firstItem = this.items[0];
    const newName = this.calculateNewName(firstItem.name, 0);
    const previewEl = this.getElement().querySelector('#preview-text');
    if (previewEl) {
      previewEl.textContent = `${firstItem.name} ➔ ${newName}`;
    }
  }

  private async executeRename() {
    this.close();
    let successCount = 0;
    const firstPane = this.items.length > 0 ? this.items[0].pane : 'left';

    try {
      for (let i = 0; i < this.items.length; i++) {
        const item = this.items[i];
        const newName = this.calculateNewName(item.name, i);
        if (newName !== item.name) {
          const oldPath = item.path;
          // fileOps.rename currently handles old path -> new name.
          // In fileOps.ts: rename(path, new_name)
          await fileOps.rename(oldPath, newName);
          successCount++;
        }
      }
      if (successCount > 0) {
        logActivity('Đổi tên hàng loạt', `Đã đổi tên ${successCount} mục.`);
        await this.onRefresh(firstPane, this.basePath);
      }
    } catch (e) {
      console.warn('Batch rename fail:', e);
      logActivity('Lỗi Đổi tên', `Chi tiết: ${e}`);
    }
  }
}
