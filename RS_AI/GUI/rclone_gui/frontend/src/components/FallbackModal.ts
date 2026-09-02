export type FallbackAction = 'fallback_server_side' | 'fallback_local' | 'cancel';

export class FallbackModal {
  private element: HTMLDivElement;
  private resolvePromise!: (res: {action: FallbackAction, applyToAll: boolean}) => void;

  constructor(canCopyDelete: boolean, srcRemote: string, dstRemote: string, isMove: boolean = true) {
    this.element = document.createElement('div');
    this.element.className = 'modal-overlay';
    
    let copyDeleteBtnHtml = '';
    const operationName = isMove ? "Di chuyển (Move)" : "Sao chép (Copy)";
    
    if (canCopyDelete && isMove) {
        copyDeleteBtnHtml = `<button id="fb-copy-delete" class="btn btn-primary" style="flex: 1; min-height: 40px; white-space: normal;">Sử dụng Sao chép & Xóa (Copy & Delete) trên máy chủ</button>`;
    } else if (isMove) {
        const reason = srcRemote !== dstRemote ? "Di chuyển chéo Cloud" : "Cloud không hỗ trợ";
        copyDeleteBtnHtml = `<button class="btn disabled" disabled style="flex: 1; min-height: 40px; opacity: 0.5; white-space: normal;">Sử dụng Sao chép & Xóa (Không khả dụng: ${reason})</button>`;
    }

    this.element.innerHTML = `
      <div class="operation-modal" style="max-width: 450px;">
        <h2>${operationName} không được hỗ trợ</h2>
        <div style="margin-bottom: 20px; color: var(--text-color);">
            Cloud gốc không hỗ trợ tính năng ${operationName} (hoặc bạn đang kéo thả chéo Cloud). 
            Vui lòng chọn một phương pháp thay thế:
        </div>
        <div style="display: flex; flex-direction: column; gap: 10px;">
            ${copyDeleteBtnHtml}
            <button id="fb-local" class="btn" style="flex: 1; min-height: 40px; white-space: normal;">Tải về máy rồi Upload lên đích (Local Transfer - Chậm)</button>
        </div>
        <div style="display: flex; gap: 10px; margin-top: 20px; justify-content: flex-end; align-items: center;">
            <label style="display: flex; align-items: center; gap: 8px; margin-right: auto; cursor: pointer; color: var(--text-color);">
                <input type="checkbox" id="fb-apply-all"> Áp dụng cho các file tiếp theo
            </label>
            <button id="fb-cancel" class="cancel btn">Huỷ (Cancel)</button>
        </div>
      </div>
    `;

    // Wire up buttons
    const btnCopyDelete = this.element.querySelector('#fb-copy-delete');
    if (btnCopyDelete) {
        btnCopyDelete.addEventListener('click', () => this.close('fallback_server_side'));
    }

    const btnLocal = this.element.querySelector('#fb-local');
    if (btnLocal) {
        btnLocal.addEventListener('click', () => this.close('fallback_local'));
    }

    const btnCancel = this.element.querySelector('#fb-cancel');
    if (btnCancel) {
        btnCancel.addEventListener('click', () => this.close('cancel'));
    }
  }

  public open(): Promise<{action: FallbackAction, applyToAll: boolean}> {
    document.body.appendChild(this.element);
    return new Promise((resolve) => {
        this.resolvePromise = resolve;
    });
  }

  private close(action: FallbackAction): void {
    const applyToAllCheckbox = this.element.querySelector('#fb-apply-all') as HTMLInputElement;
    const applyToAll = applyToAllCheckbox ? applyToAllCheckbox.checked : false;
    
    this.element.remove();
    if (this.resolvePromise) {
        this.resolvePromise({action, applyToAll});
    }
  }
}
