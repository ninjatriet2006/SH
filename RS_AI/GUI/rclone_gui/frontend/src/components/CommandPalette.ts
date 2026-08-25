export class CommandPalette {
  element: HTMLDivElement;
  constructor(commands: string[]) {
    this.element = document.createElement('div');
    this.element.className = 'command-palette';
    this.element.innerHTML = `<input placeholder='Command...'/><ul>${commands.map(c => `<li>${c}</li>`).join('')}</ul>`;
  }
  getElement(): HTMLDivElement { return this.element; }
}
