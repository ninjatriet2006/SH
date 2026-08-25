export class OperationModal {
  title: string;
  element: HTMLDivElement;
  constructor(title: string, bodyHTML: string) {
    this.title = title;
    this.element = document.createElement('div');
    this.element.className = 'operation-modal';
    this.element.innerHTML = `<h2>${title}</h2>${bodyHTML}<button class='confirm'>Confirm</button>`;
  }
  open(): void {
    document.body.appendChild(this.element);
  }
  close(): void {
    this.element.remove();
  }
  getElement(): HTMLDivElement { return this.element; }
}
