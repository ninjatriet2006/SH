import { debugStore, LogEntry } from '../services/debugStore';

export class DebugView {
  private container: HTMLElement;
  private logContainer!: HTMLDivElement;
  private isAutoScroll: boolean = true;

  constructor() {
    this.container = document.getElementById('view-debug') as HTMLElement;
    if (!this.container) return;

    this.container.innerHTML = `
      <div class="pane-toolbar">
        <div class="nemo-nav-group">
          <button class="btn btn-primary" id="btn-debug-clear">🗑️ Xoá Log</button>
          <label style="display:flex; align-items:center; gap: 5px; margin-left: 10px; cursor: pointer;">
            <input type="checkbox" id="cb-debug-autoscroll" checked> Tự động cuộn (Auto-scroll)
          </label>
        </div>
        <div class="nemo-path-group">
          <div style="font-weight: bold; margin-left: 10px;">Debug / Data Flow Tracker</div>
        </div>
      </div>
      <div id="debug-log-container" style="flex: 1; background: #1e1e1e; color: #d4d4d4; font-family: 'Courier New', Courier, monospace; font-size: 12px; padding: 10px; overflow-y: auto; border-radius: 4px; border: 1px solid var(--border-color);">
      </div>
    `;

    this.logContainer = this.container.querySelector('#debug-log-container') as HTMLDivElement;

    this.container.querySelector('#btn-debug-clear')?.addEventListener('click', () => {
      debugStore.clear();
    });

    const autoScrollCb = this.container.querySelector('#cb-debug-autoscroll') as HTMLInputElement;
    if (autoScrollCb) {
      autoScrollCb.addEventListener('change', (e) => {
        this.isAutoScroll = (e.target as HTMLInputElement).checked;
      });
    }

    // Subscribe to logs
    debugStore.subscribe(this.renderLogs.bind(this));
    
    // Initial render
    this.renderLogs(debugStore.getLogs());
  }

  private renderLogs(logs: LogEntry[]) {
    if (!this.logContainer) return;
    
    this.logContainer.innerHTML = '';
    
    if (logs.length === 0) {
      this.logContainer.innerHTML = '<div style="color: #6a9955;">// Không có dữ liệu log...</div>';
      return;
    }

    const fragment = document.createDocumentFragment();
    for (const log of logs) {
      const line = document.createElement('div');
      line.style.marginBottom = '4px';
      line.style.wordBreak = 'break-all';
      
      const timeSpan = document.createElement('span');
      timeSpan.style.color = '#569cd6';
      timeSpan.textContent = `[${log.timestamp}] `;
      
      const typeSpan = document.createElement('span');
      typeSpan.style.fontWeight = 'bold';
      typeSpan.style.color = log.type === 'API' ? '#c586c0' : log.type === 'TRANSFER' ? '#ce9178' : '#dcdcaa';
      typeSpan.textContent = `[${log.type}] `;
      
      const actionSpan = document.createElement('span');
      actionSpan.style.color = '#4ec9b0';
      actionSpan.textContent = `${log.action} `;
      
      const detailSpan = document.createElement('span');
      detailSpan.style.color = '#d4d4d4';
      detailSpan.textContent = log.detail;
      
      line.appendChild(timeSpan);
      line.appendChild(typeSpan);
      line.appendChild(actionSpan);
      line.appendChild(detailSpan);
      
      fragment.appendChild(line);
    }
    
    this.logContainer.appendChild(fragment);
    
    if (this.isAutoScroll) {
      this.logContainer.scrollTop = this.logContainer.scrollHeight;
    }
  }
}
