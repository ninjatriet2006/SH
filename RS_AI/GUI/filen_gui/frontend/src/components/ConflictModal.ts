export type ConflictResolution = 'replace' | 'skip' | 'keep_both';

export interface ConflictResult {
  resolution: ConflictResolution;
  applyToAll: boolean;
}

export class ConflictModal {
  element: HTMLDivElement;
  private resolveFn: ((res: ConflictResult) => void) | null = null;

  constructor(filename: string, isMultiple: boolean) {
    this.element = document.createElement('div');
    this.element.className = 'operation-modal conflict-modal';
    
    let html = `<h2>⚠️ File Conflict</h2>
      <p style="margin-bottom: 15px;">File "<b>${filename}</b>" đã tồn tại tại thư mục đích.</p>
      <div class="conflict-actions" style="display: flex; gap: 10px; margin-bottom: 10px;">
        <button class="btn btn-replace" style="flex: 1; background: var(--accent); color: white;">Ghi đè</button>
        <button class="btn btn-skip" style="flex: 1;">Bỏ qua</button>
        <button class="btn btn-keep" style="flex: 1;">Giữ cả hai</button>
      </div>`;
      
    if (isMultiple) {
      html += `<div>
        <label style="cursor: pointer; display: flex; align-items: center; gap: 8px;">
          <input type="checkbox" id="apply-to-all"> 
          Áp dụng cho tất cả (Apply to all)
        </label>
      </div>`;
    }
    
    this.element.innerHTML = html;
    
    this.element.querySelector('.btn-replace')?.addEventListener('click', () => this.submit('replace'));
    this.element.querySelector('.btn-skip')?.addEventListener('click', () => this.submit('skip'));
    this.element.querySelector('.btn-keep')?.addEventListener('click', () => this.submit('keep_both'));
  }

  private submit(resolution: ConflictResolution) {
    const cb = this.element.querySelector('#apply-to-all') as HTMLInputElement | null;
    const applyToAll = cb ? cb.checked : false;
    
    if (this.resolveFn) {
      this.resolveFn({ resolution, applyToAll });
    }
    this.close();
  }

  async waitForResolution(): Promise<ConflictResult> {
    return new Promise((resolve) => {
      this.resolveFn = resolve;
    });
  }

  public open(): void {
    document.body.appendChild(this.element);
  }

  public close(): void {
    this.element.remove();
  }

  public getElement(): HTMLDivElement {
    return this.element;
  }
}
