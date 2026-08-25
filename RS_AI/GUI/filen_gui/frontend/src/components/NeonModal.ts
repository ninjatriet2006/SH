export class NeonModal {
  element: HTMLDivElement;
  constructor(title: string = "") {
    this.element = document.createElement('div');
    this.element.setAttribute('role','dialog');
    this.element.setAttribute('aria-modal','true');
    this.element.tabIndex = -1;
    this.element.className = 'neon-modal';
    const header = document.createElement('h2');
    header.textContent = title;
    this.element.appendChild(header);
  }
  getElement(): HTMLDivElement { return this.element; }
}
