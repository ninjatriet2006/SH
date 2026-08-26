export type FallbackAction = 'copy_delete' | 'local_transfer' | 'cancel';

export class FallbackModal {
  private element: HTMLDivElement;
  private resolvePromise!: (action: FallbackAction) => void;

  constructor(canCopyDelete: boolean, srcRemote: string, dstRemote: string) {
    this.element = document.createElement('div');
    this.element.className = 'modal-overlay';
    
    let copyDeleteBtnHtml = '';
    if (canCopyDelete) {
        copyDeleteBtnHtml = `<button id="fb-copy-delete" class="btn btn-primary" style="flex: 1; min-height: 40px; white-space: normal;">Sử dụng Sao chép & Xóa (Copy & Delete) trên máy chủ</button>`;
    } else {
        const reason = srcRemote !== dstRemote ? "Di chuyển chéo Cloud" : "Cloud không hỗ trợ";
        copyDeleteBtnHtml = `<button class="btn disabled" disabled style="flex: 1; min-height: 40px; opacity: 0.5; white-space: normal;">Sử dụng Sao chép & Xóa (Không khả dụng: ${reason})</button>`;
    }

    this.element.innerHTML = `
      <div class="operation-modal" style="max-width: 450px;">
        <h2>Di chuyển không được hỗ trợ</h2>
        <div style="margin-bottom: 20px; color: var(--text-color);">
            Cloud gốc không hỗ trợ tính năng Move (hoặc bạn đang kéo thả chéo Cloud). 
            Vui lòng chọn một phương pháp thay thế:
        </div>
        <div style="display: flex; flex-direction: column; gap: 10px;">
            ${copyDeleteBtnHtml}
            <button id="fb-local" class="btn" style="flex: 1; min-height: 40px; white-space: normal;">Tải về máy rồi Upload lên đích (Local Transfer - Chậm)</button>
        </div>
        <div style="display: flex; gap: 10px; margin-top: 20px; justify-content: flex-end;">
            <button id="fb-cancel" class="cancel btn">Huỷ (Cancel)</button>
        </div>
      </div>
    `;

    // Wire up buttons
    const btnCopyDelete = this.element.querySelector('#fb-copy-delete');
    if (btnCopyDelete) {
        btnCopyDelete.addEventListener('click', () => this.close('copy_delete'));
    }

    const btnLocal = this.element.querySelector('#fb-local');
    if (btnLocal) {
        btnLocal.addEventListener('click', () => this.close('local_transfer'));
    }

    const btnCancel = this.element.querySelector('#fb-cancel');
    if (btnCancel) {
        btnCancel.addEventListener('click', () => this.close('cancel'));
    }
  }

  public open(): Promise<FallbackAction> {
    document.body.appendChild(this.element);
    return new Promise((resolve) => {
        this.resolvePromise = resolve;
    });
  }

  private close(action: FallbackAction): void {
    this.element.remove();
    if (this.resolvePromise) {
        this.resolvePromise(action);
    }
  }
}
