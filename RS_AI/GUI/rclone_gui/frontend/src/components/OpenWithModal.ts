import { invoke } from '@tauri-apps/api/core';
import { OperationModal } from './OperationModal';

export interface DesktopApp {
  name: string;
  exec: string;
  icon: string;
  mime_types: string[];
}

export class OpenWithModal {
  private modal: OperationModal;
  private apps: DesktopApp[] = [];
  private filteredApps: DesktopApp[] = [];
  private selectedApp: DesktopApp | null = null;
  private targetPath: string;

  constructor(targetPath: string) {
    this.targetPath = targetPath;
    this.modal = new OperationModal('Open With...', this.getHtml());
  }

  private getHtml(): string {
    return `
      <div class="open-with-container" style="display: flex; flex-direction: column; gap: 10px; height: 300px;">
        <input type="text" id="open-with-search" placeholder="Search applications..." style="padding: 8px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--surface-bg); color: var(--text-color);">
        <div id="open-with-list" style="flex: 1; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px; background: var(--surface-bg);">
          <div style="padding: 10px; text-align: center; color: var(--text-muted);">Loading applications...</div>
        </div>
      </div>
    `;
  }

  public async open(): Promise<void> {
    this.modal.open();
    
    // Disable confirm button until an app is selected
    const confirmBtn = this.modal.getElement().querySelector('.confirm') as HTMLButtonElement;
    if (confirmBtn) confirmBtn.disabled = true;

    // Setup search
    const searchInput = this.modal.getElement().querySelector('#open-with-search') as HTMLInputElement;
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        const query = (e.target as HTMLInputElement).value.toLowerCase();
        this.filteredApps = this.apps.filter(app => app.name.toLowerCase().includes(query));
        this.renderList();
      });
    }

    // Handle confirm
    confirmBtn?.addEventListener('click', async () => {
      if (this.selectedApp) {
        try {
          await invoke('sys_open_with', { path: this.targetPath, execCmd: this.selectedApp.exec });
        } catch (e) {
          console.warn('sys_open_with failed:', e);
        }
      }
      this.modal.close();
    });

    // Load apps
    try {
      this.apps = await invoke<DesktopApp[]>('sys_list_apps');
      this.filteredApps = [...this.apps];
      this.renderList();
    } catch (e) {
      console.warn('sys_list_apps failed:', e);
      const listEl = this.modal.getElement().querySelector('#open-with-list');
      if (listEl) {
        listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: red;">Failed to load applications.</div>`;
      }
    }
  }

  private renderList() {
    const listEl = this.modal.getElement().querySelector('#open-with-list');
    if (!listEl) return;

    if (this.filteredApps.length === 0) {
      listEl.innerHTML = `<div style="padding: 10px; text-align: center; color: var(--text-muted);">No applications found.</div>`;
      return;
    }

    listEl.innerHTML = '';
    this.filteredApps.forEach(app => {
      const item = document.createElement('div');
      item.style.padding = '8px 12px';
      item.style.cursor = 'pointer';
      item.style.borderBottom = '1px solid var(--border-color)';
      item.style.display = 'flex';
      item.style.alignItems = 'center';
      item.style.gap = '10px';
      
      // Simple icon placeholder if no valid icon
      item.innerHTML = `
        <div style="font-size: 14px; color: var(--text-color);">${app.name}</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-left: auto; max-width: 150px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${app.exec}</div>
      `;

      item.addEventListener('mouseover', () => {
        if (this.selectedApp !== app) item.style.background = 'var(--hover-bg)';
      });
      item.addEventListener('mouseout', () => {
        if (this.selectedApp !== app) item.style.background = 'transparent';
      });

      item.addEventListener('click', () => {
        // Deselect previous
        const prev = listEl.querySelector('[data-selected="true"]') as HTMLElement;
        if (prev) {
          prev.removeAttribute('data-selected');
          prev.style.background = 'transparent';
        }
        
        // Select new
        item.setAttribute('data-selected', 'true');
        item.style.background = 'var(--primary-color)';
        item.style.color = '#fff';
        this.selectedApp = app;

        // Enable confirm
        const confirmBtn = this.modal.getElement().querySelector('.confirm') as HTMLButtonElement;
        if (confirmBtn) confirmBtn.disabled = false;
      });

      listEl.appendChild(item);
    });
  }
}
