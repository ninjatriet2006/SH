import { appState } from '../store';

export class RecentsView {
  element: HTMLDivElement;
  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'recents-view';
    this.render();
  }
  render(): void {
    const items = appState.activityLog || [];
    if (items.length === 0) {
      this.element.innerHTML = `<h3>🕘 Nhật ký hoạt động</h3><div class="placeholder">Chưa có dữ liệu</div>`;
      return;
    }
    const rows = items
      .map((item) => {
        const timeStr = new Date(item.timestamp).toLocaleString();
        return `<tr>
          <td style="width: 150px; color: var(--text-muted);">${timeStr}</td>
          <td style="width: 120px; font-weight: bold; color: var(--text-color);">${item.action}</td>
          <td style="color: var(--text-color); word-break: break-all;">${item.details}</td>
        </tr>`;
      })
      .join('');
    this.element.innerHTML = `
      <h3>🕘 Nhật ký hoạt động</h3>
      <table class="file-table" style="table-layout: fixed; width: 100%;">
        <thead>
          <tr>
            <th style="width: 150px;">Thời gian</th>
            <th style="width: 120px;">Hành động</th>
            <th>Chi tiết</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>`;
  }
  getElement(): HTMLDivElement { return this.element; }
}
