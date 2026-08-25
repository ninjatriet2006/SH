import { formatSize } from '../../features/format';

export class PaneStatusBar {
  public element: HTMLDivElement;

  private selectedItemsLabel: HTMLSpanElement;
  private totalItemsLabel: HTMLSpanElement;
  private freeSpaceLabel: HTMLSpanElement;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'pane-status-bar';

    this.selectedItemsLabel = document.createElement('span');
    this.selectedItemsLabel.className = 'status-selected';
    this.selectedItemsLabel.textContent = ''; // Initially empty

    this.totalItemsLabel = document.createElement('span');
    this.totalItemsLabel.className = 'status-total';
    this.totalItemsLabel.textContent = ''; // Removed total items display as requested

    this.freeSpaceLabel = document.createElement('span');
    this.freeSpaceLabel.className = 'status-free-space';
    this.freeSpaceLabel.textContent = 'Trống: ?';

    this.element.appendChild(this.selectedItemsLabel);
    this.element.appendChild(this.totalItemsLabel);
    
    // Add spacer to push free space to the right
    const spacer = document.createElement('div');
    spacer.className = 'status-spacer';
    this.element.appendChild(spacer);

    this.element.appendChild(this.freeSpaceLabel);
  }

  public updateSelection(count: number, size: number) {
    if (count > 0) {
      this.selectedItemsLabel.textContent = `${count} mục được chọn (${formatSize(size)}) — `;
    } else {
      this.selectedItemsLabel.textContent = '';
    }
  }

  public updateTotal(_count: number, _size: number) {
    // Hidden as per user request
    this.totalItemsLabel.textContent = '';
  }

  public updateSpace(about: { total?: number, used?: number, free?: number }) {
    if (about.total && about.used !== undefined && about.free !== undefined) {
      this.freeSpaceLabel.textContent = `Đã dùng: ${formatSize(about.used)} / ${formatSize(about.total)} (Trống: ${formatSize(about.free)})`;
    } else if (about.free && about.free > 0) {
      this.freeSpaceLabel.textContent = `Trống: ${formatSize(about.free)}`;
    } else if (about.used && about.used > 0) {
      this.freeSpaceLabel.textContent = `Đã dùng: ${formatSize(about.used)}`;
    } else {
      this.freeSpaceLabel.textContent = '';
    }
  }
}
