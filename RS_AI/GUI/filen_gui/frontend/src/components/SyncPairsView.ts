import { appState } from '../store';

export class SyncPairsView {
  element: HTMLDivElement;
  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'sync-pairs-view';
    this.render();
  }
  render(): void {
    const files = appState.explorer?.rightFiles ?? [];
    if (files.length === 0) {
      this.element.innerHTML = `<h3>🔄 Đồng bộ</h3><div class="placeholder">Chưa có cặp đồng bộ</div>`;
      return;
    }
    const cards = files
      .map(
        (f) => `
      <div class="sync-card">
        <span class="sync-card-name">${f.name}</span>
        <span class="sync-card-status">Cloud ✓</span>
      </div>`
      )
      .join('');
    this.element.innerHTML = `<h3>🔄 Đồng bộ</h3><div class="sync-cards">${cards}</div>`;
  }
  getElement(): HTMLDivElement { return this.element; }
}
