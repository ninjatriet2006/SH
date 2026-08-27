import { transferManager, type TransferTask } from '../features/transferManager';
import { formatSize } from '../features/format';

export class TransferDrawer {
  private toggleBtn: HTMLElement;
  private body: HTMLElement;
  private label: HTMLElement;
  private isOpen = false;

  constructor() {
    this.toggleBtn = document.getElementById('drawer-toggle')!;
    this.body = document.getElementById('drawer-body')!;
    this.label = document.getElementById('drawer-label')!;

    this.toggleBtn.addEventListener('click', () => this.toggle());
    
    transferManager.onUpdate = () => this.render();

    window.addEventListener('open-transfer-drawer', () => {
      if (!this.isOpen) this.toggle();
    });

    // Add resizer for drag-to-resize
    const drawer = document.getElementById('transfer-drawer')!;
    drawer.style.position = 'relative';
    const resizer = document.createElement('div');
    resizer.className = 'drawer-resizer';
    resizer.style.height = '6px';
    resizer.style.cursor = 'ns-resize';
    resizer.style.background = 'transparent';
    resizer.style.position = 'absolute';
    resizer.style.top = '-3px';
    resizer.style.left = '0';
    resizer.style.right = '0';
    resizer.style.zIndex = '10';

    let startY = 0;
    let startHeight = 0;

    resizer.addEventListener('mousedown', (e) => {
      e.preventDefault();
      startY = e.clientY;
      startHeight = this.body.offsetHeight;

      const onMouseMove = (ev: MouseEvent) => {
        if (!this.isOpen) return; // Only resize when open
        const dy = startY - ev.clientY;
        const newHeight = Math.max(100, startHeight + dy);
        this.body.style.maxHeight = `${newHeight}px`;
        this.body.style.height = `${newHeight}px`;
      };

      const onMouseUp = () => {
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
      };

      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    });

    drawer.appendChild(resizer);
  }

  toggle() {
    this.isOpen = !this.isOpen;
    if (this.isOpen) {
      document.getElementById('transfer-drawer')!.classList.add('open');
    } else {
      document.getElementById('transfer-drawer')!.classList.remove('open');
    }
    this.render();
  }

  render() {
    const tasks = transferManager.getAllTasks();
    const activeCount = tasks.filter(t => t.status === 'queued' || t.status === 'running').length;
    
    // Update label
    const prefix = this.isOpen ? '⬇️' : '⬆️';
    this.label.textContent = `${prefix} Transfer (${activeCount}) — ${this.isOpen ? 'bấm để đóng' : 'bấm để mở'}`;
    
    if (tasks.length === 0) {
      this.body.innerHTML = '<div class="drawer-placeholder">Hàng đợi transfer (upload/download/copy/move) — trống</div>';
      return;
    }

    // Render list
    this.body.innerHTML = '';
    const header = document.createElement('div');
    header.style.display = 'flex';
    header.style.justifyContent = 'flex-end';
    header.style.marginBottom = '12px';
    const clearBtn = document.createElement('button');
    clearBtn.textContent = 'Dọn dẹp lịch sử';
    clearBtn.className = 'btn';
    clearBtn.onclick = () => transferManager.removeFinished();
    header.appendChild(clearBtn);
    this.body.appendChild(header);

    const list = document.createElement('div');
    list.className = 'transfer-list';
    
    for (const task of tasks.reverse()) {
      const card = this.renderTaskCard(task);
      list.appendChild(card);
    }
    
    this.body.appendChild(list);
  }

  private renderTaskCard(task: TransferTask): HTMLElement {
    const card = document.createElement('div');
    card.className = `transfer-card ${task.status}`;
    
    const infoRow = document.createElement('div');
    infoRow.className = 'transfer-info';
    
    const title = document.createElement('span');
    title.className = 'transfer-title';
    title.textContent = `[${task.kind.toUpperCase()}] ${task.name}`;
    
    const status = document.createElement('span');
    status.className = 'transfer-status';
    
    let statusText = '';
    let progressPct = 0;
    if (task.status === 'running') {
      if (task.totalBytes && task.totalBytes > 0) {
        progressPct = (task.bytesDone / task.totalBytes) * 100;
      } else {
        progressPct = (task.progress || 0) * 100;
      }
      statusText = `${formatSize(task.speed || 0)}/s — ${progressPct.toFixed(1)}%`;
    } else if (task.status === 'queued') {
      statusText = 'Đang chờ...';
    } else if (task.status === 'done') {
      statusText = 'Hoàn tất';
      progressPct = 100;
    } else if (task.status === 'error') {
      statusText = `Lỗi: ${task.error}`;
      progressPct = 100;
    } else if (task.status === 'cancelled') {
      statusText = 'Đã hủy';
    }
    
    status.textContent = statusText;
    infoRow.appendChild(title);
    infoRow.appendChild(status);
    
    const progressBg = document.createElement('div');
    progressBg.className = 'transfer-progress-bg';
    
    const progressFill = document.createElement('div');
    progressFill.className = `transfer-progress-fill ${task.status}`;
    progressFill.style.width = `${progressPct}%`;
    progressBg.appendChild(progressFill);
    
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn-icon';
    cancelBtn.innerHTML = '❌';
    cancelBtn.title = 'Hủy tiến trình';
    cancelBtn.style.marginLeft = '12px';
    cancelBtn.style.padding = '4px 8px';
    cancelBtn.style.backgroundColor = 'transparent';
    if (task.status === 'queued' || task.status === 'running') {
      cancelBtn.onclick = () => transferManager.cancel(task.id);
    } else {
      cancelBtn.style.visibility = 'hidden';
    }
    
    const flexRow = document.createElement('div');
    flexRow.style.display = 'flex';
    flexRow.style.alignItems = 'center';
    flexRow.style.width = '100%';
    
    const textCol = document.createElement('div');
    textCol.style.flex = '1';
    textCol.appendChild(infoRow);
    textCol.appendChild(progressBg);
    
    flexRow.appendChild(textCol);
    flexRow.appendChild(cancelBtn);
    
    card.appendChild(flexRow);

    // Render tree of transferring files
    if (task.status === 'running' && task.transferringFiles && task.transferringFiles.length > 0) {
      const treeContainer = document.createElement('div');
      treeContainer.style.marginTop = '8px';
      treeContainer.style.paddingLeft = '12px';
      treeContainer.style.borderLeft = '1px dashed var(--border-color)';
      treeContainer.style.display = 'flex';
      treeContainer.style.flexDirection = 'column';
      treeContainer.style.gap = '4px';

      for (const file of task.transferringFiles) {
        const fileRow = document.createElement('div');
        fileRow.style.display = 'flex';
        fileRow.style.alignItems = 'center';
        fileRow.style.justifyContent = 'space-between';
        fileRow.style.fontSize = '12px';
        fileRow.style.color = 'var(--text-secondary)';
        
        const nameSpan = document.createElement('span');
        nameSpan.style.whiteSpace = 'nowrap';
        nameSpan.style.overflow = 'hidden';
        nameSpan.style.textOverflow = 'ellipsis';
        nameSpan.style.maxWidth = '50%';
        nameSpan.textContent = `📄 ${file.name}`;
        
        const statsSpan = document.createElement('span');
        statsSpan.style.whiteSpace = 'nowrap';
        const eta = file.eta !== undefined && file.eta >= 0 ? `${file.eta}s` : '-';
        statsSpan.innerHTML = `<strong>${file.percentage}%</strong> &middot; ${formatSize(file.speed || 0)}/s &middot; ETA: ${eta}`;
        
        fileRow.appendChild(nameSpan);
        fileRow.appendChild(statsSpan);
        treeContainer.appendChild(fileRow);
      }
      card.appendChild(treeContainer);
    }

    return card;
  }
}
