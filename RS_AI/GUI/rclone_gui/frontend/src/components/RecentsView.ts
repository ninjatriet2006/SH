/*
[INTEGRITY NOTES]
- Mục đích: Hiển thị nhật ký hoạt động (activity log) mà `logActivity()` ghi lại.
- Trách nhiệm: Render bảng thời gian / hành động / chi tiết từ `appState.activityLog`.
- Tương tác: Mount vào #view-activity bởi main.ts khi người dùng mở tab Nhật ký.
*/
import { appState } from '../store';
import { escapeHtml } from '../features/format';

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
          <td style="width: 150px; color: var(--colors-text-muted);">${escapeHtml(timeStr)}</td>
          <td style="width: 140px; font-weight: bold; color: var(--colors-text-primary);">${escapeHtml(item.action)}</td>
          <td style="color: var(--colors-text-primary); word-break: break-all;">${escapeHtml(item.details)}</td>
        </tr>`;
      })
      .join('');
    this.element.innerHTML = `
      <h3>🕘 Nhật ký hoạt động</h3>
      <table class="file-table" style="table-layout: fixed; width: 100%;">
        <thead>
          <tr>
            <th style="width: 150px;">Thời gian</th>
            <th style="width: 140px;">Hành động</th>
            <th>Chi tiết</th>
          </tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>`;
  }
  getElement(): HTMLDivElement { return this.element; }
}
