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
      progressPct = (task.progress || 0) * 100;
      statusText = `${formatSize(task.speed)}/s — ${progressPct.toFixed(1)}%`;
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
    return card;
  }
}
