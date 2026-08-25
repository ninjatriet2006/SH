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
    this.totalItemsLabel.textContent = '0 mục (0 B)';

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

  public updateTotal(count: number, size: number) {
    this.totalItemsLabel.textContent = `${count} mục (${formatSize(size)})`;
  }

  public updateFreeSpace(bytes: number) {
    if (bytes > 0) {
      this.freeSpaceLabel.textContent = `Trống: ${formatSize(bytes)}`;
    } else {
      this.freeSpaceLabel.textContent = '';
    }
  }
}
