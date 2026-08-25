export interface ContextMenuItem {
  label?: string;
  disabled?: boolean;
  separator?: boolean;
}

export class ContextMenu {
  element: HTMLDivElement;
  constructor(items: (string | ContextMenuItem)[]) {
    this.element = document.createElement('div');
    this.element.className = 'context-menu';
    this.element.innerHTML = items
      .map((i) => {
        const item = typeof i === 'string' ? { label: i } : i;
        if (item.separator) {
          return `<div class='separator'></div>`;
        }
        const cls = item.disabled ? 'item disabled' : 'item';
        return `<div class='${cls}'>${item.label}</div>`;
      })
      .join('');
  }
  getElement(): HTMLDivElement { return this.element; }
}