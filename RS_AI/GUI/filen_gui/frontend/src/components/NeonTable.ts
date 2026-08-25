export class NeonTable {
  element: HTMLTableElement;
  constructor(headers: string[] = []) {
    this.element = document.createElement('table');
    this.element.setAttribute('role','table');
    this.element.className = 'neon-table';
    if (headers.length) {
      const thead = this.element.createTHead();
      const row = thead.insertRow();
      headers.forEach(h => {
        const th = document.createElement('th');
        th.textContent = h;
        row.appendChild(th);
      });
    }
  }
  getElement(): HTMLTableElement { return this.element; }
}
