export class NeonInput {
  element: HTMLInputElement;
  constructor(placeholder: string = "") {
    this.element = document.createElement('input');
    this.element.placeholder = placeholder;
    this.element.className = 'neon-input';
  }
  getElement(): HTMLInputElement { return this.element; }
}
