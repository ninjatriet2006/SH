/*
[INTEGRITY NOTES]
- Mục đích: Quản lý giao diện thẻ (tab) Remotes (Hiển thị danh sách, thêm, sửa, xóa cấu hình đám mây).
- Trách nhiệm: Lắng nghe sự kiện trang Remotes, gọi API backend để lấy (fetch) và hiển thị dữ liệu lên giao diện.
- Tương tác: Làm việc chặt chẽ với `bridge/remote_api.ts`, `bridge/config_api.ts` và được khởi tạo ở `main.ts`.
*/


import { formatSize, escapeHtml } from './format';
import { listRemotes, getProviders, ProviderInfo, ProviderOption, RemoteConfig, createRemote, updateRemote, deleteRemote, getBackendFeatures, getAbout, getSize } from '../../../bridge/remote_api.ts';
import { getConfigContent, setConfigContent, reorderConfig } from '../../../bridge/config_api.ts';
import { upgradeSelectToCustomDropdown } from './customDropdown.ts';

interface CapacityResult {
    html: string;
    title: string;
}

const capacityCache = new Map<string, { data: CapacityResult, timestamp: number }>();
const CACHE_TTL = 5 * 60 * 1000; // Thời gian sống của cache (5 phút)

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('Timeout')), ms);
        promise
            .then(value => {
                clearTimeout(timer);
                resolve(value);
            })
            .catch(reason => {
                clearTimeout(timer);
                reject(reason);
            });
    });
}

const getConcurrencyLimit = () => {
    const nav = navigator as any;
    if (nav.connection && nav.connection.effectiveType) {
        const type = nav.connection.effectiveType;
        if (type === '4g') return 4;
        if (type === '3g') return 2;
        return 1;
    }
    return 3;
};

class ConcurrencyQueue {
    private queue: (() => Promise<void>)[] = [];
    private activeCount = 0;
    
    add(task: () => Promise<void>) {
        this.queue.push(task);
        this.runNext();
    }

    private runNext() {
        if (this.activeCount < getConcurrencyLimit() && this.queue.length > 0) {
            const task = this.queue.shift();
            if (task) {
                this.activeCount++;
                task().finally(() => {
                    this.activeCount--;
                    this.runNext();
                });
            }
        }
    }
}

const capacityQueue = new ConcurrencyQueue();

const OPTION_TRANSLATIONS: Record<string, string> = {
  client_id: "Client ID",
  client_secret: "Client Secret",
  token: "Access Token",
  auth_url: "URL xác thực (Auth URL)",
  token_url: "URL lấy Token (Token URL)",
  client_credentials: "Xác thực Client Credentials",
  hard_delete: "Xóa vĩnh viễn (Hard Delete)",
  encoding: "Mã hóa ký tự backend",
  spoof_ua: "Spoof User Agent",
  description: "Mô tả",
  user: "Tên đăng nhập (Username)",
  pass: "Mật khẩu (Password)",
  port: "Cổng (Port)",
  host: "Máy chủ (Host)",
  endpoint: "API Endpoint",
  env_auth: "Xác thực qua Biến môi trường",
  access_key_id: "Access Key ID",
  secret_access_key: "Secret Access Key",
  region: "Khu vực (Region)",
};

export class RemotesManager {
  private tbody: HTMLElement | null;
  private providers: ProviderInfo[] = [];
  private observer: IntersectionObserver;

  constructor() {
    this.tbody = document.getElementById('remotes-table-body');
    this.observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                const td = entry.target as HTMLTableCellElement;
                const remoteName = td.dataset.remoteName;
                if (remoteName && !td.dataset.loaded) {
                    td.dataset.loaded = 'true';
                    this.loadCapacityQueue(remoteName, td);
                }
            }
        });
    }, { root: null, rootMargin: '50px', threshold: 0 });
  }

  public async init() {
    this.setupEvents();
    await this.loadProviders(); // Tải trước (Pre-load) danh sách nhà cung cấp cho nhanh
    await this.renderList();
  }

  private loadCapacityQueue(remoteName: string, cell: HTMLTableCellElement) {
      const cached = capacityCache.get(remoteName);
      if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
          cell.innerHTML = cached.data.html;
          cell.title = cached.data.title;
          return;
      }

      capacityQueue.add(async () => {
          await this.fetchCapacity(remoteName, cell);
      });
  }

  private async fetchCapacity(remoteName: string, cell: HTMLTableCellElement) {
      try {
          // Đặt thời gian chờ tối đa (Timeout) là 5 giây
          const about = await withTimeout(getAbout(`${remoteName}:`), 5000);
          let hasAboutData = false;
          if (about.total !== undefined || about.used !== undefined || about.free !== undefined) {
              hasAboutData = true;
          }

          let html = '';
          let title = '';

          if (hasAboutData) {
              if (about.total && about.used !== undefined) {
                  const percent = Math.round((about.used / about.total) * 100);
                  const barWidth = Math.min(100, percent); // Thanh hiển thị tối đa 100%
                  let color = 'var(--colors-primary)';
                  if (percent > 90) color = '#ff5c5c'; // Màu đỏ nếu gần đầy hoặc vượt hạn mức (quota)
                  else if (percent > 70) color = '#f39c12'; // Màu cam nếu khá đầy
                  
                  html = `
                    <div style="width: 100%; height: 20px; background: var(--colors-surface-overlay); border-radius: 10px; overflow: hidden; position: relative; border: 1px solid var(--colors-border-muted);">
                      <div style="width: ${barWidth}%; height: 100%; background: ${color}; transition: width 0.3s ease;"></div>
                      <div style="position: absolute; top: 0; left: 0; width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: bold; color: ${barWidth > 50 ? '#fff' : 'var(--colors-text-primary)'}; text-shadow: ${barWidth > 50 ? '0 0 2px rgba(0,0,0,0.5)' : 'none'};">
                        ${formatSize(about.used)} / ${formatSize(about.total)} (${percent}%)
                      </div>
                    </div>
                  `;
                  title = `Trống: ${formatSize(about.free || 0)}`;
              } else if (about.free && about.free > 0) {
                  html = `Trống: <span style="color: var(--colors-primary); font-weight: bold;">${formatSize(about.free)}</span>`;
              } else if (about.used && about.used > 0) {
                  html = `Đã dùng: <span style="color: var(--colors-primary); font-weight: bold;">${formatSize(about.used)}</span>`;
              } else {
                  html = `<span style="color: var(--colors-text-muted); font-style: italic;">Không hỗ trợ</span>`;
              }
          } else {
              // Phương án dự phòng (Fallback): Dùng lệnh size của rclone
              try {
                  const sizeInfo = await withTimeout(getSize(`${remoteName}:`), 5000);
                  if (sizeInfo && sizeInfo.bytes !== undefined) {
                      html = `Đã dùng (Dự phòng): <span style="color: var(--colors-primary); font-weight: bold;">${formatSize(sizeInfo.bytes)}</span>`;
                      title = `Số lượng mục: ${sizeInfo.count || 0}`;
                  } else {
                      html = `<span style="color: var(--colors-text-muted); font-style: italic;">Không xác định</span>`;
                  }
              } catch (fallbackErr) {
                  html = `<span style="color: var(--colors-text-muted); font-style: italic;">${fallbackErr instanceof Error && fallbackErr.message === 'Timeout' ? 'Quá thời gian' : 'Không xác định'}</span>`;
              }
          }

          if (html) {
              cell.innerHTML = html;
              if (title) cell.title = title;
              capacityCache.set(remoteName, { data: { html, title }, timestamp: Date.now() });
          }
      } catch (err) {
          console.warn('Failed to get about for remote', remoteName, err);
          cell.innerHTML = `<span style="color: var(--colors-text-muted); font-style: italic;">${err instanceof Error && err.message === 'Timeout' ? 'Quá thời gian' : 'Lỗi'}</span>`;
      }
  }

  private setupEvents() {
    const btnRefresh = document.getElementById('btn-refresh-remotes');
    if (btnRefresh) {
      btnRefresh.addEventListener('click', () => this.renderList());
    }

    const btnSort = document.getElementById('btn-sort-remotes');
    if (btnSort) {
      btnSort.addEventListener('click', async () => {
        try {
          const remotes = await listRemotes();
          let names = remotes.filter(r => !(r.name === 'Local' && r.type === 'local')).map(r => r.name);
          names.sort((a, b) => a.localeCompare(b, undefined, { sensitivity: 'base' }));
          
          await reorderConfig(names);
          
          this.renderList();
        } catch (e) {
          alert('Lỗi sắp xếp: ' + e);
        }
      });
    }

    const btnAdd = document.getElementById('btn-add-remote');
    if (btnAdd) {
      btnAdd.addEventListener('click', () => this.showAddModal());
    }

    const btnExport = document.getElementById('btn-export-config');
    if (btnExport) {
      btnExport.addEventListener('click', async () => {
        try {
          const content = await getConfigContent();
          const blob = new Blob([content], { type: 'text/plain' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = 'rclone.conf';
          a.click();
          URL.revokeObjectURL(url);
        } catch (e) {
          alert('Lỗi xuất config: ' + e);
        }
      });
    }

    const btnImport = document.getElementById('btn-import-config');
    const inputImport = document.getElementById('input-import-config') as HTMLInputElement;
    if (btnImport && inputImport) {
      btnImport.addEventListener('click', () => {
        inputImport.click();
      });

      inputImport.addEventListener('change', async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (!file) return;

        try {
          const content = await file.text();
          await setConfigContent(content);
          alert('Nhập cấu hình thành công!');
          this.renderList();
        } catch (err) {
          alert('Lỗi nhập config: ' + err);
        } finally {
          inputImport.value = ''; // Reset
        }
      });
    }
  }

  private async loadProviders() {
    try {
      this.providers = await getProviders();
    } catch (e) {
      console.error("Lỗi khi load providers:", e);
    }
  }

  public async renderList() {
    if (!this.tbody) return;
    this.tbody.innerHTML = '<tr><td colspan="4" style="text-align: center;">Đang tải...</td></tr>';
    
    const remotes = await listRemotes();
    
    if (remotes.length === 0) {
      this.tbody.innerHTML = '<tr><td colspan="4" style="text-align: center;">Chưa có Remote nào.</td></tr>';
      return;
    }
    
    this.tbody.innerHTML = '';
    remotes.forEach((remote: RemoteConfig) => {
      // Bỏ qua ổ Local ảo
      if (remote.name === 'Local' && remote.type === 'local') return;
      
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td>${escapeHtml(remote.name)}</td>
        <td>${escapeHtml(remote.type)}</td>
        <td class="capacity-cell" style="color: var(--colors-text-muted);">Đang tải...</td>
        <td class="actions">
          <button class="btn btn-secondary btn-move-up" title="Đẩy Lên">↑</button>
          <button class="btn btn-secondary btn-move-down" title="Đẩy Xuống">↓</button>
          <button class="btn btn-secondary btn-feature-remote">ℹ️ Tính năng</button>
          <button class="btn btn-primary btn-edit-remote">✏️ Sửa</button>
          <button class="btn btn-danger btn-delete-remote">🗑️ Xóa</button>
        </td>
      `;
      
      const capacityCell = tr.querySelector('.capacity-cell') as HTMLTableCellElement;
      capacityCell.dataset.remoteName = remote.name;
      this.observer.observe(capacityCell);
      
      const btnUp = tr.querySelector('.btn-move-up') as HTMLButtonElement;
      btnUp.addEventListener('click', async () => {
        const names = remotes.filter((r: RemoteConfig) => !(r.name === 'Local' && r.type === 'local')).map((r: RemoteConfig) => r.name);
        const idx = names.indexOf(remote.name);
        if (idx > 0) {
          [names[idx - 1], names[idx]] = [names[idx], names[idx - 1]];
          await reorderConfig(names);
          this.renderList();
        }
      });

      const btnDown = tr.querySelector('.btn-move-down') as HTMLButtonElement;
      btnDown.addEventListener('click', async () => {
        const names = remotes.filter((r: RemoteConfig) => !(r.name === 'Local' && r.type === 'local')).map((r: RemoteConfig) => r.name);
        const idx = names.indexOf(remote.name);
        if (idx >= 0 && idx < names.length - 1) {
          [names[idx], names[idx + 1]] = [names[idx + 1], names[idx]];
          await reorderConfig(names);
          this.renderList();
        }
      });

      const btnEdit = tr.querySelector('.btn-edit-remote') as HTMLButtonElement;
      btnEdit.addEventListener('click', () => this.showEditModal(remote));

      const btnDelete = tr.querySelector('.btn-delete-remote') as HTMLButtonElement;
      btnDelete.addEventListener('click', async () => {
        if (confirm(`Bạn có chắc muốn xóa remote "${remote.name}"?`)) {
          const ok = await deleteRemote(remote.name);
          if (ok) this.renderList();
        }
      });

      const btnFeature = tr.querySelector('.btn-feature-remote') as HTMLButtonElement;
      btnFeature.addEventListener('click', () => this.showFeaturesModal(remote));

      this.tbody!.appendChild(tr);
    });
  }

  private renderDynamicFormTabs(container: HTMLElement, provider: ProviderInfo, existingValues: Record<string, any>) {
    if (!provider || !provider.Options) {
      container.innerHTML = '<div>Không tải được cấu hình.</div>';
      return;
    }

    // Tạo HTML cho Tabs
    let html = `
      <div style="display: flex; gap: 5px; margin-bottom: 15px; border-bottom: 2px solid var(--colors-border-muted);">
        <button type="button" class="btn tab-btn active" data-tab="basic" style="border-radius: 5px 5px 0 0; border: none; background: var(--colors-surface-overlay); color: var(--colors-primary); font-weight: bold; border-bottom: 2px solid var(--colors-primary); padding: 8px 15px;">[1] CƠ BẢN (BASIC)</button>
        <button type="button" class="btn tab-btn" data-tab="advanced" style="border-radius: 5px 5px 0 0; border: none; background: transparent; color: var(--colors-text-secondary); font-weight: normal; border-bottom: 2px solid transparent; padding: 8px 15px;">[2] NÂNG CAO (ADVANCED)</button>
      </div>
      
      <div id="tab-content-basic" class="tab-content" style="display: block;"></div>
      <div id="tab-content-advanced" class="tab-content" style="display: none;"></div>
    `;
    container.innerHTML = html;

    const tabBasic = container.querySelector('#tab-content-basic') as HTMLDivElement;
    const tabAdvanced = container.querySelector('#tab-content-advanced') as HTMLDivElement;
    const tabBtns = container.querySelectorAll('.tab-btn');

    // Chuyển Tab logic
    tabBtns.forEach(btn => {
      btn.addEventListener('click', (e) => {
        const target = e.currentTarget as HTMLButtonElement;
        const tabName = target.getAttribute('data-tab');
        
        // Reset styles
        tabBtns.forEach(b => {
          const bt = b as HTMLButtonElement;
          bt.style.color = 'var(--colors-text-secondary)';
          bt.style.fontWeight = 'normal';
          bt.style.borderBottomColor = 'transparent';
          bt.style.background = 'transparent';
        });
        tabBasic.style.display = 'none';
        tabAdvanced.style.display = 'none';

        // Active style
        target.style.color = 'var(--colors-primary)';
        target.style.fontWeight = 'bold';
        target.style.borderBottomColor = 'var(--colors-primary)';
        target.style.background = 'var(--colors-surface-overlay)';
        
        if (tabName === 'basic') tabBasic.style.display = 'block';
        if (tabName === 'advanced') tabAdvanced.style.display = 'block';
      });
    });

    let basicHtml = '';
    let advancedHtml = '';

    provider.Options.forEach((opt: ProviderOption) => {
      if (opt.Name === 'edit_advanced') return;

      const isRequired = opt.Required ? '<span style="color: #ff5c5c; font-size: 0.85em; font-weight: normal;">(Bắt buộc)</span>' : '<span style="color: var(--colors-text-secondary); font-size: 0.85em; font-weight: normal;">(Tùy chọn)</span>';
      
      const existingValue = existingValues[opt.Name] !== undefined ? existingValues[opt.Name] : (opt.DefaultStr || '');
      const helpText = (opt.Help || '').replace(/\n/g, '<br/>');
      const displayName = OPTION_TRANSLATIONS[opt.Name] ? `${opt.Name} (${OPTION_TRANSLATIONS[opt.Name]})` : opt.Name;

      let inputHtml = '';
      if (opt.Type === 'bool') {
        const isTrue = existingValue === 'true' || existingValue === true;
        inputHtml = `
          <select data-opt-name="${opt.Name}" class="remote-opt-input dynamic-input" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;">
            <option value="true" ${isTrue ? 'selected' : ''}>&lt; true &gt;</option>
            <option value="false" ${!isTrue ? 'selected' : ''}>&lt; false &gt;</option>
          </select>
        `;
      } else if (opt.Name === 'token') {
        let parsed: any = {};
        try {
            if (existingValue) {
                parsed = JSON.parse(existingValue);
            }
        } catch(e) {}
        
        // Luôn đảm bảo có các key cơ bản để người dùng nhập nếu chưa có
        const basicKeys = ['access_token', 'token_type', 'refresh_token', 'expiry', 'expires_in'];
        basicKeys.forEach(k => {
            if (parsed[k] === undefined) parsed[k] = '';
        });

        let subInputsHtml = '';
        for (const key of Object.keys(parsed)) {
            const val = parsed[key];
            const escapedVal = String(val).replace(/"/g, '&quot;');
            subInputsHtml += `
                <div style="margin-bottom: 8px;">
                    <label style="display: block; font-size: 0.85em; color: var(--colors-text-secondary); margin-bottom: 2px;">${key}</label>
                    <input type="text" data-token-key="${key}" class="token-sub-input" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 6px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px; font-family: monospace; font-size: 0.9em;" value="${escapedVal}" />
                </div>
            `;
        }

        inputHtml = `
            <details style="background: var(--colors-surface-input, #0e1422); border: 1px solid var(--colors-border-muted, #555); border-radius: 4px; padding: 10px;">
                <summary style="cursor: pointer; font-weight: bold; color: var(--colors-text-primary, #fff); outline: none;">
                    [+] Mở rộng xem/sửa chi tiết JSON (Không bắt buộc)
                </summary>
                <div style="margin-top: 15px; border-top: 1px dashed var(--colors-border-muted, #555); padding-top: 15px;">
                    ${subInputsHtml}
                </div>
            </details>
        `;
      } else {
        const inputType = opt.IsPassword ? 'password' : 'text';
        const escapedValue = String(existingValue).replace(/"/g, '&quot;');
        inputHtml = `<input type="${inputType}" data-opt-name="${opt.Name}" class="remote-opt-input" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;" value="${escapedValue}" placeholder="${opt.DefaultStr || ''}" />`;
      }

      const fieldHtml = `
        <div style="margin-bottom: 15px;" class="remote-opt-group">
          <label style="display: block; margin-bottom: 5px; font-weight: bold; color: #ffaa55;" title="${opt.Name}">
            &gt;&gt; ${displayName} ${isRequired}
          </label>
          ${inputHtml}
          <small style="color: var(--colors-text-secondary); display: block; margin-top: 4px; line-height: 1.4;">${helpText}</small>
        </div>
      `;

      if (opt.Advanced) {
        advancedHtml += fieldHtml;
      } else {
        basicHtml += fieldHtml;
      }
    });

    if (!basicHtml) basicHtml = '<div style="color: var(--colors-text-secondary);">Không có tham số CƠ BẢN nào.</div>';
    if (!advancedHtml) advancedHtml = '<div style="color: var(--colors-text-secondary);">Không có tham số NÂNG CAO nào.</div>';

    tabBasic.innerHTML = basicHtml;
    tabAdvanced.innerHTML = advancedHtml;
    
    // Nâng cấp tất cả các thẻ select động thành custom dropdown
    container.querySelectorAll('select.dynamic-input').forEach(sel => {
        upgradeSelectToCustomDropdown(sel as HTMLSelectElement, false);
    });
  }

  private showAddModal() {
    const modalContainer = document.getElementById('modal-container');
    if (!modalContainer) return;

    modalContainer.innerHTML = '';
    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    
    // Sắp xếp nhà cung cấp theo tên để dễ dùng hơn (UX tốt hơn)
    const sortedProviders = [...this.providers].sort((a, b) => a.Name.localeCompare(b.Name));
    const providerOptions = sortedProviders.map(p => `<option value="${p.Name}">${p.Description || p.Name} (${p.Name})</option>`).join('');

    modal.innerHTML = `
      <div class="operation-modal" style="width: 650px; max-width: 90vw; max-height: 90vh; display: flex; flex-direction: column;">
        <h3 style="margin-top: 0;">Trình Hướng Dẫn Thêm Remote (Wizard)</h3>
        
        <!-- Các bước -->
        <div id="wizard-steps-header" style="display: flex; gap: 10px; margin-bottom: 20px; font-weight: bold; font-size: 0.9em;">
          <div id="step-1-label" style="color: var(--colors-primary);">1. Chọn Loại & Tên</div>
          <div style="color: var(--colors-text-secondary);">&gt;</div>
          <div id="step-2-label" style="color: var(--colors-text-secondary);">2. Chế độ Xác thực</div>
          <div style="color: var(--colors-text-secondary);">&gt;</div>
          <div id="step-3-label" style="color: var(--colors-text-secondary);">3. Cấu hình Nâng cao</div>
        </div>

        <div id="wizard-body" style="flex: 1; overflow-y: auto; padding-right: 10px; margin-bottom: 15px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px; min-height: 350px;">
          
          <!-- STEP 1: Basic Info -->
          <div id="wizard-step-1">
            <div style="margin-bottom: 15px;">
              <label style="display: block; margin-bottom: 5px; font-weight: bold;">Tên Remote (Name) <span style="color: #ff5c5c;">*</span>:</label>
              <input type="text" id="new-remote-name" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;" placeholder="Vd: my-drive, secret-box..." />
              <small style="color: var(--colors-text-secondary); display: block; margin-top: 4px;">Tên viết liền không dấu, không khoảng trắng (vd: gdrive1).</small>
            </div>
            <div style="margin-bottom: 15px;">
              <label style="display: block; margin-bottom: 5px; font-weight: bold;">Loại Remote (Provider) <span style="color: #ff5c5c;">*</span>:</label>
              <select id="new-remote-type" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;">
                <option value="">-- Nhập để tìm hoặc chọn nhà cung cấp --</option>
                ${providerOptions}
              </select>
            </div>
          </div>

          <!-- STEP 2: Auth Mode -->
          <div id="wizard-step-2" style="display: none;">
            <div style="margin-bottom: 15px;">
              <label style="display: block; margin-bottom: 5px; font-weight: bold;">Chế độ Xác thực (Auth Mode):</label>
              <select id="new-remote-auth-mode" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;">
                <option value="auto">Tự động (Web Browser OAuth) - Mặc định</option>
                <option value="headless">Headless (Nhập Token thủ công / Máy chủ không UI)</option>
              </select>
              <small style="color: var(--colors-text-secondary); display: block; margin-top: 4px;">Web Browser: Rclone tự bật web để đăng nhập. Headless: Dành cho cấu hình từ token có sẵn.</small>
            </div>
            
            <div id="headless-token-container" style="display: none; margin-bottom: 15px;">
               <label style="display: block; margin-bottom: 5px; font-weight: bold;">Mã Token (JSON) <span style="color: #ff5c5c;">*</span>:</label>
               <textarea id="headless-token-input" style="color-scheme: dark; width: 100%; height: 100px; box-sizing: border-box; resize: vertical; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px; font-family: monospace;" placeholder='{"access_token":"...","token_type":"Bearer","refresh_token":"...","expiry":"..."}'></textarea>
               <small style="color: var(--colors-text-secondary); display: block; margin-top: 4px;">Lấy bằng lệnh: <code>rclone authorize "Tên_Provider"</code> trên máy tính cá nhân.</small>
            </div>
          </div>

          <!-- STEP 3: Advanced Options -->
          <div id="wizard-step-3" style="display: none;">
            <div id="dynamic-form-container" style="flex: 1; overflow-y: auto; padding-right: 10px; margin-bottom: 15px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
              <!-- Dynamic inputs will be rendered here -->
            </div>
            <div style="margin-top: 15px; border-top: 1px dashed var(--colors-border-muted); padding-top: 15px;">
                <label style="display: block; margin-bottom: 5px; font-weight: bold;">Custom Keys (Tuỳ chọn thêm):</label>
                <div id="custom-keys-container"></div>
                <button class="btn" id="btn-add-custom-key" style="margin-top: 8px;">+ Add New Key</button>
            </div>
          </div>
          
          <div id="wizard-loading" style="display: none; text-align: center; color: var(--colors-primary); margin-top: 20px;">
             <span class="spinner" style="display: inline-block; width: 20px; height: 20px; border: 2px solid var(--colors-primary); border-top-color: transparent; border-radius: 50%; animation: spin 1s linear infinite;"></span>
             <div style="margin-top: 10px;" id="wizard-loading-text">Đang xử lý, vui lòng chờ...</div>
          </div>

        </div>

        <div style="display: flex; justify-content: space-between; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
          <div>
            <button class="btn" id="btn-cancel-modal">Hủy</button>
          </div>
          <div style="display: flex; gap: 10px;">
            <button class="btn" id="btn-prev-step" style="display: none;">Quay lại</button>
            <button class="btn btn-primary" id="btn-next-step">Tiếp tục</button>
            <button class="btn btn-primary" id="btn-save-remote" style="display: none;">Lưu Remote</button>
          </div>
        </div>
      </div>
    `;
    modalContainer.appendChild(modal);

    let currentStep = 1;
    let customKeyCount = 0;

    // Các phần tử DOM
    const btnCancel = modal.querySelector('#btn-cancel-modal') as HTMLButtonElement;
    const btnPrev = modal.querySelector('#btn-prev-step') as HTMLButtonElement;
    const btnNext = modal.querySelector('#btn-next-step') as HTMLButtonElement;
    const btnSave = modal.querySelector('#btn-save-remote') as HTMLButtonElement;
    
    const step1 = modal.querySelector('#wizard-step-1') as HTMLDivElement;
    const step2 = modal.querySelector('#wizard-step-2') as HTMLDivElement;
    const step3 = modal.querySelector('#wizard-step-3') as HTMLDivElement;
    const loading = modal.querySelector('#wizard-loading') as HTMLDivElement;
    const loadingText = modal.querySelector('#wizard-loading-text') as HTMLDivElement;
    
    const label1 = modal.querySelector('#step-1-label') as HTMLDivElement;
    const label2 = modal.querySelector('#step-2-label') as HTMLDivElement;
    const label3 = modal.querySelector('#step-3-label') as HTMLDivElement;
    
    const nameInput = modal.querySelector('#new-remote-name') as HTMLInputElement;
    const typeSelect = modal.querySelector('#new-remote-type') as HTMLSelectElement;
    
    // Áp dụng custom dropdown có tính năng tìm kiếm cho Loại Remote
    upgradeSelectToCustomDropdown(typeSelect, true);
    
    const authModeSelect = modal.querySelector('#new-remote-auth-mode') as HTMLSelectElement;
    upgradeSelectToCustomDropdown(authModeSelect, false);

    const headlessTokenContainer = modal.querySelector('#headless-token-container') as HTMLDivElement;
    const headlessTokenInput = modal.querySelector('#headless-token-input') as HTMLTextAreaElement;
    
    const dynamicContainer = modal.querySelector('#dynamic-form-container') as HTMLDivElement;
    const customKeysContainer = modal.querySelector('#custom-keys-container') as HTMLDivElement;
    const btnAddCustomKey = modal.querySelector('#btn-add-custom-key') as HTMLButtonElement;

    // CSS cho animation spinner nếu chưa có
    if (!document.getElementById('spinner-style')) {
        const style = document.createElement('style');
        style.id = 'spinner-style';
        style.innerHTML = `@keyframes spin { 100% { transform: rotate(360deg); } }`;
        document.head.appendChild(style);
    }

    btnCancel.addEventListener('click', () => modalContainer.innerHTML = '');

    typeSelect.addEventListener('change', () => {
      const selected = typeSelect.value;
      if (selected === 'headless') {
            headlessTokenContainer.style.display = 'block';
        } else {
            headlessTokenContainer.style.display = 'none';
        }
    });

    authModeSelect.addEventListener('change', () => {
        if (authModeSelect.value === 'headless') {
            headlessTokenContainer.style.display = 'block';
        } else {
            headlessTokenContainer.style.display = 'none';
        }
    });

    btnAddCustomKey.addEventListener('click', () => {
        customKeyCount++;
        const row = document.createElement('div');
        row.style.display = 'flex';
        row.style.gap = '10px';
        row.style.marginBottom = '10px';
        row.innerHTML = `
            <input type="text" class="pane-filter custom-key-name" placeholder="Key (vd: chunk_size)" style="flex: 1;" />
            <input type="text" class="pane-filter custom-key-value" placeholder="Value" style="flex: 2;" />
            <button class="btn btn-danger btn-remove-key">X</button>
        `;
        
        const btnRemove = row.querySelector('.btn-remove-key') as HTMLButtonElement;
        btnRemove.addEventListener('click', () => row.remove());
        
        customKeysContainer.appendChild(row);
    });

    const updateWizardUI = () => {
        step1.style.display = currentStep === 1 ? 'block' : 'none';
        step2.style.display = currentStep === 2 ? 'block' : 'none';
        step3.style.display = currentStep === 3 ? 'block' : 'none';
        
        btnPrev.style.display = currentStep > 1 ? 'block' : 'none';
        btnNext.style.display = currentStep < 3 ? 'block' : 'none';
        btnSave.style.display = currentStep === 3 ? 'block' : 'none';

        label1.style.color = currentStep >= 1 ? 'var(--colors-primary)' : 'var(--colors-text-secondary)';
        label2.style.color = currentStep >= 2 ? 'var(--colors-primary)' : 'var(--colors-text-secondary)';
        label3.style.color = currentStep >= 3 ? 'var(--colors-primary)' : 'var(--colors-text-secondary)';
    };

    btnPrev.addEventListener('click', () => {
        if (currentStep > 1) {
            currentStep--;
            updateWizardUI();
        }
    });

    btnNext.addEventListener('click', () => {
        if (currentStep === 1) {
            if (!nameInput.value.trim() || !typeSelect.value) {
                alert('Vui lòng nhập Tên và chọn Loại Remote!');
                return;
            }
        }
        if (currentStep === 2) {
            if (authModeSelect.value === 'headless' && !headlessTokenInput.value.trim()) {
                alert('Vui lòng nhập Token JSON cho chế độ Headless!');
                return;
            }
            renderDynamicForm(); // Chuẩn bị cho bước 3
        }
        
        if (currentStep < 3) {
            currentStep++;
            updateWizardUI();
        }
    });

    const renderDynamicForm = () => {
      const selectedType = typeSelect.value;
      if (!selectedType) return;

      const provider = this.providers.find(p => p.Name === selectedType);
      if (!provider) {
        dynamicContainer.innerHTML = '<div>Không có tham số cấu hình bổ sung.</div>';
        return;
      }
      this.renderDynamicFormTabs(dynamicContainer, provider, {});
    };
    
    // Xử lý lưu dữ liệu (Save functionality)
    btnSave.addEventListener('click', async () => {
      const name = nameInput.value.trim();
      const provider = typeSelect.value;
      
      if (!name || !provider) return;

      const options: Record<string, string> = {};
      
      // Lấy từ Dynamic Form
      const inputs = dynamicContainer.querySelectorAll('.remote-opt-input');
      inputs.forEach(el => {
          const input = el as HTMLInputElement;
          const optName = input.getAttribute('data-opt-name');
          const val = input.value.trim();
          if (optName && val) {
              options[optName] = val;
          }
      });
      
      // Lấy riêng cho Token
      const tokenInputs = dynamicContainer.querySelectorAll('.token-sub-input') as NodeListOf<HTMLInputElement>;
      if (tokenInputs.length > 0) {
          const tokenObj: Record<string, any> = {};
          let hasTokenData = false;
          tokenInputs.forEach(input => {
              const key = input.getAttribute('data-token-key');
              let val: any = input.value.trim();
              if (key && val !== '') {
                  if (key === 'expires_in' && !isNaN(Number(val))) {
                      val = Number(val);
                  }
                  tokenObj[key] = val;
                  hasTokenData = true;
              }
          });
          if (hasTokenData) {
              options['token'] = JSON.stringify(tokenObj);
          }
      }
      
      // Lấy từ Custom Keys
      const customRows = customKeysContainer.querySelectorAll('div');
      customRows.forEach(row => {
          const keyInput = row.querySelector('.custom-key-name') as HTMLInputElement;
          const valInput = row.querySelector('.custom-key-value') as HTMLInputElement;
          if (keyInput && valInput && keyInput.value.trim()) {
              options[keyInput.value.trim()] = valInput.value.trim();
          }
      });
      
      // Xử lý Headless
      if (authModeSelect.value === 'headless') {
          options['token'] = headlessTokenInput.value.trim();
          options['config_is_local'] = 'false';
      } else {
          // Trình duyệt tự động
          options['config_is_local'] = 'true';
      }

      // Ẩn nội dung, hiện loading spinner
      step3.style.display = 'none';
      btnPrev.style.display = 'none';
      btnSave.style.display = 'none';
      btnCancel.style.display = 'none';
      loading.style.display = 'block';
      
      if (authModeSelect.value === 'auto') {
          loadingText.innerHTML = "Đang chạy Xác thực (OAuth)...<br/><br/>Hãy kiểm tra Trình duyệt Web của bạn, đăng nhập và cấp quyền cho Rclone.<br/>Tiến trình sẽ tự hoàn tất sau khi bạn đăng nhập xong!";
      } else {
          loadingText.innerHTML = "Đang lưu cấu hình Remote...";
      }

      const success = await createRemote(name, provider, options);
      if (success) {
          modalContainer.innerHTML = '';
          this.renderList();
      } else {
          // Khôi phục UI nếu lỗi
          step3.style.display = 'block';
          btnPrev.style.display = 'block';
          btnSave.style.display = 'block';
          btnCancel.style.display = 'block';
          loading.style.display = 'none';
      }
    });
  }

  private showEditModal(remote: RemoteConfig) {
    const modalContainer = document.getElementById('modal-container');
    if (!modalContainer) return;

    modalContainer.innerHTML = '';
    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    
    modal.innerHTML = `
      <div class="operation-modal" style="width: 650px; max-width: 90vw; max-height: 90vh; display: flex; flex-direction: column;">
        <h3 style="margin-top: 0; margin-bottom: 20px;">Sửa Remote</h3>

        <div style="margin-bottom: 15px;">
            <label style="display: block; margin-bottom: 5px; font-weight: bold; color: var(--colors-text-primary, #fff);">Tên Remote (Remote Name) <span style="color: #ff5c5c;">*</span>:</label>
            <input type="text" id="edit-remote-name" value="${escapeHtml(remote.name)}" style="color-scheme: dark; width: 100%; box-sizing: border-box; background: var(--colors-surface-input, #0e1422); color: var(--colors-text-primary, #fff); padding: 8px; border: 1px solid var(--colors-border-muted, #555); border-radius: 4px;" />
            <small style="color: var(--colors-text-secondary); display: block; margin-top: 4px;">Đổi tên sẽ xóa cấu hình cũ và tạo cấu hình mới (Cơ chế bắt buộc của Rclone)</small>
        </div>

        <div id="dynamic-form-container" style="flex: 1; overflow-y: auto; padding-right: 10px; margin-bottom: 15px;">
          <!-- Dynamic inputs will be rendered here -->
        </div>

        <div style="display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
          <button class="btn" id="btn-cancel-modal">Hủy</button>
          <button class="btn btn-primary" id="btn-save-remote">Cập nhật</button>
        </div>
      </div>
    `;
    modalContainer.appendChild(modal);

    const btnCancel = modal.querySelector('#btn-cancel-modal') as HTMLButtonElement;
    btnCancel.addEventListener('click', () => modalContainer.innerHTML = '');

    const dynamicContainer = modal.querySelector('#dynamic-form-container') as HTMLDivElement;

    const renderDynamicForm = () => {
      const provider = this.providers.find(p => p.Name === remote.type);
      if (!provider) {
        dynamicContainer.innerHTML = '<div>Không tải được cấu hình.</div>';
        return;
      }
      this.renderDynamicFormTabs(dynamicContainer, provider, remote);
    };

    renderDynamicForm();
    
    const btnSave = modal.querySelector('#btn-save-remote') as HTMLButtonElement;
    btnSave.addEventListener('click', async () => {
      const newNameInput = modal.querySelector('#edit-remote-name') as HTMLInputElement;
      const newName = newNameInput ? newNameInput.value.trim() : remote.name;
      
      if (!newName) {
          alert("Tên Remote không được để trống!");
          return;
      }

      const options: Record<string, string> = {};
      const inputs = dynamicContainer.querySelectorAll('.remote-opt-input') as NodeListOf<HTMLInputElement | HTMLSelectElement>;
      inputs.forEach(input => {
          const optName = input.getAttribute('data-opt-name');
          if (optName && input.value.trim() !== '') {
              options[optName] = input.value.trim();
          }
      });
      
      const tokenInputs = dynamicContainer.querySelectorAll('.token-sub-input') as NodeListOf<HTMLInputElement>;
      if (tokenInputs.length > 0) {
          const tokenObj: Record<string, any> = {};
          let hasTokenData = false;
          tokenInputs.forEach(input => {
              const key = input.getAttribute('data-token-key');
              let val: any = input.value.trim();
              if (key && val !== '') {
                  if (key === 'expires_in' && !isNaN(Number(val))) {
                      val = Number(val);
                  }
                  tokenObj[key] = val;
                  hasTokenData = true;
              }
          });
          if (hasTokenData) {
              options['token'] = JSON.stringify(tokenObj);
          }
      }

      btnSave.disabled = true;
      btnSave.textContent = 'Đang cập nhật...';

      if (newName === remote.name) {
          const success = await updateRemote(remote.name, options);
          if (success) {
              modalContainer.innerHTML = '';
              this.renderList();
          } else {
              btnSave.disabled = false;
              btnSave.textContent = 'Cập nhật';
          }
      } else {
          // Rename: Create new, then delete old
          const successCreate = await createRemote(newName, remote.type, options);
          if (successCreate) {
              await deleteRemote(remote.name);
              modalContainer.innerHTML = '';
              this.renderList();
          } else {
              btnSave.disabled = false;
              btnSave.textContent = 'Cập nhật';
          }
      }
    });
  }

  private async showFeaturesModal(remote: RemoteConfig) {
    const modalContainer = document.getElementById('modal-container');
    if (!modalContainer) return;

    modalContainer.innerHTML = '';
    const modal = document.createElement('div');
    modal.className = 'modal-overlay';
    
    modal.innerHTML = `
      <div class="operation-modal" style="width: 800px; max-width: 90vw; max-height: 90vh; display: flex; flex-direction: column;">
        <h3 style="margin-top: 0; display: flex; justify-content: space-between;">
          <span>Tính năng Backend: ${remote.name}</span>
          <span id="features-loading-spinner" style="font-size: 0.8em; color: var(--colors-text-secondary);">Đang phân tích...</span>
        </h3>
        <div id="features-content" style="flex: 1; overflow-y: auto; padding-right: 10px; margin-bottom: 15px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
          <!-- Content goes here -->
        </div>
        <div style="display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
          <button class="btn btn-primary" id="btn-close-features">Đóng</button>
        </div>
      </div>
    `;
    modalContainer.appendChild(modal);

    const btnClose = modal.querySelector('#btn-close-features') as HTMLButtonElement;
    btnClose.addEventListener('click', () => modalContainer.innerHTML = '');

    const contentDiv = modal.querySelector('#features-content') as HTMLDivElement;
    const spinner = modal.querySelector('#features-loading-spinner') as HTMLSpanElement;

    try {
      if (remote.type === 'union') {
        // Handle union remote manually to compare upstreams
        let upstreamsStr = remote.upstreams || '';
        if (typeof upstreamsStr !== 'string') upstreamsStr = String(upstreamsStr);
        // Upstreams are usually separated by space, and may contain colons.
        // E.g. "Yandex1:" "Yandex2:" "Yandex3:"
        const upstreamNames = upstreamsStr.match(/"([^"]+)"|'([^']+)'|([^\\s]+)/g)?.map((s: string) => {
          let name = s.replace(/^["']|["']$/g, '');
          if (name.endsWith(':')) name = name.slice(0, -1);
          return name;
        }).filter((n: string) => n) || [];

        if (upstreamNames.length === 0) {
          contentDiv.innerHTML = '<p>Không tìm thấy remote thành viên trong Union này.</p>';
          spinner.style.display = 'none';
          return;
        }

        const results = await Promise.all(upstreamNames.map(async (u: string) => {
          const data = await getBackendFeatures(u);
          return { name: u, features: data?.Features || {} };
        }));

        spinner.style.display = 'none';

        // Collect all unique feature keys
        const allKeysSet = new Set<string>();
        results.forEach(r => Object.keys(r.features).forEach(k => allKeysSet.add(k)));
        const allKeys = Array.from(allKeysSet).sort();

        // Check for mismatch
        let hasMismatch = false;
        for (const key of allKeys) {
          const firstVal = results[0].features[key];
          if (results.some(r => r.features[key] !== firstVal)) {
            hasMismatch = true;
            break;
          }
        }

        let html = '';
        if (hasMismatch) {
          html += `
            <div style="background-color: #ff444422; color: #ff5c5c; padding: 10px; border-radius: 5px; margin-bottom: 15px; border: 1px solid #ff5c5c; font-weight: bold;">
              ⚠ CẢNH BÁO: Các cloud thành viên không đồng đẳng! (Thiếu/Khác biệt tính năng)
            </div>
          `;
        }

        html += `
          <table style="width: 100%; border-collapse: collapse; margin-bottom: 15px;">
            <thead>
              <tr style="border-bottom: 1px solid var(--colors-border-muted);">
                <th style="text-align: left; padding: 8px;">Tính năng</th>
        `;
        results.forEach(r => {
          html += `<th style="text-align: center; padding: 8px; color: var(--colors-text-secondary); font-size: 0.9em;">${r.name}</th>`;
        });
        html += `
              </tr>
            </thead>
            <tbody>
        `;

        allKeys.forEach(key => {
          const firstVal = results[0].features[key];
          const isMismatch = results.some(r => r.features[key] !== firstVal);
          
          html += `<tr style="border-bottom: 1px solid var(--colors-border-muted); ${isMismatch ? 'background-color: #ffaaaa11;' : ''}">`;
          html += `<td style="padding: 8px; ${isMismatch ? 'color: #ffaa55; font-weight: bold;' : 'color: var(--colors-text-normal);'}">${isMismatch ? '⚠ ' : ''}${key}</td>`;
          
          results.forEach(r => {
            const val = r.features[key];
            if (val === true) {
              html += `<td style="text-align: center; padding: 8px; color: #4CAF50;">[ YES ]</td>`;
            } else if (val === false) {
              html += `<td style="text-align: center; padding: 8px; color: #f44336;">[ NO ]</td>`;
            } else {
              html += `<td style="text-align: center; padding: 8px; color: #888;">[ N/A ]</td>`;
            }
          });
          html += `</tr>`;
        });

        html += `</tbody></table>`;
        contentDiv.innerHTML = html;

      } else {
        // Normal remote
        const data = await getBackendFeatures(remote.name);
        spinner.style.display = 'none';

        if (!data || !data.Features) {
          contentDiv.innerHTML = '<p style="color: #f44336;">Lỗi khi tải thông tin tính năng.</p>';
          return;
        }

        const features = data.Features;
        let html = `
          <table style="width: 100%; border-collapse: collapse;">
            <thead>
              <tr style="border-bottom: 1px solid var(--colors-border-muted);">
                <th style="text-align: left; padding: 8px;">Tính năng (Feature)</th>
                <th style="text-align: center; padding: 8px;">Trạng thái</th>
              </tr>
            </thead>
            <tbody>
        `;

        Object.keys(features).sort().forEach(key => {
          const val = features[key];
          html += `
            <tr style="border-bottom: 1px solid var(--colors-border-muted);">
              <td style="padding: 8px; color: var(--colors-text-normal);">${key}</td>
              <td style="text-align: center; padding: 8px; ${val ? 'color: #4CAF50; font-weight: bold;' : 'color: #f44336;'}">${val ? '[ HỖ TRỢ ]' : '[ KHÔNG ]'}</td>
            </tr>
          `;
        });

        html += `</tbody></table>`;
        contentDiv.innerHTML = html;
      }
    } catch (error) {
      spinner.style.display = 'none';
      contentDiv.innerHTML = `<p style="color: #f44336;">Đã xảy ra lỗi: \${error}</p>`;
    }
  }
}
