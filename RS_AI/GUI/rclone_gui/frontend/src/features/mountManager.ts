/*
[INTEGRITY NOTES]
- Mục đích: Quản lý giao diện tab Mount/Services (hiển thị, điều khiển).
- Trách nhiệm:
  - Hiển thị danh sách các dịch vụ mount hiện có.
  - Xử lý Form thêm/sửa/xoá systemd service cho tính năng mount rclone.
  - Khởi tạo các event lắng nghe cho các nút hành động trong #view-mounts.
- Tương tác: Gọi lệnh xuống `bridge/mount_api.ts` và `bridge/remote_api.ts`.
*/

import { 
    checkFuseInstalled, 
    createMountService, 
    deleteMountService, 
    listMountServices, 
    manageMountService,
    getMountServiceConfig,
    MountConfig
} from '../../../bridge/mount_api';
import { listRemotes, RemoteConfig } from '../../../bridge/remote_api';
import { escapeHtml } from './format';
import { upgradeSelectToCustomDropdown } from './customDropdown';

export class MountManager {
    private tbody: HTMLTableSectionElement | null;
    private remotes: RemoteConfig[] = [];
    
    constructor() {
        this.tbody = document.getElementById('mounts-table-body') as HTMLTableSectionElement;
        
        const btnAdd = document.getElementById('btn-add-mount');
        if (btnAdd) btnAdd.addEventListener('click', () => this.showAddModal());
        
        const btnRefresh = document.getElementById('btn-refresh-mounts');
        if (btnRefresh) btnRefresh.addEventListener('click', () => this.renderList());
        
        // Cảnh báo nếu chưa có FUSE
        checkFuseInstalled().then(hasFuse => {
            if (!hasFuse) {
                alert("CẢNH BÁO: Không tìm thấy FUSE (fusermount/fusermount3) trên hệ thống.\nTính năng Mount sẽ không hoạt động!\nVui lòng cài đặt: sudo apt install fuse3 (hoặc fuse)");
            }
        });
    }
    
    public async renderList() {
        if (!this.tbody) return;
        this.tbody.innerHTML = '<tr><td colspan="5" style="text-align: center;">Đang tải danh sách...</td></tr>';
        
        try {
            const services = await listMountServices();
            this.remotes = await listRemotes();
            
            this.tbody.innerHTML = '';
            if (services.length === 0) {
                this.tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; color: var(--colors-text-muted);">Chưa có Mount Service nào.</td></tr>';
                return;
            }
            
            services.forEach(svc => {
                const tr = document.createElement('tr');
                
                const isUserStr = svc.is_user ? '<span style="color: #64b5f6;">User</span>' : '<span style="color: #ffb74d;">System (Root)</span>';
                const statusStr = svc.status === 'running' 
                    ? '<span style="color: #4caf50; font-weight: bold;">🟢 Đang chạy</span>' 
                    : '<span style="color: #f44336;">🔴 Đã dừng</span>';
                const enabledStr = svc.enabled ? '✅ Có' : '❌ Không';
                
                tr.innerHTML = `
                    <td style="font-weight: bold;">${svc.name.startsWith('rclone-') ? svc.name.substring(7) : svc.name}</td>
                    <td>${isUserStr}</td>
                    <td>${statusStr}</td>
                    <td>${enabledStr}</td>
                    <td style="white-space: normal;">
                        <div style="display: flex; flex-wrap: wrap; gap: 5px;">
                            <button class="btn btn-action-start" data-name="${svc.name}" data-is-user="${svc.is_user}">▶️ Chạy</button>
                            <button class="btn btn-action-stop" data-name="${svc.name}" data-is-user="${svc.is_user}">⏹️ Dừng</button>
                            <button class="btn btn-action-enable" data-name="${svc.name}" data-is-user="${svc.is_user}">🚀 Bật cùng OS</button>
                            <button class="btn btn-action-disable" data-name="${svc.name}" data-is-user="${svc.is_user}">🚫 Tắt cùng OS</button>
                            <button class="btn btn-action-edit" data-name="${svc.name}" data-is-user="${svc.is_user}">⚙️ Sửa</button>
                            <button class="btn btn-danger btn-action-delete" data-name="${svc.name}" data-is-user="${svc.is_user}">🗑 Xoá</button>
                        </div>
                    </td>
                `;
                
                this.tbody!.appendChild(tr);
            });
            
            // Gán sự kiện cho các nút thao tác (Add events)
            this.tbody.querySelectorAll('.btn-action-start').forEach(btn => {
                btn.addEventListener('click', (e) => this.handleManage(e, 'start'));
            });
            this.tbody.querySelectorAll('.btn-action-stop').forEach(btn => {
                btn.addEventListener('click', (e) => this.handleManage(e, 'stop'));
            });
            this.tbody.querySelectorAll('.btn-action-enable').forEach(btn => {
                btn.addEventListener('click', (e) => this.handleManage(e, 'enable'));
            });
            this.tbody.querySelectorAll('.btn-action-disable').forEach(btn => {
                btn.addEventListener('click', (e) => this.handleManage(e, 'disable'));
            });
            this.tbody.querySelectorAll('.btn-action-delete').forEach(btn => {
                btn.addEventListener('click', async (e) => {
                    const target = e.target as HTMLButtonElement;
                    const name = target.getAttribute('data-name');
                    const isUser = target.getAttribute('data-is-user') === 'true';
                    if (!name) return;
                    
                    if (confirm(`Bạn có chắc chắn muốn Xoá service: ${name}?\nThao tác này sẽ unmount và xoá cấu hình.`)) {
                        target.disabled = true;
                        target.textContent = 'Đang xoá...';
                        const success = await deleteMountService(name, isUser);
                        if (success) this.renderList();
                    }
                });
            });

            this.tbody.querySelectorAll('.btn-action-edit').forEach(btn => {
                btn.addEventListener('click', async (e) => {
                    const target = e.target as HTMLButtonElement;
                    const name = target.getAttribute('data-name');
                    const isUser = target.getAttribute('data-is-user') === 'true';
                    if (!name) return;
                    
                    target.disabled = true;
                    target.textContent = '...';
                    const config = await getMountServiceConfig(name, isUser);
                    target.disabled = false;
                    target.textContent = '⚙️ Sửa';
                    
                    if (config) {
                        this.showAddModal(config);
                    }
                });
            });
            
        } catch (error) {
            this.tbody.innerHTML = `<tr><td colspan="5" style="text-align: center; color: red;">Lỗi tải dữ liệu: ${error}</td></tr>`;
        }
    }
    
    private async handleManage(e: Event, action: string) {
        const target = e.target as HTMLButtonElement;
        const name = target.getAttribute('data-name');
        const isUser = target.getAttribute('data-is-user') === 'true';
        if (!name) return;
        
        target.disabled = true;
        const originalText = target.textContent;
        target.textContent = '...';
        
        await manageMountService(name, isUser, action);
        
        target.textContent = originalText;
        target.disabled = false;
        
        // Đợi một chút cho service thay đổi trạng thái và cập nhật lại danh sách
        setTimeout(() => this.renderList(), 1000);
    }
    
    private showAddModal(existingConfig: MountConfig | null = null) {
        const modalContainer = document.getElementById('modal-container');
        if (!modalContainer) return;
    
        modalContainer.innerHTML = '';
        const modal = document.createElement('div');
        modal.className = 'modal-overlay';
        
        const remoteOptions = this.remotes.map(r => `<option value="${escapeHtml(r.name)}">${escapeHtml(r.name)} (${escapeHtml(r.type)})</option>`).join('');
    
        modal.innerHTML = `
          <div class="operation-modal" style="width: 600px; max-width: 90vw; max-height: 90vh; display: flex; flex-direction: column;">
            <h3 style="margin-top: 0;">${existingConfig ? 'Chỉnh sửa Mount Service' : 'Tạo Mount Service Mới'}</h3>
            
            <div style="flex: 1; overflow-y: auto; padding-right: 10px; margin-bottom: 15px;">
                <div style="margin-bottom: 15px;">
                  <label style="display: block; margin-bottom: 5px; font-weight: bold;">Tên Service <span style="color: #ff5c5c;">*</span>:</label>
                  <input type="text" id="mount-service-name" class="pane-filter" style="width: 100%; box-sizing: border-box;" placeholder="VD: gdrive-main" value="" />
                </div>
                
                <div style="margin-bottom: 15px;">
                  <label style="display: block; margin-bottom: 5px; font-weight: bold;">Cấp độ Service (Service Level):</label>
                  <select id="mount-service-level" class="pane-filter" style="width: 100%; box-sizing: border-box; background: var(--colors-surface-input); color: var(--colors-text-primary);">
                    <option value="user">User Level (Không cần quyền Root - Khuyên dùng)</option>
                    <option value="system">System Level (Cần quyền Root / pkexec)</option>
                  </select>
                </div>
        
                <div style="margin-bottom: 15px;">
                  <label style="display: block; margin-bottom: 5px; font-weight: bold;">Chọn Remote Nguồn <span style="color: #ff5c5c;">*</span>:</label>
                  <select id="mount-remote" class="pane-filter" style="width: 100%; box-sizing: border-box; background: var(--colors-surface-input); color: var(--colors-text-primary);">
                    <option value="">-- Chọn Remote --</option>
                    ${remoteOptions}
                  </select>
                </div>
                
                <div style="margin-bottom: 15px;">
                  <label style="display: block; margin-bottom: 5px; font-weight: bold;">Đường dẫn trên Remote (Remote Path):</label>
                  <input type="text" id="mount-remote-path" class="pane-filter" style="width: 100%; box-sizing: border-box;" placeholder="VD: Video Leak/Movies" />
                  <div style="font-size: 12px; color: var(--colors-text-muted); margin-top: 4px;">Để trống để mount thư mục gốc. Các thư mục con bên trong cách nhau bởi dấu /</div>
                </div>
                
                <div style="margin-bottom: 15px;">
                  <label style="display: block; margin-bottom: 5px; font-weight: bold;">Thư mục Mount (Local Path) <span style="color: #ff5c5c;">*</span>:</label>
                  <input type="text" id="mount-path" class="pane-filter" style="width: 100%; box-sizing: border-box;" placeholder="/home/user/mnt/gdrive" />
                </div>
                
                <div style="margin-bottom: 15px; padding-top: 15px; border-top: 1px dashed var(--colors-border-muted);">
                    <div style="font-weight: bold; margin-bottom: 10px; color: var(--colors-primary);">Cấu hình Nâng cao (FUSE & VFS Cache)</div>
                    
                    <label style="display: block; margin-bottom: 5px;">VFS Cache Mode:</label>
                    <select id="mount-vfs-mode" class="pane-filter" style="width: 100%; box-sizing: border-box; margin-bottom: 10px;">
                        <option value="off">off (Mặc định)</option>
                        <option value="minimal">minimal</option>
                        <option value="writes">writes</option>
                        <option value="full">full (Khuyên dùng cho GDrive/OneDrive)</option>
                    </select>
                    
                    <label style="display: block; margin-bottom: 5px;">VFS Cache Max Size:</label>
                    <input type="text" id="mount-vfs-max-size" class="pane-filter" style="width: 100%; box-sizing: border-box; margin-bottom: 10px;" placeholder="VD: 5G, 10G" />
                    
                    <label style="display: block; margin-bottom: 5px;">VFS Cache Max Age:</label>
                    <input type="text" id="mount-vfs-max-age" class="pane-filter" style="width: 100%; box-sizing: border-box; margin-bottom: 10px;" placeholder="VD: 24h" />
                    
                    <label style="display: block; margin-bottom: 5px;">Dir Cache Time:</label>
                    <input type="text" id="mount-dir-cache-time" class="pane-filter" style="width: 100%; box-sizing: border-box; margin-bottom: 10px;" placeholder="VD: 72h" />
                    
                    <label style="display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 10px;">
                        <input type="checkbox" id="mount-allow-other" />
                        <span>Cho phép user khác truy cập (--allow-other)</span>
                    </label>
                    
                    <label style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                        <input type="checkbox" id="mount-read-only" />
                        <span>Chỉ đọc (--read-only)</span>
                    </label>
                </div>
            </div>
    
            <div style="display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--colors-border-muted); padding-top: 15px;">
              <button class="btn" id="btn-cancel-modal">Hủy</button>
              <button class="btn btn-primary" id="btn-save-mount">Lưu & Tạo Service</button>
            </div>
          </div>
        `;
        modalContainer.appendChild(modal);
        
        if (existingConfig) {
            let displaySvcName = existingConfig.service_name;
            if (displaySvcName.startsWith('rclone-')) {
                displaySvcName = displaySvcName.substring(7);
            }
            (modal.querySelector('#mount-service-name') as HTMLInputElement).value = displaySvcName;
            // Cho phép đổi tên để dọn dẹp các service cũ không chuẩn
            (modal.querySelector('#mount-service-level') as HTMLSelectElement).value = existingConfig.is_user_level ? 'user' : 'system';
            (modal.querySelector('#mount-service-level') as HTMLSelectElement).disabled = true;
            
            (modal.querySelector('#mount-remote') as HTMLSelectElement).value = existingConfig.remote_name;
            (modal.querySelector('#mount-remote-path') as HTMLInputElement).value = existingConfig.remote_path;
            (modal.querySelector('#mount-path') as HTMLInputElement).value = existingConfig.mount_path;
            (modal.querySelector('#mount-vfs-mode') as HTMLSelectElement).value = existingConfig.vfs_cache_mode || 'off';
            (modal.querySelector('#mount-vfs-max-size') as HTMLInputElement).value = existingConfig.vfs_cache_max_size;
            (modal.querySelector('#mount-vfs-max-age') as HTMLInputElement).value = existingConfig.vfs_cache_max_age;
            (modal.querySelector('#mount-dir-cache-time') as HTMLInputElement).value = existingConfig.dir_cache_time;
            (modal.querySelector('#mount-allow-other') as HTMLInputElement).checked = existingConfig.allow_other;
            (modal.querySelector('#mount-read-only') as HTMLInputElement).checked = existingConfig.read_only;
            (modal.querySelector('#btn-save-mount') as HTMLButtonElement).textContent = 'Lưu Thay Đổi';
        }
    
        // Khởi tạo các select thành custom dropdown cho đẹp (Upgrade selects to custom dropdown)
        upgradeSelectToCustomDropdown(modal.querySelector('#mount-service-level') as HTMLSelectElement, false);
        upgradeSelectToCustomDropdown(modal.querySelector('#mount-remote') as HTMLSelectElement, true);
        upgradeSelectToCustomDropdown(modal.querySelector('#mount-vfs-mode') as HTMLSelectElement, false);
    
        const btnCancel = modal.querySelector('#btn-cancel-modal') as HTMLButtonElement;
        btnCancel.addEventListener('click', () => modalContainer.innerHTML = '');
        
        const btnSave = modal.querySelector('#btn-save-mount') as HTMLButtonElement;
        btnSave.addEventListener('click', async () => {
            let service_name = (modal.querySelector('#mount-service-name') as HTMLInputElement).value.trim();
            if (!service_name) {
                alert('Vui lòng nhập tên service!');
                return;
            }
            if (!service_name.startsWith('rclone-')) {
                service_name = 'rclone-' + service_name;
            }
            
            const is_user_level = (modal.querySelector('#mount-service-level') as HTMLSelectElement).value === 'user';
            const remote_name = (modal.querySelector('#mount-remote') as HTMLSelectElement).value;
            const remote_path = (modal.querySelector('#mount-remote-path') as HTMLInputElement).value.trim();
            
            const mount_path = (modal.querySelector('#mount-path') as HTMLInputElement).value.trim();
            const vfs_cache_mode = (modal.querySelector('#mount-vfs-mode') as HTMLSelectElement).value;
            const vfs_cache_max_size = (modal.querySelector('#mount-vfs-max-size') as HTMLInputElement).value.trim();
            const vfs_cache_max_age = (modal.querySelector('#mount-vfs-max-age') as HTMLInputElement).value.trim();
            const dir_cache_time = (modal.querySelector('#mount-dir-cache-time') as HTMLInputElement).value.trim();
            const allow_other = (modal.querySelector('#mount-allow-other') as HTMLInputElement).checked;
            const read_only = (modal.querySelector('#mount-read-only') as HTMLInputElement).checked;
            
            if (!remote_name || !mount_path) {
                alert('Vui lòng chọn Remote và nhập thư mục Mount!');
                return;
            }
            
            btnSave.disabled = true;
            btnSave.textContent = 'Đang xử lý...';
            const success = await createMountService({
                service_name,
                is_user_level,
                remote_name,
                remote_path,
                mount_path,
                description: `Rclone mount for ${remote_name}`,
                vfs_cache_mode: vfs_cache_mode === 'off' ? '' : vfs_cache_mode,
                vfs_cache_max_size,
                vfs_cache_max_age,
                dir_cache_time,
                buffer_size: '64M',
                allow_other,
                read_only
            });
            
            if (success) {
                // Thu dọn service cũ nếu người dùng đang edit và tên service thay đổi
                if (existingConfig && existingConfig.service_name !== service_name) {
                    try {
                        await deleteMountService(existingConfig.service_name, existingConfig.is_user_level);
                    } catch (e) {
                        console.warn("Failed to delete old service after rename:", e);
                    }
                }
                modalContainer.innerHTML = '';
                this.renderList();
            } else {
                btnSave.disabled = false;
                btnSave.textContent = existingConfig ? 'Lưu Thay Đổi' : 'Lưu & Tạo Service';
            }
        });
    }
}
