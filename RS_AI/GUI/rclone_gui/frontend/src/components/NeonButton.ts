export class NeonButton {
  element: HTMLButtonElement;
  constructor(label: string, onClick?: () => void) {
    this.element = document.createElement('button');
    this.element.textContent = label;
    if (!label) this.element.setAttribute('aria-label','button');
    this.element.className = 'neon-button';
    if (onClick) this.element.addEventListener('click', onClick);
  }
  getElement(): HTMLButtonElement {
    return this.element;
  }
}
