import { OperationModal } from './OperationModal';
import { type Pane } from '../services/explorerStore';
import type { FileItem } from '../store';
import * as fileOps from '../services/fileOps';
import { invoke } from '@tauri-apps/api/core';
import { emblemStore } from '../services/emblemStore';
import { formatSize, formatDate, escapeHtml } from '../features/format';

const AVAILABLE_EMOJIS = ['⭐', '❤️', '🔒', '🔥', '⚠️', '✅', '📌', '🎵', '📷', '💼', '🚀', '💡'];

export class PropertiesModal {
  private modal: OperationModal;
  private file: FileItem;
  private fullPath: string;
  private stats: fileOps.StatInfo | null = null;
  private permOctalStr = '0000';
  private apps: any[] = [];

  constructor(file: FileItem, fullPath: string, _pane: Pane) {
    this.file = file;
    this.fullPath = fullPath;
    
    // Start with a loading HTML while fetching stats
    this.modal = new OperationModal(`${file.name} Properties`, this.getLoadingHtml());
  }

  public async open(): Promise<void> {
    this.modal.open();
    
    if (this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) {
      try {
        const [stats, apps] = await Promise.all([
          fileOps.statAdvanced(this.fullPath),
          invoke('sys_list_apps').catch(() => []) as Promise<any[]>
        ]);
        this.stats = stats;
        this.apps = apps;
      } catch (e) {
        console.warn('Failed to get properties data', e);
      }
    }

    // Refresh UI with actual content
    this.modal.getBody().innerHTML = this.getContentHtml();
    this.attachEventListeners();
  }

  private getLoadingHtml(): string {
    return `<div style="padding: 20px; text-align: center; color: var(--text-muted);">Loading properties...</div>`;
  }

  private getContentHtml(): string {
    const type = this.file.is_dir ? 'Folder' : this.file.file_type || 'Unknown';
    let sizeStr = this.file.is_dir ? '-' : formatSize(this.file.size);
    let contentStr = '-';
    let uid = 0;
    let gid = 0;

    if (this.stats) {
      sizeStr = formatSize(this.stats.size);
      if (this.file.is_dir) {
        contentStr = `${this.stats.file_count} files, ${this.stats.dir_count} folders`;
      }
      this.permOctalStr = (this.stats.permissions & 0o777).toString(8).padStart(4, '0');
      uid = this.stats.uid;
      gid = this.stats.gid;
    }

    return `
      <div class="tabs-header">
        <button class="tab-btn active" data-tab="basic">Basic</button>
        ${(this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) ? `<button class="tab-btn" data-tab="permissions">Permissions</button>` : ''}
        ${!this.file.is_dir && (this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) ? `<button class="tab-btn" data-tab="openwith">Open With</button>` : ''}
        <button class="tab-btn" data-tab="emblems">Emblems</button>
      </div>

      <div class="tab-pane active" id="tab-basic">
        <div class="prop-row"><span>Name</span><span>${escapeHtml(this.file.name)}</span></div>
        <div class="prop-row"><span>Type</span><span>${escapeHtml(type)}</span></div>
        <div class="prop-row"><span>Size</span><span>${sizeStr}</span></div>
        ${this.file.is_dir && this.stats ? `<div class="prop-row"><span>Contents</span><span>${escapeHtml(contentStr)}</span></div>` : ''}
        <div class="prop-row"><span>Path</span><span>${escapeHtml(this.fullPath)}</span></div>
        <div class="prop-row"><span>Modified</span><span>${escapeHtml(formatDate(this.file.mod_time))}</span></div>
      </div>

      ${(this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) ? `
      <div class="tab-pane" id="tab-permissions">
        <div class="prop-row" style="margin-bottom: 15px;">
          <span>Ownership</span>
          <span style="display: flex; gap: 10px; align-items: center;">
            <label>UID: <input type="number" id="prop-uid" value="${uid}" style="width: 60px; padding: 2px;" disabled></label>
            <label>GID: <input type="number" id="prop-gid" value="${gid}" style="width: 60px; padding: 2px;" disabled></label>
          </span>
        </div>
        
        <div class="permissions-grid">
          <div class="header"></div>
          <div class="header">Read</div>
          <div class="header">Write</div>
          <div class="header">Execute</div>
          
          <div class="row-label">Owner</div>
          <div class="check-cell"><input type="checkbox" data-bit="0400" ${this.hasPerm(0o400)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0200" ${this.hasPerm(0o200)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0100" ${this.hasPerm(0o100)} disabled></div>
          
          <div class="row-label">Group</div>
          <div class="check-cell"><input type="checkbox" data-bit="0040" ${this.hasPerm(0o040)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0020" ${this.hasPerm(0o020)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0010" ${this.hasPerm(0o010)} disabled></div>
          
          <div class="row-label">Others</div>
          <div class="check-cell"><input type="checkbox" data-bit="0004" ${this.hasPerm(0o004)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0002" ${this.hasPerm(0o002)} disabled></div>
          <div class="check-cell"><input type="checkbox" data-bit="0001" ${this.hasPerm(0o001)} disabled></div>
        </div>
        
        <div style="font-size: 13px; color: var(--text-muted); text-align: right;">
          Octal: <span id="prop-octal-preview">${this.permOctalStr}</span>
        </div>
      </div>
      ` : ''}

      ${!this.file.is_dir && (this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) ? `
      <div class="tab-pane" id="tab-openwith">
        <div style="font-size: 13px; margin-bottom: 10px;">Select an application to open this file:</div>
        <div class="apps-list" style="max-height: 150px; overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px; margin-bottom: 15px; background: var(--bg-color);">
          ${this.apps.map(app => `
            <div class="app-item" data-exec="${app.exec.replace(/%[a-zA-Z]/g, '').trim().replace(/"/g, '&quot;')}" style="padding: 6px 10px; cursor: pointer; border-bottom: 1px solid var(--border-color); display: flex; align-items: center; gap: 8px;">
              <span>${app.name}</span>
            </div>
          `).join('')}
          ${this.apps.length === 0 ? '<div style="padding: 10px; color: var(--text-muted);">No applications found.</div>' : ''}
        </div>
        <div class="open-with-container">
          <label style="font-size: 13px; color: var(--text-color);">Or custom command:</label>
          <input type="text" id="prop-open-cmd" placeholder="e.g. code, vlc, gedit..." style="padding: 6px; border-radius: 4px; border: 1px solid var(--border-color); background: var(--bg-color); color: var(--text-color); font-size: 13px;">
          <button id="prop-open-btn" style="padding: 6px 12px; background: var(--primary-color); color: white; border: none; border-radius: 4px; cursor: pointer; align-self: flex-start; margin-top: 8px;">Open Now</button>
        </div>
      </div>
      ` : ''}

      <div class="tab-pane" id="tab-emblems">
        <div style="font-size: 13px; margin-bottom: 10px;">Select emblems to attach to this file/folder:</div>
        <div class="emblems-grid" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(40px, 1fr)); gap: 10px; margin-bottom: 15px;">
          ${AVAILABLE_EMOJIS.map(emoji => {
            const isActive = emblemStore.getEmblems(this.fullPath).includes(emoji);
            return `<div class="emblem-item ${isActive ? 'active' : ''}" data-emoji="${emoji}" style="font-size: 24px; text-align: center; cursor: pointer; padding: 4px; border-radius: 4px; border: 1px solid ${isActive ? 'var(--colors-primary)' : 'transparent'}; background: ${isActive ? 'var(--colors-surface-glass)' : 'transparent'};">${emoji}</div>`;
          }).join('')}
        </div>
      </div>
    `;
  }

  private attachEventListeners() {
    const el = this.modal.getElement();
    
    // Tabs logic
    const tabBtns = el.querySelectorAll('.tab-btn');
    const tabPanes = el.querySelectorAll('.tab-pane');
    
    tabBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        tabBtns.forEach(b => b.classList.remove('active'));
        tabPanes.forEach(p => p.classList.remove('active'));
        
        btn.classList.add('active');
        const targetId = `tab-${(btn as HTMLElement).dataset.tab}`;
        const targetPane = el.querySelector(`#${targetId}`);
        if (targetPane) targetPane.classList.add('active');
      });
    });

    // Checkbox logic for permissions
    const checkboxes = el.querySelectorAll('.permissions-grid input[type="checkbox"]');
    const octalPreview = el.querySelector('#prop-octal-preview');
    
    checkboxes.forEach(cb => {
      cb.addEventListener('change', () => {
        let newPerm = 0;
        checkboxes.forEach(chk => {
          if ((chk as HTMLInputElement).checked) {
            const bit = parseInt((chk as HTMLInputElement).dataset.bit || '0', 8);
            newPerm |= bit;
          }
        });
        this.permOctalStr = newPerm.toString(8).padStart(4, '0');
        if (octalPreview) octalPreview.textContent = this.permOctalStr;
      });
    });

    // Open With logic
    const openBtn = el.querySelector('#prop-open-btn');
    const cmdInp = el.querySelector('#prop-open-cmd') as HTMLInputElement;

    // App list selection
    const appItems = el.querySelectorAll('.app-item');
    appItems.forEach(item => {
      item.addEventListener('click', () => {
        appItems.forEach(i => i.classList.remove('selected'));
        item.classList.add('selected');
        if (cmdInp) {
          cmdInp.value = (item as HTMLElement).dataset.exec || '';
        }
      });
      // Double click to open immediately
      item.addEventListener('dblclick', async () => {
        const app = (item as HTMLElement).dataset.exec;
        if (app) {
          try {
            await invoke('sys_open_with', { path: this.fullPath, app });
            this.modal.close();
          } catch (e) {
            console.warn('Failed to open', e);
          }
        }
      });
    });

    if (openBtn) {
      openBtn.addEventListener('click', async () => {
        const app = cmdInp?.value?.trim();
        if (app) {
          try {
            await invoke('sys_open_with', { path: this.fullPath, app });
            this.modal.close();
          } catch (e) {
            console.warn('Failed to open with custom app', e);
          }
        }
      });
    }

    // Emblems logic
    const emblemItems = el.querySelectorAll('.emblem-item');
    emblemItems.forEach(item => {
      item.addEventListener('click', () => {
        const emoji = (item as HTMLElement).dataset.emoji;
        if (emoji) {
          emblemStore.toggleEmblem(this.fullPath, emoji);
          
          // Toggle UI state locally
          const isActive = emblemStore.getEmblems(this.fullPath).includes(emoji);
          if (isActive) {
            item.classList.add('active');
            (item as HTMLElement).style.border = '1px solid var(--colors-primary)';
            (item as HTMLElement).style.background = 'var(--colors-surface-glass)';
          } else {
            item.classList.remove('active');
            (item as HTMLElement).style.border = '1px solid transparent';
            (item as HTMLElement).style.background = 'transparent';
          }
        }
      });
    });

    // Save button logic (Confirm)
    const confirmBtn = el.querySelector('.confirm') as HTMLButtonElement;
    if (confirmBtn) {
      confirmBtn.addEventListener('click', async () => {
        if (this.fullPath.startsWith('Local::') || !this.fullPath.includes('::')) {
          // 1. Save Permissions
          if (this.permOctalStr && this.stats) {
            const oldOctal = (this.stats.permissions & 0o777).toString(8).padStart(4, '0');
            if (this.permOctalStr !== oldOctal) {
              const mode = parseInt(this.permOctalStr, 8);
              try {
                await fileOps.chmod(this.fullPath, mode);
              } catch (e) {
                console.warn('chmod failed', e);
              }
            }
          }
          
          // 2. Save Ownership
          const uidInp = el.querySelector('#prop-uid') as HTMLInputElement;
          const gidInp = el.querySelector('#prop-gid') as HTMLInputElement;
          if (uidInp && gidInp && this.stats) {
            const newUid = parseInt(uidInp.value, 10);
            const newGid = parseInt(gidInp.value, 10);
            if (!isNaN(newUid) && !isNaN(newGid) && (newUid !== this.stats.uid || newGid !== this.stats.gid)) {
              try {
                await fileOps.chown(this.fullPath, newUid, newGid);
              } catch (e) {
                console.warn('chown failed', e);
              }
            }
          }
        }
        this.modal.close();
      });
    }
  }

  private hasPerm(bit: number): string {
    if (!this.stats) return '';
    return (this.stats.permissions & bit) ? 'checked' : '';
  }
}
