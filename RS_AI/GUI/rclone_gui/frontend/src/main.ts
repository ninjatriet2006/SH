/*
[INTEGRITY NOTES]
Mục đích: Điểm vào chính (entry point) cho logic giao diện rcloneGUI.
Trách nhiệm: Khởi tạo UI, load ngôn ngữ (i18n), thiết lập sự kiện các tab, sidebar.
Các module tương tác: /bridge/remote_api.ts, /langs/vi.json
*/

import { RemotesManager } from './features/remotesManager.ts';
import { MountManager } from './features/mountManager';
import { DualPaneExplorer } from './components/DualPaneExplorer.ts';
import { TransferDrawer } from './components/TransferDrawer';
import { DebugView } from './components/DebugView.ts';

// Nhúng CSS thông qua Vite bundler
import '../../themes/tokens.css';
import '../../themes/style.css';

// Tạm thời import trực tiếp JSON thay vì fetch để tránh lỗi đường dẫn tĩnh ngoài root
import viLang from '../../langs/vi.json';

// State lưu trữ dữ liệu từ điển hiện tại
let currentLangData: Record<string, string> = {};
let remotesManager: RemotesManager | null = null;
let mountManager: MountManager | null = null;
let debugView: DebugView | null = null;

/**
 * Hàm đệ quy cập nhật UI text dựa trên data-lang-id.
 * Quy tắc ID Linking: quét toàn bộ DOM, tìm [data-lang-id], cập nhật textContent.
 */
function update_language_ui(root: HTMLElement = document.body) {
  const elements = root.querySelectorAll('[data-lang-id]');
  elements.forEach(el => {
    const id = el.getAttribute('data-lang-id');
    if (id && currentLangData[id]) {
      // Chỉ cập nhật textContent nếu không phải là input placeholder
      if (el.tagName === 'INPUT' && (el as HTMLInputElement).placeholder !== undefined) {
        (el as HTMLInputElement).placeholder = currentLangData[id];
      } else {
        el.textContent = currentLangData[id];
      }
    }
  });
}

/**
 * Hàm load ngôn ngữ
 */
async function loadLanguage(_lang: string) {
  try {
    currentLangData = viLang;
    update_language_ui();
  } catch (error) {
    console.error(`Lỗi khi load ngôn ngữ:`, error);
  }
}

function setupTabs() {
  document.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.app-view').forEach(v => v.classList.remove('active'));
      
      btn.classList.add('active');
      const viewId = btn.getAttribute('data-view');
      if (viewId) {
        document.getElementById(viewId)?.classList.add('active');
      }
    });
  });
}

document.addEventListener('DOMContentLoaded', () => {
  setupTabs();
  
  // Tích hợp DualPaneExplorer chuẩn từ filenGUI
  const explorerContainer = document.getElementById('view-explorer');
  if (explorerContainer) {
    explorerContainer.innerHTML = ''; // Xoá code cũ
    const dualPane = new DualPaneExplorer();
    explorerContainer.appendChild(dualPane.container);
  }
});

/**
 * Thiết lập Menu ngữ cảnh (Chuột phải)
 */
function setupContextMenu() {
  const contextMenu = document.getElementById('context-menu');
  if (!contextMenu) return;

  // Lắng nghe chuột phải toàn cục, nhưng chỉ hiện nếu click vào file-row
  document.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement;
    const fileRow = target.closest('.file-row');
    
    if (fileRow) {
      e.preventDefault();
      const fileName = fileRow.getAttribute('data-name');
      
      contextMenu.innerHTML = `
        <div class="item" id="ctx-copy">Copy ${fileName}</div>
        <div class="item" id="ctx-move">Move ${fileName}</div>
        <div class="item" id="ctx-rename">Rename</div>
        <div class="separator"></div>
        <div class="item" id="ctx-delete" style="color:var(--colors-neon-coral)">Delete</div>
      `;
      
      contextMenu.style.display = 'block';
      contextMenu.style.left = `${e.clientX}px`;
      contextMenu.style.top = `${e.clientY}px`;
      
      // Xử lý sự kiện menu
      document.getElementById('ctx-copy')?.addEventListener('click', () => {
        console.log('Copy:', fileName);
      });
      // Tương tự cho các nút khác...
    } else {
      contextMenu.style.display = 'none';
    }
  });

  // Ẩn menu khi click ra ngoài
  document.addEventListener('click', () => {
    contextMenu.style.display = 'none';
  });
}

/**
 * Thiết lập các sự kiện giao diện (UI Events).
 */
function setupEvents() {
  // Tabs Navigation
  const tabs = document.querySelectorAll('.nav-tab');
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      // 1. Cập nhật class active cho tab
      document.querySelector('.nav-tab.active')?.classList.remove('active');
      tab.classList.add('active');
      
      // 2. Lấy tên view cần chuyển
      const viewName = tab.getAttribute('data-view') || 'explorer';
      console.log('Chuyển sang view:', viewName);
      
      // 3. Ẩn tất cả các view
      const views = document.querySelectorAll('.app-view');
      views.forEach(v => {
        v.classList.remove('active');
        (v as HTMLElement).style.display = 'none';
      });
      
      // 4. Hiển thị view được chọn
      const targetView = document.getElementById(`view-${viewName}`);
      if (targetView) {
        targetView.classList.add('active');
        targetView.style.display = viewName === 'explorer' ? 'grid' : 'flex';
      }
      
      // 5. Logic riêng từng trang
      if (viewName === 'remotes') {
        if (!remotesManager) {
          remotesManager = new RemotesManager();
          remotesManager.init();
        }
        remotesManager.renderList();
      } else if (viewName === 'mounts') {
        if (!mountManager) {
          mountManager = new MountManager();
        }
      } else if (viewName === 'debug') {
        if (!debugView) {
          debugView = new DebugView();
        }
      }
    });
  });

  // Toggle Transfer Drawer
  const drawerToggle = document.getElementById('drawer-toggle');
  const drawer = document.getElementById('transfer-drawer');
  drawerToggle?.addEventListener('click', () => {
    drawer?.classList.toggle('open');
  });

  // Resize Sidebar (đơn giản hóa)
  const resizer = document.getElementById('sidebar-resizer');
  const bodyRow = document.querySelector('.body-row') as HTMLElement;
  
  let isResizing = false;
  resizer?.addEventListener('mousedown', () => {
    isResizing = true;
    document.body.style.cursor = 'col-resize';
  });
  
  document.addEventListener('mousemove', (e) => {
    if (!isResizing) return;
    const newWidth = Math.max(150, Math.min(e.clientX, 400));
    if (bodyRow) {
      bodyRow.style.gridTemplateColumns = `${newWidth}px 4px 1fr`;
    }
  });
  
  document.addEventListener('mouseup', () => {
    if (isResizing) {
      isResizing = false;
      document.body.style.cursor = 'default';
    }
  });
}



// Chạy khởi tạo khi load xong DOM
document.addEventListener('DOMContentLoaded', async () => {
  await loadLanguage('vi');
  setupEvents();
  setupContextMenu();
  new TransferDrawer();
  console.log('rcloneGUI khởi tạo thành công!');
});
