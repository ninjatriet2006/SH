/*
[INTEGRITY NOTES]
Mục đích: Điểm vào chính (entry point) cho logic giao diện rcloneGUI.
Trách nhiệm: Khởi tạo UI, load ngôn ngữ (i18n), thiết lập sự kiện các tab, sidebar.
Các module tương tác: /bridge/remote_api.ts, /langs/vi.json
*/

import { RemotesManager } from './features/remotesManager.ts';
import { MountManager } from './features/mountManager';
import { DualPaneExplorer } from './components/DualPaneExplorer.ts';
import { MenuBar } from './components/MenuBar';
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

/**
 * Thiết lập các sự kiện giao diện (UI Events).
 */
function setupEvents() {
  // Tabs Navigation
  const tabs = document.querySelectorAll('.nav-tab');
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      // 1. Cập nhật class active cho tab
      // 2. Lấy tên view cần chuyển
      const viewName = tab.getAttribute('data-view') || 'explorer';
      const targetView = document.getElementById(`view-${viewName}`);
      if (!targetView) {
        console.warn(`View chưa được triển khai: ${viewName}`);
        return;
      }
      document.querySelector('.nav-tab.active')?.classList.remove('active');
      tab.classList.add('active');
      console.log('Chuyển sang view:', viewName);
      
      // 3. Ẩn tất cả các view
      const views = document.querySelectorAll('.app-view');
      views.forEach(v => {
        v.classList.remove('active');
        (v as HTMLElement).style.display = 'none';
      });
      
      // 4. Hiển thị view được chọn
      targetView.classList.add('active');
      targetView.style.display = viewName === 'explorer' ? 'grid' : 'flex';
      
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
  // Tạo Explorer trước khi gắn các listener tương tác của ứng dụng.
  const explorerContainer = document.getElementById('view-explorer');
  if (explorerContainer) {
    explorerContainer.innerHTML = '';
    const dualPane = new DualPaneExplorer();
    explorerContainer.appendChild(dualPane.container);

    // MenuBar chỉ nhận đúng tập lệnh nó cần, không dùng biến toàn cục.
    const menubarContainer = document.getElementById('menubar-container');
    if (menubarContainer) {
      menubarContainer.appendChild(new MenuBar(dualPane.commands).getElement());
    }
  }

  await loadLanguage('vi');
  setupEvents();
  new TransferDrawer();
  console.log('rcloneGUI khởi tạo thành công!');
});
