export class OperationModal {
  title: string;
  element: HTMLDivElement;
  constructor(title: string, bodyHTML: string) {
    this.title = title;
    this.element = document.createElement('div');
    this.element.className = 'modal-overlay';
    this.element.innerHTML = `
      <div class="operation-modal">
        <h2>${title}</h2>
        ${bodyHTML}
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
}
