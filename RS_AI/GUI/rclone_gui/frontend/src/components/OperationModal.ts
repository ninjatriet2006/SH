/*
[INTEGRITY NOTES]
- Mục đích: Khung hộp thoại dùng chung (overlay + tiêu đề + thân + 2 nút Huỷ/Xác nhận).
- Trách nhiệm: Cung cấp một container `.modal-body` ổn định để các modal con
  (PropertiesModal, SearchModal, OpenWithModal...) thay nội dung sau khi tải xong dữ liệu.
- Tương tác: Được dùng bởi contextMenu, clipboard, DualPaneExplorer và các modal con.
*/
import { escapeHtml } from '../features/format';

export class OperationModal {
  title: string;
  element: HTMLDivElement;
  constructor(title: string, bodyHTML: string) {
    this.title = title;
    this.element = document.createElement('div');
    this.element.className = 'modal-overlay';
    this.element.innerHTML = `
      <div class="operation-modal">
        <h2>${escapeHtml(title)}</h2>
        <div class="modal-body">${bodyHTML}</div>
        <div style="display: flex; gap: 10px; margin-top: 15px; justify-content: flex-end;">
            <button class='cancel btn'>Huỷ (Cancel)</button>
            <button class='confirm btn btn-primary'>Xác nhận (Confirm)</button>
        </div>
      </div>
    `;
    
    // Wire up cancel button
    const cancelBtn = this.element.querySelector('.cancel');
    if (cancelBtn) {
        cancelBtn.addEventListener('click', () => this.close());
    }
  }
  open(): void {
    document.body.appendChild(this.element);
  }
  close(): void {
    this.element.remove();
  }
  getElement(): HTMLDivElement { return this.element; }

  /** Trả về container thân hộp thoại để thay nội dung sau khi tải dữ liệu xong. */
  getBody(): HTMLDivElement {
    return this.element.querySelector('.modal-body') as HTMLDivElement;
  }
}
