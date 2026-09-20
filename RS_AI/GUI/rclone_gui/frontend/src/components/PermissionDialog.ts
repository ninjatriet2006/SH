/*
[INTEGRITY NOTES]
- Mục đích: Dialog consent leo thang quyền S2 (hỏi 1 lần / luôn / không).
- Trách nhiệm: Hiện modal thuần DOM khi backend trả `PERMISSION_CONSENT`, resolve lựa chọn.
- Tương tác: Được `features/transferManager.ts` gọi; không gọi IPC trực tiếp.
*/

export type PermissionChoice = 'once' | 'always' | 'never';

export class PermissionDialog {
  private element: HTMLDivElement;
  private resolvePromise!: (choice: PermissionChoice) => void;
  private settled = false;

  constructor(taskName: string, detail: string) {
    this.element = document.createElement('div');
    this.element.className = 'modal-overlay';
    // UNIVERSAL: escape HTML để tránh tiêm mã qua tên file/thông điệp lỗi.
    const safeName = taskName.replace(/[&<>"']/g, (c) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    } as Record<string, string>)[c]);
    const safeDetail = detail.slice(0, 300).replace(/[&<>"']/g, (c) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    } as Record<string, string>)[c]);
    this.element.innerHTML = `
      <div class="operation-modal" style="max-width: 450px;">
        <h2>Cần quyền hệ thống</h2>
        <div style="margin-bottom: 12px; color: var(--text-color);">
          Tác vụ <b>${safeName}</b> bị thiếu quyền.
          <div style="opacity: 0.75; font-size: 12px; margin-top: 6px;">${safeDetail}</div>
        </div>
        <div style="display: flex; flex-direction: column; gap: 10px;">
          <button id="pm-once" class="btn btn-primary" style="min-height: 40px;">Cho phép 1 lần</button>
          <button id="pm-always" class="btn" style="min-height: 40px;">Luôn cho phép (hệ thống)</button>
          <button id="pm-never" class="cancel btn" style="min-height: 40px;">Không cho phép</button>
        </div>
      </div>
    `;
    this.element.querySelector('#pm-once')?.addEventListener('click', () => this.close('once'));
    this.element.querySelector('#pm-always')?.addEventListener('click', () => this.close('always'));
    this.element.querySelector('#pm-never')?.addEventListener('click', () => this.close('never'));
  }

  public open(): Promise<PermissionChoice> {
    document.body.appendChild(this.element);
    return new Promise((resolve) => {
      this.resolvePromise = resolve;
    });
  }

  private close(choice: PermissionChoice): void {
    if (this.settled) return;
    this.settled = true;
    this.element.remove();
    this.resolvePromise(choice);
  }
}
