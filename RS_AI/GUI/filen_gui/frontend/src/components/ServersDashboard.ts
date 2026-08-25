import { invoke } from '@tauri-apps/api/core';

export class ServersDashboard {
  element: HTMLDivElement;
  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'servers-dashboard';
    this.element.innerHTML = `<h3>🖥️ Servers</h3><div class="server-status"></div><div class="log-console"></div>`;
    this.loadStatfs();
  }
  log(msg: string): void {
    const consoleEl = this.element.querySelector('.log-console');
    if (consoleEl) consoleEl.textContent += `[${new Date().toLocaleTimeString()}] ${msg}\n`;
  }
  async loadStatfs(): Promise<void> {
    const status = this.element.querySelector('.server-status');
    this.log('mounting…');
    try {
      const [used, max] = await invoke<[string, string]>('auth_statfs_terminal', { account: null });
      if (status) {
        status.innerHTML = `
          <div class="stat-card">
            <div class="stat-line">Đã dùng: ${used}</div>
            <div class="stat-line">Tổng dung lượng: ${max}</div>
          </div>`;
      }
      this.log(`statfs OK → used=${used}, max=${max}`);
    } catch (e) {
      if (status) {
        status.innerHTML = `<div class="placeholder">Cần đăng nhập để xem thông tin máy chủ</div>`;
      }
      this.log(`statfs fail → ${String(e)}`);
    }
  }
  getElement(): HTMLDivElement { return this.element; }
}
